//! Comprehensive error handling tests.
//!
//! Tests error types, error messages, retryability, and error propagation.

use std::time::Duration;

use semantic_scholar_mcp::error::{ClientError, ToolError};

// =============================================================================
// ClientError Construction Tests
// =============================================================================

#[test]
fn test_rate_limited_error_construction() {
    let err = ClientError::rate_limited(60);
    assert!(matches!(err, ClientError::RateLimited { .. }));
    assert_eq!(err.retry_after(), Some(Duration::from_secs(60)));
}

#[test]
fn test_rate_limited_zero_seconds() {
    let err = ClientError::rate_limited(0);
    assert_eq!(err.retry_after(), Some(Duration::from_secs(0)));
}

#[test]
fn test_rate_limited_large_value() {
    let err = ClientError::rate_limited(86400); // 24 hours
    assert_eq!(err.retry_after(), Some(Duration::from_secs(86400)));
}

#[test]
fn test_not_found_error_construction() {
    let err = ClientError::not_found("paper/123");
    assert!(matches!(err, ClientError::NotFound { .. }));
    assert!(err.to_string().contains("paper/123"));
}

#[test]
fn test_not_found_empty_resource() {
    let err = ClientError::not_found("");
    assert!(err.to_string().contains("not found"));
}

#[test]
fn test_not_found_unicode_resource() {
    let err = ClientError::not_found("论文/中文标题");
    assert!(err.to_string().contains("论文"));
}

#[test]
fn test_bad_request_error_construction() {
    let err = ClientError::bad_request("query cannot be empty");
    assert!(matches!(err, ClientError::BadRequest { .. }));
    assert!(err.to_string().contains("query cannot be empty"));
}

#[test]
fn test_bad_request_long_message() {
    let long_msg = "x".repeat(1000);
    let err = ClientError::bad_request(&long_msg);
    assert!(err.to_string().contains(&long_msg));
}

#[test]
fn test_server_error_construction() {
    let err = ClientError::server(500, "Internal Server Error");
    assert!(matches!(err, ClientError::Server { .. }));
    assert!(err.to_string().contains("500"));
}

#[test]
fn test_server_error_502() {
    let err = ClientError::server(502, "Bad Gateway");
    assert!(err.to_string().contains("502"));
    assert!(err.to_string().contains("Bad Gateway"));
}

#[test]
fn test_server_error_503() {
    let err = ClientError::server(503, "Service Unavailable");
    assert!(err.to_string().contains("503"));
}

#[test]
fn test_server_error_504() {
    let err = ClientError::server(504, "Gateway Timeout");
    assert!(err.is_retryable());
}

#[test]
fn test_timeout_error() {
    let err = ClientError::Timeout(Duration::from_secs(30));
    assert!(err.to_string().contains("30"));
    assert!(err.is_retryable());
}

#[test]
fn test_timeout_zero() {
    let err = ClientError::Timeout(Duration::from_secs(0));
    assert!(err.is_retryable());
}

#[test]
fn test_unexpected_status_construction() {
    let err = ClientError::UnexpectedStatus {
        status: 418,
        message: "I'm a teapot".to_string(),
    };
    assert!(err.to_string().contains("418"));
    assert!(err.to_string().contains("teapot"));
}

// =============================================================================
// ClientError Retryability Tests
// =============================================================================

#[test]
fn test_retryable_rate_limited() {
    assert!(ClientError::rate_limited(60).is_retryable());
}

#[test]
fn test_retryable_timeout() {
    assert!(ClientError::Timeout(Duration::from_secs(30)).is_retryable());
}

#[test]
fn test_retryable_server_500() {
    assert!(ClientError::server(500, "error").is_retryable());
}

#[test]
fn test_retryable_server_502() {
    assert!(ClientError::server(502, "error").is_retryable());
}

#[test]
fn test_retryable_server_503() {
    assert!(ClientError::server(503, "error").is_retryable());
}

#[test]
fn test_not_retryable_not_found() {
    assert!(!ClientError::not_found("resource").is_retryable());
}

#[test]
fn test_not_retryable_bad_request() {
    assert!(!ClientError::bad_request("invalid").is_retryable());
}

#[test]
fn test_not_retryable_unexpected_status() {
    let err = ClientError::UnexpectedStatus {
        status: 418,
        message: "teapot".to_string(),
    };
    assert!(!err.is_retryable());
}

// =============================================================================
// ClientError retry_after Tests
// =============================================================================

#[test]
fn test_retry_after_rate_limited() {
    let err = ClientError::rate_limited(120);
    assert_eq!(err.retry_after(), Some(Duration::from_secs(120)));
}

#[test]
fn test_retry_after_not_rate_limited() {
    assert_eq!(ClientError::not_found("x").retry_after(), None);
    assert_eq!(ClientError::bad_request("x").retry_after(), None);
    assert_eq!(ClientError::server(500, "x").retry_after(), None);
    assert_eq!(
        ClientError::Timeout(Duration::from_secs(30)).retry_after(),
        None
    );
}

// =============================================================================
// ToolError Construction Tests
// =============================================================================

#[test]
fn test_tool_validation_error() {
    let err = ToolError::validation("query", "cannot be empty");
    assert!(matches!(err, ToolError::Validation { .. }));
    // Display shows "Validation error: {message}" - field is not in Display
    assert!(err.to_string().contains("cannot be empty"));
    assert!(err.to_string().contains("Validation"));
}

#[test]
fn test_tool_validation_empty_field() {
    let err = ToolError::validation("", "some message");
    assert!(err.to_string().contains("some message"));
}

#[test]
fn test_tool_internal_error() {
    let err = ToolError::internal("division by zero");
    assert!(matches!(err, ToolError::Internal(_)));
    assert!(err.to_string().contains("division by zero"));
}

#[test]
fn test_tool_unavailable_error() {
    let err = ToolError::unavailable("API offline");
    assert!(matches!(err, ToolError::Unavailable(_)));
    assert!(err.to_string().contains("API offline"));
}

// =============================================================================
// ToolError User Message Tests
// =============================================================================

#[test]
fn test_user_message_rate_limited() {
    let client_err = ClientError::rate_limited(60);
    let tool_err = ToolError::Client(client_err);
    let msg = tool_err.to_user_message();
    assert!(msg.contains("Rate limited"));
    assert!(msg.contains("wait"));
}

#[test]
fn test_user_message_not_found() {
    let client_err = ClientError::not_found("paper/123");
    let tool_err = ToolError::Client(client_err);
    let msg = tool_err.to_user_message();
    assert!(msg.contains("Not found"));
    assert!(msg.contains("paper/123"));
}

#[test]
fn test_user_message_validation() {
    let err = ToolError::validation("year", "must be positive");
    let msg = err.to_user_message();
    assert!(msg.contains("year"));
    assert!(msg.contains("must be positive"));
}

#[test]
fn test_user_message_internal() {
    let err = ToolError::internal("something broke");
    let msg = err.to_user_message();
    assert!(msg.contains("something broke"));
}

// =============================================================================
// Error Display Tests
// =============================================================================

#[test]
fn test_client_error_display_rate_limited() {
    let err = ClientError::rate_limited(60);
    let display = format!("{}", err);
    assert!(display.contains("Rate limited"));
}

#[test]
fn test_client_error_display_not_found() {
    let err = ClientError::not_found("resource");
    let display = format!("{}", err);
    assert!(display.contains("not found"));
}

#[test]
fn test_client_error_display_bad_request() {
    let err = ClientError::bad_request("invalid");
    let display = format!("{}", err);
    assert!(display.contains("Bad request"));
}

#[test]
fn test_client_error_display_server() {
    let err = ClientError::server(500, "error");
    let display = format!("{}", err);
    assert!(display.contains("Server error"));
}

#[test]
fn test_tool_error_display_client() {
    let client_err = ClientError::not_found("paper");
    let tool_err = ToolError::Client(client_err);
    let display = format!("{}", tool_err);
    assert!(display.contains("API error"));
}

#[test]
fn test_tool_error_display_validation() {
    let err = ToolError::validation("field", "message");
    let display = format!("{}", err);
    assert!(display.contains("Validation error"));
}

// =============================================================================
// Error Debug Tests
// =============================================================================

#[test]
fn test_client_error_debug() {
    let err = ClientError::rate_limited(60);
    let debug = format!("{:?}", err);
    assert!(debug.contains("RateLimited"));
}

#[test]
fn test_tool_error_debug() {
    let err = ToolError::internal("test");
    let debug = format!("{:?}", err);
    assert!(debug.contains("Internal"));
}

// =============================================================================
// Error From Conversion Tests
// =============================================================================

#[test]
fn test_client_error_from_json_error() {
    let json_err: serde_json::Error = serde_json::from_str::<i32>("invalid").unwrap_err();
    let client_err = ClientError::from(json_err);
    assert!(matches!(client_err, ClientError::Parse(_)));
}

#[test]
fn test_tool_error_from_client_error() {
    let client_err = ClientError::not_found("paper");
    let tool_err = ToolError::from(client_err);
    assert!(matches!(tool_err, ToolError::Client(_)));
}

#[test]
fn test_tool_error_from_json_error() {
    let json_err: serde_json::Error = serde_json::from_str::<i32>("invalid").unwrap_err();
    let tool_err = ToolError::from(json_err);
    assert!(matches!(tool_err, ToolError::Serialization(_)));
}
