//! OAuth 2.0 endpoint handlers for MCP authentication.
//!
//! Implements:
//! - RFC 9728: OAuth Protected Resource Metadata
//! - RFC 8414: OAuth Authorization Server Metadata
//! - RFC 7591: Dynamic Client Registration
//! - RFC 7636: PKCE (S256)
//! - RFC 6749: OAuth 2.0 Authorization Code Grant

use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::{HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use serde::Deserialize;

use super::pkce;
use super::store::OAuthStore;
use crate::server::transport::HttpState;

// ─── RFC 9728: Protected Resource Metadata ───────────────────────────────────

/// `GET /.well-known/oauth-protected-resource`
///
/// Tells clients where to find the authorization server for this resource.
pub async fn handle_protected_resource(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "resource": state.base_url,
        "authorization_servers": [state.base_url],
        "bearer_methods_supported": ["header"],
        "scopes_supported": ["mcp"]
    }))
}

// ─── RFC 8414: Authorization Server Metadata ─────────────────────────────────

/// `GET /.well-known/oauth-authorization-server`
///
/// Describes the OAuth endpoints and capabilities.
pub async fn handle_auth_server_metadata(State(state): State<Arc<HttpState>>) -> impl IntoResponse {
    Json(serde_json::json!({
        "issuer": state.base_url,
        "authorization_endpoint": format!("{}/authorize", state.base_url),
        "token_endpoint": format!("{}/token", state.base_url),
        "registration_endpoint": format!("{}/register", state.base_url),
        "scopes_supported": ["mcp"],
        "response_types_supported": ["code"],
        "grant_types_supported": ["authorization_code", "refresh_token"],
        "token_endpoint_auth_methods_supported": ["none"],
        "code_challenge_methods_supported": ["S256"]
    }))
}

// ─── RFC 7591: Dynamic Client Registration ───────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub client_name: Option<String>,
    pub redirect_uris: Option<Vec<String>>,
    #[serde(default)]
    pub grant_types: Vec<String>,
    #[serde(default)]
    pub response_types: Vec<String>,
    pub token_endpoint_auth_method: Option<String>,
}

/// `POST /register`
///
/// Register a new OAuth client dynamically.
pub async fn handle_register(
    State(state): State<Arc<HttpState>>,
    Json(req): Json<RegisterRequest>,
) -> Response {
    let Some(ref oauth_store) = state.oauth_store else {
        return (StatusCode::NOT_FOUND, "OAuth not configured").into_response();
    };

    let redirect_uris = req.redirect_uris.unwrap_or_default();
    if redirect_uris.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_client_metadata",
                "error_description": "redirect_uris is required"
            })),
        )
            .into_response();
    }

    let client = oauth_store.register_client(req.client_name, redirect_uris).await;

    tracing::info!(client_id = %client.client_id, "Registered OAuth client");

    (
        StatusCode::CREATED,
        Json(serde_json::json!({
            "client_id": client.client_id,
            "client_name": client.client_name,
            "redirect_uris": client.redirect_uris,
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none"
        })),
    )
        .into_response()
}

// ─── Authorization Endpoint ──────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct AuthorizeQuery {
    pub client_id: Option<String>,
    pub redirect_uri: Option<String>,
    pub response_type: Option<String>,
    pub state: Option<String>,
    pub code_challenge: Option<String>,
    pub code_challenge_method: Option<String>,
    pub scope: Option<String>,
}

/// `GET /authorize`
///
/// Auto-approve the authorization request. This is a single-user server where
/// the auth token is already configured server-side, so there's no need for an
/// interactive login page. Any client that successfully completed dynamic
/// registration and provides valid PKCE parameters is auto-approved.
pub async fn handle_authorize_get(
    State(state): State<Arc<HttpState>>,
    Query(query): Query<AuthorizeQuery>,
) -> Response {
    let Some(ref oauth_store) = state.oauth_store else {
        return (StatusCode::NOT_FOUND, "OAuth not configured").into_response();
    };

    // Validate required parameters
    let Some(client_id) = query.client_id.as_deref() else {
        return (StatusCode::BAD_REQUEST, "Missing client_id").into_response();
    };
    let Some(redirect_uri) = query.redirect_uri.as_deref() else {
        return (StatusCode::BAD_REQUEST, "Missing redirect_uri").into_response();
    };
    let Some(code_challenge) = query.code_challenge.as_deref() else {
        return (StatusCode::BAD_REQUEST, "Missing code_challenge").into_response();
    };

    if query.response_type.as_deref() != Some("code") {
        return (StatusCode::BAD_REQUEST, "response_type must be 'code'").into_response();
    }
    if query.code_challenge_method.as_deref() != Some("S256") {
        return (StatusCode::BAD_REQUEST, "code_challenge_method must be 'S256'").into_response();
    }

    // Validate client
    let Some(client) = oauth_store.get_client(client_id).await else {
        return (StatusCode::BAD_REQUEST, "Unknown client_id").into_response();
    };

    // Validate redirect_uri matches registered URIs
    if !client.redirect_uris.iter().any(|u| u == redirect_uri) {
        return (StatusCode::BAD_REQUEST, "redirect_uri not registered for this client")
            .into_response();
    }

    let scope = query.scope.as_deref().unwrap_or("mcp");

    // Auto-approve: issue auth code immediately without interactive login
    let code = oauth_store
        .create_auth_code(
            client_id.to_owned(),
            redirect_uri.to_owned(),
            code_challenge.to_owned(),
            scope.to_owned(),
        )
        .await;

    tracing::info!(client_id = %client_id, "Auto-approved authorization");

    // Build redirect URL with code and state
    let mut redirect_url = redirect_uri.to_owned();
    redirect_url.push_str(if redirect_url.contains('?') { "&" } else { "?" });
    redirect_url.push_str(&format!("code={code}"));
    if let Some(ref oauth_state) = query.state {
        redirect_url.push_str(&format!("&state={}", url_encode(oauth_state)));
    }

    (StatusCode::FOUND, [("Location", redirect_url)]).into_response()
}

// ─── Token Endpoint ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct TokenRequest {
    pub grant_type: String,
    pub code: Option<String>,
    pub redirect_uri: Option<String>,
    pub code_verifier: Option<String>,
    pub client_id: Option<String>,
    pub refresh_token: Option<String>,
}

/// `POST /token`
///
/// Exchange authorization code for tokens, or refresh tokens.
pub async fn handle_token(
    State(state): State<Arc<HttpState>>,
    axum::Form(form): axum::Form<TokenRequest>,
) -> Response {
    let Some(ref oauth_store) = state.oauth_store else {
        return (StatusCode::NOT_FOUND, "OAuth not configured").into_response();
    };

    match form.grant_type.as_str() {
        "authorization_code" => handle_authorization_code_grant(oauth_store, &form).await,
        "refresh_token" => handle_refresh_token_grant(oauth_store, &form).await,
        _ => (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "unsupported_grant_type"
            })),
        )
            .into_response(),
    }
}

async fn handle_authorization_code_grant(store: &OAuthStore, form: &TokenRequest) -> Response {
    let Some(ref code) = form.code else {
        return token_error("invalid_request", "Missing code");
    };
    let Some(ref code_verifier) = form.code_verifier else {
        return token_error("invalid_request", "Missing code_verifier");
    };

    // Consume the auth code (one-time)
    let Some(auth_code) = store.consume_auth_code(code).await else {
        return token_error("invalid_grant", "Invalid or expired authorization code");
    };

    // Verify redirect_uri matches
    if let Some(ref redirect_uri) = form.redirect_uri {
        if *redirect_uri != auth_code.redirect_uri {
            return token_error("invalid_grant", "redirect_uri mismatch");
        }
    }

    // Verify PKCE
    if !pkce::verify_s256(code_verifier, &auth_code.code_challenge) {
        return token_error("invalid_grant", "PKCE verification failed");
    }

    // Issue tokens
    let pair = store.create_token_pair(&auth_code.client_id, &auth_code.scope).await;

    tracing::info!(client_id = %auth_code.client_id, "Issued token pair");

    token_success(&pair)
}

async fn handle_refresh_token_grant(store: &OAuthStore, form: &TokenRequest) -> Response {
    let Some(ref refresh_token) = form.refresh_token else {
        return token_error("invalid_request", "Missing refresh_token");
    };

    let Some(pair) = store.refresh_token_pair(refresh_token).await else {
        return token_error("invalid_grant", "Invalid or expired refresh token");
    };

    tracing::info!("Refreshed token pair");

    token_success(&pair)
}

/// Build a token response with required OAuth 2.0 cache headers (RFC 6749 §5.1).
fn token_success(pair: &super::store::TokenPair) -> Response {
    let mut response = Json(serde_json::json!({
        "access_token": pair.access_token,
        "token_type": "Bearer",
        "expires_in": pair.expires_in,
        "refresh_token": pair.refresh_token,
        "scope": pair.scope
    }))
    .into_response();

    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response
}

fn token_error(error: &str, description: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(serde_json::json!({
            "error": error,
            "error_description": description
        })),
    )
        .into_response()
}

/// Percent-encode a string for use in URL query parameters.
fn url_encode(s: &str) -> String {
    let mut encoded = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    encoded
}
