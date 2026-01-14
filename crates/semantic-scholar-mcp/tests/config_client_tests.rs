//! Configuration and client tests.
//!
//! Tests actual behavior, not constants.

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;

// =============================================================================
// Config Behavior Tests
// =============================================================================

#[test]
fn test_config_default_has_no_api_key() {
    let config = Config::default();
    assert!(!config.has_api_key());
}

#[test]
fn test_config_with_api_key() {
    let config = Config::new(Some("test-key".to_string()));
    assert!(config.has_api_key());
    assert_eq!(config.api_key.as_deref(), Some("test-key"));
}

#[test]
fn test_config_clone_preserves_api_key() {
    let config = Config::new(Some("secret".to_string()));
    let cloned = config.clone();
    assert_eq!(config.api_key, cloned.api_key);
}

// =============================================================================
// Client Behavior Tests
// =============================================================================

#[test]
fn test_client_creation_succeeds() {
    let config = Config::default();
    let client = SemanticScholarClient::new(config);
    assert!(client.is_ok());
}

#[test]
fn test_client_with_api_key_succeeds() {
    let config = Config::new(Some("test-key".to_string()));
    let client = SemanticScholarClient::new(config);
    assert!(client.is_ok());
}

#[test]
fn test_client_reports_api_key_status() {
    let config = Config::new(Some("key".to_string()));
    let client = SemanticScholarClient::new(config).unwrap();
    assert!(client.has_api_key());

    let config_no_key = Config::default();
    let client_no_key = SemanticScholarClient::new(config_no_key).unwrap();
    assert!(!client_no_key.has_api_key());
}

#[test]
fn test_client_debug_hides_api_key() {
    let config = Config::new(Some("super-secret-key".to_string()));
    let client = SemanticScholarClient::new(config).unwrap();
    let debug = format!("{client:?}");
    // API key should NOT appear in debug output
    assert!(!debug.contains("super-secret-key"));
    assert!(debug.contains("has_api_key"));
}

#[test]
fn test_client_is_cloneable() {
    let config = Config::default();
    let client = SemanticScholarClient::new(config).unwrap();
    let _cloned = client;
    // Should compile and work
}
