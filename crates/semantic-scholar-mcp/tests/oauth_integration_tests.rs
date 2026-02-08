//! Full end-to-end integration tests for the OAuth 2.0 flow via HTTP.
//!
//! Unlike oauth_tests.rs which tests the store directly for some parts,
//! this tests the actual HTTP endpoints using axum's Router.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::json;
use sha2::{Digest, Sha256};
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

#[tokio::test]
async fn test_full_oauth_http_flow() {
    let app = build_test_router();

    // 1. Discovery
    let response = app
        .clone()
        .oneshot(Request::get("/.well-known/oauth-protected-resource").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // 2. Register Client
    let response = app
        .clone()
        .oneshot(
            Request::post("/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "client_name": "Integration Test Client",
                        "redirect_uris": ["https://client.example.com/cb"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let client_info: serde_json::Value = serde_json::from_slice(&body).unwrap();
    let client_id = client_info["client_id"].as_str().unwrap().to_string();

    // 3. Prepare PKCE
    let code_verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    let hash = Sha256::digest(code_verifier.as_bytes());
    let code_challenge = URL_SAFE_NO_PAD.encode(hash);

    // 4. Authorize (GET auto-approves and redirects with auth code)
    let authorize_uri = format!(
        "/authorize?client_id={}&redirect_uri={}&response_type=code&state=xyz123&code_challenge={}&code_challenge_method=S256&scope=mcp",
        client_id,
        url_encode("https://client.example.com/cb"),
        code_challenge,
    );

    let response = app
        .clone()
        .oneshot(Request::get(&authorize_uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    // Should redirect to callback with code (auto-approved)
    assert_eq!(response.status(), StatusCode::FOUND);
    let location = response.headers().get("Location").unwrap().to_str().unwrap();
    assert!(location.starts_with("https://client.example.com/cb"));
    assert!(location.contains("code="));
    assert!(location.contains("state=xyz123"));

    // Extract code from Location header
    let url = url::Url::parse(location).unwrap();
    let pairs: std::collections::HashMap<_, _> = url.query_pairs().collect();
    let auth_code = pairs.get("code").unwrap().to_string();

    // 5. Exchange Code for Token
    let params = [
        ("grant_type", "authorization_code"),
        ("code", &auth_code),
        ("redirect_uri", "https://client.example.com/cb"),
        ("code_verifier", code_verifier),
        ("client_id", &client_id),
    ];
    let body_str = serde_urlencoded::to_string(params).unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::post("/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let token_info: serde_json::Value = serde_json::from_slice(&body).unwrap();

    let access_token = token_info["access_token"].as_str().unwrap();
    let refresh_token = token_info["refresh_token"].as_str().unwrap();

    // 6. Use Access Token on Protected Endpoint
    let response = app
        .clone()
        .oneshot(
            Request::post("/mcp")
                .header("Authorization", format!("Bearer {}", access_token))
                .header("Content-Type", "application/json")
                .body(Body::from(json!({"jsonrpc":"2.0","method":"tools/list","id":1}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(result.get("result").is_some());

    // 7. Refresh Token
    let params = [("grant_type", "refresh_token"), ("refresh_token", refresh_token)];
    let body_str = serde_urlencoded::to_string(params).unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::post("/token")
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(Body::from(body_str))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let new_token_info: serde_json::Value = serde_json::from_slice(&body).unwrap();

    assert_ne!(new_token_info["access_token"].as_str().unwrap(), access_token);
}

#[tokio::test]
async fn test_authorize_rejects_unregistered_client() {
    let app = build_test_router();

    let response = app
        .clone()
        .oneshot(
            Request::get(
                "/authorize?client_id=unknown&redirect_uri=https://cb.com&response_type=code&code_challenge=abc&code_challenge_method=S256",
            )
            .body(Body::empty())
            .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_authorize_rejects_wrong_redirect_uri() {
    let app = build_test_router();

    // Register client with specific redirect_uri
    let response = app
        .clone()
        .oneshot(
            Request::post("/register")
                .header("Content-Type", "application/json")
                .body(Body::from(
                    json!({
                        "client_name": "Test",
                        "redirect_uris": ["https://legit.com/cb"]
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let client_id = serde_json::from_slice::<serde_json::Value>(&body).unwrap()["client_id"]
        .as_str()
        .unwrap()
        .to_string();

    // Try to authorize with a different redirect_uri
    let uri = format!(
        "/authorize?client_id={}&redirect_uri={}&response_type=code&code_challenge=abc&code_challenge_method=S256",
        client_id,
        url_encode("https://evil.com/steal"),
    );

    let response = app
        .clone()
        .oneshot(Request::get(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
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
