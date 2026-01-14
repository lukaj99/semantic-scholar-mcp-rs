//! Semantic Scholar API client.
//!
//! Provides async HTTP client with:
//! - Connection pooling via reqwest
//! - Retry middleware with exponential backoff
//! - Rate limiting (5 req/s normal, 1 req/s batch)
//! - Response caching with 5-minute TTL

mod middleware;

use std::time::Duration;

use moka::future::Cache;
use reqwest::Client;
use reqwest_middleware::{ClientBuilder, ClientWithMiddleware};
use reqwest_retry::{RetryTransientMiddleware, policies::ExponentialBackoff};

use crate::config::{Config, api};
use crate::error::{ClientError, ClientResult};
use crate::models::{
    Author, AuthorSearchResult, BulkSearchResult, Paper, SearchResult, SnippetSearchResult,
};

/// Semantic Scholar API client.
#[derive(Clone)]
pub struct SemanticScholarClient {
    /// HTTP client with middleware.
    client: ClientWithMiddleware,

    /// Response cache.
    cache: Cache<String, serde_json::Value>,

    /// API key (optional).
    api_key: Option<String>,

    /// Graph API base URL.
    graph_api_url: String,

    /// Recommendations API base URL.
    recommendations_api_url: String,

    /// Rate limit delay.
    rate_limit_delay: std::time::Duration,

    /// Batch rate limit delay.
    batch_rate_limit_delay: std::time::Duration,
}

impl SemanticScholarClient {
    /// Create a new client with the given configuration.
    ///
    /// # Errors
    ///
    /// Returns error if HTTP client initialization fails.
    pub fn new(config: Config) -> anyhow::Result<Self> {
        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert(
            reqwest::header::CONTENT_TYPE,
            "application/json".parse().expect("valid content-type header"),
        );

        if let Some(ref key) = config.api_key {
            headers.insert("x-api-key", key.parse()?);
        }

        let client = Client::builder()
            .default_headers(headers)
            .timeout(config.request_timeout)
            .connect_timeout(config.connect_timeout)
            .pool_max_idle_per_host(api::MAX_KEEPALIVE)
            .pool_idle_timeout(api::KEEPALIVE_EXPIRY)
            .gzip(true)
            .build()?;

        let retry_policy = ExponentialBackoff::builder()
            .retry_bounds(Duration::from_secs(1), Duration::from_secs(30))
            .build_with_max_retries(3);

        let client = ClientBuilder::new(client)
            .with(RetryTransientMiddleware::new_with_policy(retry_policy))
            .build();

        let cache = Cache::builder()
            .max_capacity(config.cache_max_size)
            .time_to_live(config.cache_ttl)
            .build();

        Ok(Self {
            client,
            cache,
            api_key: config.api_key,
            graph_api_url: config.graph_api_url,
            recommendations_api_url: config.recommendations_api_url,
            rate_limit_delay: config.rate_limit_delay,
            batch_rate_limit_delay: config.batch_rate_limit_delay,
        })
    }

    /// Check if an API key is configured.
    #[must_use]
    pub fn has_api_key(&self) -> bool {
        self.api_key.is_some()
    }

    /// Search for papers.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn search_papers(
        &self,
        query: &str,
        offset: i32,
        limit: i32,
        fields: &[&str],
    ) -> ClientResult<SearchResult> {
        let url = format!("{}/paper/search", self.graph_api_url);

        let params = vec![
            ("query".to_string(), query.to_string()),
            ("offset".to_string(), offset.to_string()),
            ("limit".to_string(), limit.to_string()),
            ("fields".to_string(), fields.join(",")),
        ];

        self.get(&url, &params).await
    }

    /// Get a single paper by ID.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_paper(&self, paper_id: &str, fields: &[&str]) -> ClientResult<Paper> {
        let url = format!("{}/paper/{}", self.graph_api_url, paper_id);
        let params = vec![("fields".to_string(), fields.join(","))];

        self.get(&url, &params).await
    }

    /// Get multiple papers by ID (batch API).
    ///
    /// Invalid IDs are filtered out (API returns null for them).
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_papers_batch(
        &self,
        paper_ids: &[String],
        fields: &[&str],
    ) -> ClientResult<Vec<Paper>> {
        let url = format!("{}/paper/batch", self.graph_api_url);
        let params = vec![("fields".to_string(), fields.join(","))];

        let body = serde_json::json!({
            "ids": paper_ids
        });

        // API returns [Paper, null, Paper] for invalid IDs - filter nulls
        let results: Vec<Option<Paper>> = self.post(&url, &params, &body).await?;
        Ok(results.into_iter().flatten().collect())
    }

    /// Search for authors.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn search_authors(
        &self,
        query: &str,
        offset: i32,
        limit: i32,
    ) -> ClientResult<AuthorSearchResult> {
        let url = format!("{}/author/search", self.graph_api_url);

        let params = vec![
            ("query".to_string(), query.to_string()),
            ("offset".to_string(), offset.to_string()),
            ("limit".to_string(), limit.to_string()),
        ];

        self.get(&url, &params).await
    }

    /// Get an author by ID.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_author(&self, author_id: &str) -> ClientResult<Author> {
        let url = format!("{}/author/{}", self.graph_api_url, author_id);
        let params: Vec<(String, String)> = vec![];

        self.get(&url, &params).await
    }

    /// Get paper citations.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_citations(
        &self,
        paper_id: &str,
        offset: i32,
        limit: i32,
        fields: &[&str],
    ) -> ClientResult<crate::models::CitationResult> {
        let url = format!("{}/paper/{}/citations", self.graph_api_url, paper_id);

        let params = vec![
            ("offset".to_string(), offset.to_string()),
            ("limit".to_string(), limit.to_string()),
            ("fields".to_string(), format!("citingPaper.{}", fields.join(",citingPaper."))),
        ];

        self.get(&url, &params).await
    }

    /// Get paper references.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_references(
        &self,
        paper_id: &str,
        offset: i32,
        limit: i32,
        fields: &[&str],
    ) -> ClientResult<crate::models::CitationResult> {
        let url = format!("{}/paper/{}/references", self.graph_api_url, paper_id);

        let params = vec![
            ("offset".to_string(), offset.to_string()),
            ("limit".to_string(), limit.to_string()),
            ("fields".to_string(), format!("citedPaper.{}", fields.join(",citedPaper."))),
        ];

        self.get(&url, &params).await
    }

    /// Get recommendations for papers.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_recommendations(
        &self,
        positive_ids: &[String],
        negative_ids: Option<&[String]>,
        limit: i32,
        fields: &[&str],
    ) -> ClientResult<Vec<Paper>> {
        let url = if positive_ids.len() == 1 {
            format!("{}/papers/forpaper/{}", self.recommendations_api_url, positive_ids[0])
        } else {
            format!("{}/papers/", self.recommendations_api_url)
        };

        let params = vec![
            ("limit".to_string(), limit.to_string()),
            ("fields".to_string(), fields.join(",")),
        ];

        #[derive(serde::Deserialize)]
        struct RecommendationResult {
            #[serde(rename = "recommendedPapers")]
            recommended_papers: Vec<Paper>,
        }

        if positive_ids.len() == 1 {
            let result: RecommendationResult = self.get(&url, &params).await?;
            Ok(result.recommended_papers)
        } else {
            let body = serde_json::json!({
                "positivePaperIds": positive_ids,
                "negativePaperIds": negative_ids.unwrap_or(&[])
            });

            let result: RecommendationResult = self.post(&url, &params, &body).await?;
            Ok(result.recommended_papers)
        }
    }

    /// Bulk search for papers with boolean query syntax.
    ///
    /// Supports: +term (AND), -term (NOT), |term (OR), "phrase", term*, term~N
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn search_papers_bulk(
        &self,
        query: &str,
        token: Option<&str>,
        fields: &[&str],
        sort: Option<&str>,
        filters: &[(String, String)],
    ) -> ClientResult<BulkSearchResult> {
        let url = format!("{}/paper/search/bulk", self.graph_api_url);

        let mut params = vec![
            ("query".to_string(), query.to_string()),
            ("fields".to_string(), fields.join(",")),
        ];

        if let Some(t) = token {
            params.push(("token".to_string(), t.to_string()));
        }

        if let Some(s) = sort {
            params.push(("sort".to_string(), s.to_string()));
        }

        // Add filter parameters
        for (k, v) in filters {
            params.push((k.clone(), v.clone()));
        }

        self.get(&url, &params).await
    }

    /// Search for text snippets matching a query.
    ///
    /// Returns highlighted text excerpts from paper titles, abstracts, and body text.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn search_snippets(
        &self,
        query: &str,
        limit: i32,
        filters: &[(String, String)],
    ) -> ClientResult<SnippetSearchResult> {
        let url = format!("{}/snippet/search", self.graph_api_url);

        let mut params = vec![
            ("query".to_string(), query.to_string()),
            ("limit".to_string(), limit.to_string()),
            // Snippet API only supports snippet-specific fields, no paper fields
            ("fields".to_string(), "snippet.text,snippet.snippetKind,snippet.section".to_string()),
        ];

        // Add filter parameters
        for (k, v) in filters {
            params.push((k.clone(), v.clone()));
        }

        self.get(&url, &params).await
    }

    /// Autocomplete paper titles.
    ///
    /// Returns suggestions for partial title queries.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn autocomplete_papers(
        &self,
        query: &str,
    ) -> ClientResult<Vec<crate::models::AutocompleteMatch>> {
        let url = format!("{}/paper/autocomplete", self.graph_api_url);
        let params = vec![("query".to_string(), query.to_string())];

        #[derive(serde::Deserialize)]
        struct AutocompleteResponse {
            #[serde(default)]
            matches: Vec<crate::models::AutocompleteMatch>,
        }

        let result: AutocompleteResponse = self.get(&url, &params).await?;
        Ok(result.matches)
    }

    /// Search for a paper by exact title match.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn search_paper_by_title(
        &self,
        title: &str,
        fields: &[&str],
    ) -> ClientResult<Option<Paper>> {
        let url = format!("{}/paper/search/match", self.graph_api_url);
        let params = vec![
            ("query".to_string(), title.to_string()),
            ("fields".to_string(), fields.join(",")),
        ];

        // API returns either paper directly or error
        let result: Option<Paper> = self.get(&url, &params).await.ok();
        Ok(result)
    }

    /// Get detailed author information for a paper.
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_paper_authors(
        &self,
        paper_id: &str,
    ) -> ClientResult<Vec<crate::models::Author>> {
        let url = format!("{}/paper/{}/authors", self.graph_api_url, paper_id);
        let params = vec![(
            "fields".to_string(),
            "authorId,name,affiliations,homepage,paperCount,citationCount,hIndex,externalIds"
                .to_string(),
        )];

        #[derive(serde::Deserialize)]
        struct AuthorsResponse {
            data: Vec<crate::models::Author>,
        }

        let result: AuthorsResponse = self.get(&url, &params).await?;
        Ok(result.data)
    }

    /// Get multiple authors by ID (batch API).
    ///
    /// # Errors
    ///
    /// Returns error on API failure.
    pub async fn get_authors_batch(
        &self,
        author_ids: &[String],
    ) -> ClientResult<Vec<crate::models::Author>> {
        let url = format!("{}/author/batch", self.graph_api_url);
        let params = vec![(
            "fields".to_string(),
            "authorId,name,affiliations,homepage,paperCount,citationCount,hIndex,externalIds"
                .to_string(),
        )];

        let body = serde_json::json!({
            "ids": author_ids
        });

        // API returns [Author, null, Author] for invalid IDs - filter nulls
        let results: Vec<Option<crate::models::Author>> = self.post(&url, &params, &body).await?;
        Ok(results.into_iter().flatten().collect())
    }

    /// Make a GET request.
    async fn get<T>(&self, url: &str, params: &[(String, String)]) -> ClientResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Check cache
        let cache_key = self.cache_key("GET", url, params);
        if let Some(cached) = self.cache.get(&cache_key).await {
            return serde_json::from_value(cached).map_err(ClientError::from);
        }

        // Rate limit
        tokio::time::sleep(self.rate_limit_delay).await;

        let response = self.client.get(url).query(params).send().await?;

        let response = self.handle_response(response).await?;
        let value: serde_json::Value = response.json().await?;

        // Cache response
        self.cache.insert(cache_key, value.clone()).await;

        serde_json::from_value(value).map_err(ClientError::from)
    }

    /// Make a POST request.
    async fn post<T>(
        &self,
        url: &str,
        params: &[(String, String)],
        body: &serde_json::Value,
    ) -> ClientResult<T>
    where
        T: serde::de::DeserializeOwned,
    {
        // Rate limit (slower for batch)
        tokio::time::sleep(self.batch_rate_limit_delay).await;

        let body_str = serde_json::to_string(body)?;

        let response = self
            .client
            .post(url)
            .query(params)
            .header("Content-Type", "application/json")
            .body(body_str)
            .send()
            .await?;

        let response = self.handle_response(response).await?;
        let value: serde_json::Value = response.json().await?;

        serde_json::from_value(value).map_err(ClientError::from)
    }

    /// Handle API response status codes.
    async fn handle_response(
        &self,
        response: reqwest::Response,
    ) -> ClientResult<reqwest::Response> {
        let status = response.status();

        if status.is_success() {
            return Ok(response);
        }

        match status.as_u16() {
            429 => {
                let retry_after = response
                    .headers()
                    .get("Retry-After")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(60);

                Err(ClientError::rate_limited(retry_after))
            }
            404 => {
                let text = response.text().await.unwrap_or_default();
                Err(ClientError::not_found(text))
            }
            400 => {
                let text = response.text().await.unwrap_or_default();
                Err(ClientError::bad_request(text))
            }
            500..=599 => {
                let text = response.text().await.unwrap_or_default();
                Err(ClientError::server(status.as_u16(), text))
            }
            _ => {
                let text = response.text().await.unwrap_or_default();
                Err(ClientError::UnexpectedStatus { status: status.as_u16(), message: text })
            }
        }
    }

    /// Generate cache key.
    fn cache_key(&self, method: &str, url: &str, params: &[(String, String)]) -> String {
        use md5::{Digest, Md5};

        let mut hasher = Md5::new();
        hasher.update(method.as_bytes());
        hasher.update(b"|");
        hasher.update(url.as_bytes());
        hasher.update(b"|");

        for (k, v) in params {
            hasher.update(k.as_bytes());
            hasher.update(b"=");
            hasher.update(v.as_bytes());
            hasher.update(b"&");
        }

        format!("{:x}", hasher.finalize())
    }
}

impl std::fmt::Debug for SemanticScholarClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SemanticScholarClient").field("has_api_key", &self.has_api_key()).finish()
    }
}
