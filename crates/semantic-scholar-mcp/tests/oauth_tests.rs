//! Integration tests for the OAuth 2.0 authorization flow.
//!
//! Tests the full OAuth lifecycle: discovery → registration → authorization → token exchange.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::json;
use tower::ServiceExt;

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::server::transport::create_router;
use semantic_scholar_mcp::tools::{self, ToolContext};

const AUTH_TOKEN: &str = "test-secret-token-12345";
const BASE_URL: &str = "https://example.com";

fn build_test_router() -> axum::Router {
    let config = Config::for_testing("http://unused.localhost");
    let client = SemanticScholarClient::new(config).unwrap();
    let ctx = ToolContext::new(Arc::new(client));
    let tools = tools::register_all_tools();

    create_router(tools, ctx, Some(BASE_URL.to_string()), Some(AUTH_TOKEN.to_string()))
}

// ─── Discovery ───────────────────────────────────────────────────────────────

#[tokio::test]
async fn test_protected_resource_metadata() {
    let app = build_test_router();

    let response = app
        .oneshot(Request::get("/.well-known/oauth-protected-resource").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["resource"], BASE_URL);
    assert!(json["authorization_servers"].as_array().unwrap().contains(&json!(BASE_URL)));
}

#[tokio::test]
async fn test_auth_server_metadata() {
    let app = build_test_router();

    let response = app
        .oneshot(
            Request::get("/.well-known/oauth-authorization-server").body(Body::empty()).unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_eq!(json["issuer"], BASE_URL);
    assert_eq!(json["authorization_endpoint"], format!("{BASE_URL}/authorize"));
    assert_eq!(json["token_endpoint"], format!("{BASE_URL}/token"));
    assert_eq!(json["registration_endpoint"], format!("{BASE_URL}/register"));
    assert!(json["code_challenge_methods_supported"].as_array().unwrap().contains(&json!("S256")));
}

// ─── 401 with WWW-Authenticate ──────────────────────────────────────────────

#[tokio::test]
async fn test_401_includes_www_authenticate() {
    let app = build_test_router();

    let response = app
        .oneshot(
            Request::post("/mcp")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"jsonrpc":"2.0","method":"tools/list","id":1}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    let www_auth = response.headers().get("WWW-Authenticate").unwrap().to_str().unwrap();
    assert!(www_auth.contains("oauth-protected-resource"));
}

// ─── Dynamic Client Registration ─────────────────────────────────────────────

#[tokio::test]
async fn test_register_client() {
    let app = build_test_router();

    let response = app
        .oneshot(
            Request::post("/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "client_name": "Test Client",
                        "redirect_uris": ["http://localhost:3000/callback"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert!(json["client_id"].as_str().is_some());
    assert_eq!(json["client_name"], "Test Client");
}

#[tokio::test]
async fn test_register_requires_redirect_uris() {
    let app = build_test_router();

    let response = app
        .oneshot(
            Request::post("/register")
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"client_name": "Bad Client"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// ─── Legacy token auth still works ───────────────────────────────────────────

#[tokio::test]
async fn test_legacy_token_auth_still_works() {
    let app = build_test_router();

    let response = app
        .oneshot(
            Request::post(&format!("/mcp?token={AUTH_TOKEN}"))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"jsonrpc":"2.0","method":"tools/list","id":1}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_legacy_bearer_auth_still_works() {
    let app = build_test_router();

    let response = app
        .oneshot(
            Request::post("/mcp")
                .header("Content-Type", "application/json")
                .header("Authorization", format!("Bearer {AUTH_TOKEN}"))
                .body(Body::from(json!({"jsonrpc":"2.0","method":"tools/list","id":1}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

// ─── MCP Discovery with OAuth ────────────────────────────────────────────────

#[tokio::test]
async fn test_discovery_omits_token_when_oauth_active() {
    let app = build_test_router();

    let response = app
        .oneshot(Request::get("/.well-known/mcp.json").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // OAuth is active, so endpoints should NOT contain ?token=
    let mcp_endpoint = json["endpoints"]["mcp"].as_str().unwrap();
    assert!(!mcp_endpoint.contains("token="), "OAuth discovery should not leak token in URLs");

    // Should not have auth: { type: "none" } either
    assert!(json.get("auth").is_none(), "OAuth discovery should not claim auth: none");
}

// ─── Full OAuth Flow (end-to-end via OAuthStore) ─────────────────────────────

#[tokio::test]
async fn test_full_oauth_flow_via_store() {
    // This tests the OAuth store directly to verify the complete flow,
    // since the authorize POST requires form submission which is harder with tower::oneshot.
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use semantic_scholar_mcp::server::oauth::OAuthStore;
    use sha2::{Digest, Sha256};

    let store = OAuthStore::new();

    // 1. Register client
    let client = store
        .register_client(
            Some("Claude.ai".to_string()),
            vec!["https://claude.ai/callback".to_string()],
        )
        .await;
    assert!(!client.client_id.is_empty());

    // 2. Create PKCE verifier + challenge
    let code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    // 3. Create auth code (simulates successful authorize POST)
    let auth_code = store
        .create_auth_code(
            client.client_id.clone(),
            "https://claude.ai/callback".to_string(),
            code_challenge,
            "mcp".to_string(),
        )
        .await;

    // 4. Exchange code for tokens (with PKCE verification)
    let code_info = store.consume_auth_code(&auth_code).await.unwrap();
    assert_eq!(code_info.client_id, client.client_id);

    // Verify PKCE
    assert!(semantic_scholar_mcp::server::oauth::pkce::verify_s256(
        code_verifier,
        &code_info.code_challenge
    ));

    // 5. Issue tokens
    let tokens = store.create_token_pair(&code_info.client_id, &code_info.scope).await;
    assert!(!tokens.access_token.is_empty());
    assert!(!tokens.refresh_token.is_empty());

    // 6. Validate access token
    let validated = store.validate_access_token(&tokens.access_token).await;
    assert_eq!(validated.as_deref(), Some(client.client_id.as_str()));

    // 7. Refresh
    let new_tokens = store.refresh_token_pair(&tokens.refresh_token).await.unwrap();
    assert_ne!(new_tokens.access_token, tokens.access_token);

    // Old token is invalid
    assert!(store.validate_access_token(&tokens.access_token).await.is_none());

    // New token is valid
    assert!(store.validate_access_token(&new_tokens.access_token).await.is_some());
}
