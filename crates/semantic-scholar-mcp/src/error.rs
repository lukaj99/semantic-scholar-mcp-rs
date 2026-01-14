//! Error types for the Semantic Scholar MCP server.
//!
//! Uses `thiserror` for structured error handling with automatic `From` implementations.

use std::time::Duration;

/// Errors from the HTTP client layer.
#[derive(thiserror::Error, Debug)]
pub enum ClientError {
    /// HTTP transport error (connection, DNS, TLS, etc.)
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// Middleware error
    #[error("Middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),

    /// Rate limited by Semantic Scholar API (429 response)
    #[error("Rate limited, retry after {retry_after:?}")]
    RateLimited {
        /// Suggested wait time before retry
        retry_after: Duration,
    },

    /// Resource not found (404 response)
    #[error("Resource not found: {resource}")]
    NotFound {
        /// Description of the missing resource
        resource: String,
    },

    /// Invalid request parameters (400 response)
    #[error("Bad request: {message}")]
    BadRequest {
        /// Error message from API
        message: String,
    },

    /// Request timeout
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    /// JSON parsing error
    #[error("Failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),

    /// Server error (5xx response)
    #[error("Server error ({status}): {message}")]
    Server {
        /// HTTP status code
        status: u16,
        /// Error message
        message: String,
    },

    /// Unexpected HTTP status
    #[error("Unexpected status {status}: {message}")]
    UnexpectedStatus {
        /// HTTP status code
        status: u16,
        /// Response body or message
        message: String,
    },
}

impl ClientError {
    /// Create a rate limited error with retry-after duration.
    #[must_use]
    pub fn rate_limited(seconds: u64) -> Self {
        Self::RateLimited { retry_after: Duration::from_secs(seconds) }
    }

    /// Create a not found error.
    #[must_use]
    pub fn not_found(resource: impl Into<String>) -> Self {
        Self::NotFound { resource: resource.into() }
    }

    /// Create a bad request error.
    #[must_use]
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest { message: message.into() }
    }

    /// Create a server error.
    #[must_use]
    pub fn server(status: u16, message: impl Into<String>) -> Self {
        Self::Server { status, message: message.into() }
    }

    /// Returns true if this error is retryable.
    #[must_use]
    pub const fn is_retryable(&self) -> bool {
        matches!(self, Self::RateLimited { .. } | Self::Timeout(_) | Self::Server { .. })
    }

    /// Get the retry-after duration if this is a rate limit error.
    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after } => Some(*retry_after),
            _ => None,
        }
    }
}

/// Errors from MCP tool execution.
#[derive(thiserror::Error, Debug)]
pub enum ToolError {
    /// Error from the API client
    #[error("API error: {0}")]
    Client(#[from] ClientError),

    /// Input validation failed
    #[error("Validation error: {message}")]
    Validation {
        /// Field that failed validation
        field: String,
        /// Validation error message
        message: String,
    },

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Internal tool logic error
    #[error("Internal error: {0}")]
    Internal(String),

    /// Resource not available
    #[error("Resource unavailable: {0}")]
    Unavailable(String),
}

impl ToolError {
    /// Create a validation error.
    #[must_use]
    pub fn validation(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Validation { field: field.into(), message: message.into() }
    }

    /// Create an internal error.
    #[must_use]
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal(message.into())
    }

    /// Create an unavailable error.
    #[must_use]
    pub fn unavailable(message: impl Into<String>) -> Self {
        Self::Unavailable(message.into())
    }

    /// Convert to a user-friendly error message for MCP response.
    #[must_use]
    pub fn to_user_message(&self) -> String {
        match self {
            Self::Client(ClientError::RateLimited { retry_after }) => {
                format!(
                    "Rate limited by Semantic Scholar API. Please wait {:?} before retrying.",
                    retry_after
                )
            }
            Self::Client(ClientError::NotFound { resource }) => {
                format!("Not found: {resource}. Please check the ID is correct.")
            }
            Self::Validation { field, message } => {
                format!("Invalid input for '{field}': {message}")
            }
            _ => self.to_string(),
        }
    }
}

/// Result type alias for client operations.
pub type ClientResult<T> = Result<T, ClientError>;

/// Result type alias for tool operations.
pub type ToolResult<T> = Result<T, ToolError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_error_retryable() {
        assert!(ClientError::rate_limited(60).is_retryable());
        assert!(ClientError::Timeout(Duration::from_secs(30)).is_retryable());
        assert!(ClientError::server(500, "Internal error").is_retryable());

        assert!(!ClientError::not_found("paper123").is_retryable());
        assert!(!ClientError::bad_request("invalid query").is_retryable());
    }

    #[test]
    fn test_client_error_retry_after() {
        let err = ClientError::rate_limited(60);
        assert_eq!(err.retry_after(), Some(Duration::from_secs(60)));

        let err = ClientError::not_found("paper");
        assert_eq!(err.retry_after(), None);
    }

    #[test]
    fn test_tool_error_user_message() {
        let err = ToolError::validation("query", "cannot be empty");
        assert!(err.to_user_message().contains("query"));
        assert!(err.to_user_message().contains("cannot be empty"));
    }
}
