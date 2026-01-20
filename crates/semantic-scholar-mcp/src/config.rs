//! Configuration for the Semantic Scholar MCP server.

use std::time::Duration;

/// API configuration constants.
pub mod api {
    use std::time::Duration;

    /// Base URL for Semantic Scholar API.
    pub const BASE_URL: &str = "https://api.semanticscholar.org";

    /// Graph API endpoint.
    pub const GRAPH_API: &str = "https://api.semanticscholar.org/graph/v1";

    /// Recommendations API endpoint.
    pub const RECOMMENDATIONS_API: &str = "https://api.semanticscholar.org/recommendations/v1";

    /// Request timeout (increased for complex operations like cocitation_analysis).
    pub const REQUEST_TIMEOUT: Duration = Duration::from_secs(90);

    /// Connection timeout.
    pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);

    /// Rate limit delay between requests without API key (200ms = 5 req/s).
    pub const RATE_LIMIT_DELAY: Duration = Duration::from_millis(200);

    /// Rate limit delay between requests with API key (10ms = 100 req/s).
    pub const RATE_LIMIT_DELAY_WITH_KEY: Duration = Duration::from_millis(10);

    /// Rate limit delay for batch requests without API key (1s = 1 req/s).
    pub const BATCH_RATE_LIMIT_DELAY: Duration = Duration::from_secs(1);

    /// Rate limit delay for batch requests with API key (100ms = 10 req/s).
    pub const BATCH_RATE_LIMIT_DELAY_WITH_KEY: Duration = Duration::from_millis(100);

    /// Cache TTL (5 minutes).
    pub const CACHE_TTL: Duration = Duration::from_secs(300);

    /// Maximum cache size.
    pub const CACHE_MAX_SIZE: u64 = 1000;

    /// Maximum connections.
    pub const MAX_CONNECTIONS: usize = 20;

    /// Maximum keepalive connections.
    pub const MAX_KEEPALIVE: usize = 10;

    /// Keepalive expiry.
    pub const KEEPALIVE_EXPIRY: Duration = Duration::from_secs(30);
}

/// Paper field sets for API requests.
pub mod fields {
    /// Minimal fields for compact responses (token-efficient).
    pub const MINIMAL: &[&str] = &["paperId", "title", "year", "citationCount", "authors"];

    /// Default fields for most use cases.
    pub const DEFAULT: &[&str] = &[
        "paperId",
        "title",
        "abstract",
        "year",
        "citationCount",
        "referenceCount",
        "fieldsOfStudy",
        "authors",
        "venue",
        "publicationDate",
        "openAccessPdf",
        "externalIds",
    ];

    /// Extended fields (use sparingly - embeddings are 768 floats!).
    pub const EXTENDED: &[&str] = &[
        "paperId",
        "title",
        "abstract",
        "year",
        "citationCount",
        "referenceCount",
        "fieldsOfStudy",
        "authors",
        "venue",
        "publicationDate",
        "openAccessPdf",
        "externalIds",
        "tldr",
        "embedding",
    ];

    /// Author fields for author queries.
    pub const AUTHOR: &[&str] =
        &["authorId", "name", "affiliations", "homepage", "paperCount", "citationCount", "hIndex"];
}

/// Server configuration.
#[derive(Debug, Clone)]
pub struct Config {
    /// Semantic Scholar API key (optional).
    pub api_key: Option<String>,

    /// Authentication token for the MCP server (optional).
    pub auth_token: Option<String>,

    /// Base URL for Graph API (for testing with mock servers).
    pub graph_api_url: String,

    /// Base URL for Recommendations API (for testing with mock servers).
    pub recommendations_api_url: String,

    /// Request timeout.
    pub request_timeout: Duration,

    /// Connection timeout.
    pub connect_timeout: Duration,

    /// Rate limit delay between requests.
    pub rate_limit_delay: Duration,

    /// Rate limit delay for batch requests.
    pub batch_rate_limit_delay: Duration,

    /// Cache TTL.
    pub cache_ttl: Duration,

    /// Maximum cache size.
    pub cache_max_size: u64,
}

impl Config {
    /// Create a new configuration with optional API key.
    ///
    /// Rate limits are automatically adjusted based on API key presence:
    /// - Without key: 5 req/s normal, 1 req/s batch
    /// - With key: 100 req/s normal, 10 req/s batch
    #[must_use]
    pub fn new(api_key: Option<String>, auth_token: Option<String>) -> Self {
        let has_key = api_key.is_some();
        Self {
            api_key,
            auth_token,
            graph_api_url: api::GRAPH_API.to_string(),
            recommendations_api_url: api::RECOMMENDATIONS_API.to_string(),
            request_timeout: api::REQUEST_TIMEOUT,
            connect_timeout: api::CONNECT_TIMEOUT,
            rate_limit_delay: if has_key {
                api::RATE_LIMIT_DELAY_WITH_KEY
            } else {
                api::RATE_LIMIT_DELAY
            },
            batch_rate_limit_delay: if has_key {
                api::BATCH_RATE_LIMIT_DELAY_WITH_KEY
            } else {
                api::BATCH_RATE_LIMIT_DELAY
            },
            cache_ttl: api::CACHE_TTL,
            cache_max_size: api::CACHE_MAX_SIZE,
        }
    }

    /// Create a test configuration with custom URLs for mock servers.
    #[must_use]
    pub fn for_testing(base_url: &str) -> Self {
        Self {
            api_key: None,
            auth_token: None,
            graph_api_url: format!("{}/graph/v1", base_url),
            recommendations_api_url: format!("{}/recommendations/v1", base_url),
            request_timeout: Duration::from_secs(5),
            connect_timeout: Duration::from_secs(2),
            rate_limit_delay: Duration::from_millis(0), // No delay in tests
            batch_rate_limit_delay: Duration::from_millis(0),
            cache_ttl: Duration::from_secs(0), // No caching in tests
            cache_max_size: 0,
        }
    }

    /// Create configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns error if environment variables are invalid.
    pub fn from_env() -> anyhow::Result<Self> {
        let api_key = std::env::var("SEMANTIC_SCHOLAR_API_KEY").ok();
        let auth_token = std::env::var("MCP_SERVER_AUTH_TOKEN").ok();
        Ok(Self::new(api_key, auth_token))
    }

    /// Check if an API key is configured.
    #[must_use]
    pub const fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new(None, None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = Config::default();
        assert!(config.api_key.is_none());
        assert!(!config.has_api_key());
    }

    #[test]
    fn test_config_with_api_key() {
        let config = Config::new(Some("test-key".to_string()), None);
        assert!(config.has_api_key());
        assert_eq!(config.api_key, Some("test-key".to_string()));
    }

    #[test]
    fn test_fields() {
        assert!(fields::MINIMAL.contains(&"paperId"));
        assert!(fields::DEFAULT.contains(&"abstract"));
        assert!(fields::EXTENDED.contains(&"embedding"));
    }
}
