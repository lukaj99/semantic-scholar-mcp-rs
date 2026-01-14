//! Mock-based tool tests using wiremock.
//!
//! These tests verify actual tool behavior by mocking the Semantic Scholar API.
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    ExhaustiveSearchTool, McpTool, RecommendationsTool, ToolContext,
    BatchMetadataTool, AuthorSearchTool,
};

/// Create a test context with a mock server.
fn setup_test_context(mock_server: &MockServer) -> ToolContext {
    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    ToolContext::new(Arc::new(client))
}

/// Sample paper JSON for mocking.
fn sample_paper_json(id: &str, title: &str, year: i32, citations: i32) -> serde_json::Value {
    json!({
        "paperId": id,
        "title": title,
        "abstract": format!("Abstract for {}", title),
        "year": year,
        "citationCount": citations,
        "referenceCount": 10,
        "authors": [{"authorId": "1", "name": "Test Author"}],
        "venue": "Test Conference",
        "fieldsOfStudy": ["Computer Science"],
        "externalIds": {"DOI": format!("10.1234/{}", id)}
    })
}

/// Sample search result JSON.
fn sample_search_result(papers: Vec<serde_json::Value>, next: Option<i32>) -> serde_json::Value {
    json!({
        "total": papers.len(),
        "offset": 0,
        "next": next,
        "data": papers
    })
}

// =============================================================================
// ExhaustiveSearchTool Tests
// =============================================================================

#[tokio::test]
async fn test_exhaustive_search_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .and(query_param("query", "machine learning"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![
                sample_paper_json("paper1", "ML Paper One", 2023, 100),
                sample_paper_json("paper2", "ML Paper Two", 2024, 50),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "machine learning"}))
        .await
        .unwrap();

    assert!(result.contains("ML Paper One"));
    assert!(result.contains("ML Paper Two"));
    assert!(result.contains("2023"));
}

#[tokio::test]
async fn test_exhaustive_search_with_year_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![
                sample_paper_json("paper1", "Old Paper", 2020, 100),
                sample_paper_json("paper2", "New Paper", 2024, 50),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    // Filter to only 2024
    let result = tool
        .execute(&ctx, json!({"query": "test", "yearStart": 2024}))
        .await
        .unwrap();

    // Should only include the 2024 paper
    assert!(result.contains("New Paper"));
    assert!(!result.contains("Old Paper"));
}

#[tokio::test]
async fn test_exhaustive_search_with_citation_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![
                sample_paper_json("paper1", "Popular Paper", 2023, 500),
                sample_paper_json("paper2", "Unpopular Paper", 2023, 5),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "test", "minCitations": 100}))
        .await
        .unwrap();

    assert!(result.contains("Popular Paper"));
    assert!(!result.contains("Unpopular Paper"));
}

#[tokio::test]
async fn test_exhaustive_search_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![sample_paper_json("paper1", "Test Paper", 2023, 100)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "test", "responseFormat": "json"}))
        .await
        .unwrap();

    // Should be valid JSON
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
    assert_eq!(parsed.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn test_exhaustive_search_pagination() {
    let mock_server = MockServer::start().await;

    // First page
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![sample_paper_json("paper1", "Page 1 Paper", 2023, 100)],
            Some(100),
        )))
        .mount(&mock_server)
        .await;

    // Second page
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .and(query_param("offset", "100"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![sample_paper_json("paper2", "Page 2 Paper", 2024, 50)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "test", "maxResults": 200}))
        .await
        .unwrap();

    // Should include papers from both pages
    assert!(result.contains("Page 1 Paper"));
    assert!(result.contains("Page 2 Paper"));
}

#[tokio::test]
async fn test_exhaustive_search_max_results_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_search_result(
            vec![
                sample_paper_json("p1", "Paper 1", 2023, 10),
                sample_paper_json("p2", "Paper 2", 2023, 20),
                sample_paper_json("p3", "Paper 3", 2023, 30),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "test", "maxResults": 2, "responseFormat": "json"}))
        .await
        .unwrap();

    let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed.len(), 2);
}

// =============================================================================
// RecommendationsTool Tests
// =============================================================================

#[tokio::test]
async fn test_recommendations_single_seed() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper_json("rec1", "Recommended Paper 1", 2023, 100),
                sample_paper_json("rec2", "Recommended Paper 2", 2024, 200),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = RecommendationsTool;

    let result = tool
        .execute(&ctx, json!({"positivePaperIds": ["seed123"]}))
        .await
        .unwrap();

    assert!(result.contains("Recommended Paper 1"));
    assert!(result.contains("Recommended Paper 2"));
}

#[tokio::test]
async fn test_recommendations_multiple_seeds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper_json("rec1", "Multi-Seed Rec", 2023, 150),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = RecommendationsTool;

    let result = tool
        .execute(&ctx, json!({"positivePaperIds": ["seed1", "seed2"]}))
        .await
        .unwrap();

    assert!(result.contains("Multi-Seed Rec"));
}

#[tokio::test]
async fn test_recommendations_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper_json("rec1", "JSON Rec", 2023, 100),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = RecommendationsTool;

    let result = tool
        .execute(&ctx, json!({
            "positivePaperIds": ["seed123"],
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
}

// =============================================================================
// BatchMetadataTool Tests
// =============================================================================

#[tokio::test]
async fn test_batch_metadata_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper_json("p1", "Batch Paper 1", 2023, 100),
            sample_paper_json("p2", "Batch Paper 2", 2024, 200),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = BatchMetadataTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1", "p2"]}))
        .await
        .unwrap();

    assert!(result.contains("Batch Paper 1"));
    assert!(result.contains("Batch Paper 2"));
}

#[tokio::test]
async fn test_batch_metadata_with_nulls() {
    let mock_server = MockServer::start().await;

    // API returns null for invalid IDs
    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper_json("p1", "Valid Paper", 2023, 100),
            null,
            sample_paper_json("p3", "Another Valid", 2024, 200),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = BatchMetadataTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1", "invalid", "p3"]}))
        .await
        .unwrap();

    // Should skip null and include valid papers
    assert!(result.contains("Valid Paper"));
    assert!(result.contains("Another Valid"));
}

// =============================================================================
// AuthorSearchTool Tests
// =============================================================================

#[tokio::test]
async fn test_author_search_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/search"))
        .and(query_param("query", "John Smith"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 2,
            "data": [
                {
                    "authorId": "a1",
                    "name": "John Smith",
                    "affiliations": ["MIT"],
                    "paperCount": 50,
                    "citationCount": 1000,
                    "hIndex": 15
                },
                {
                    "authorId": "a2",
                    "name": "John J. Smith",
                    "affiliations": ["Stanford"],
                    "paperCount": 30,
                    "citationCount": 500,
                    "hIndex": 10
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "John Smith"}))
        .await
        .unwrap();

    assert!(result.contains("John Smith"));
    assert!(result.contains("MIT") || result.contains("Stanford"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_exhaustive_search_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool.execute(&ctx, json!({"query": "test"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_exhaustive_search_rate_limited() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
                .set_body_string("Rate limited"),
        )
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool.execute(&ctx, json!({"query": "test"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_exhaustive_search_invalid_input() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    // Missing required 'query' field
    let result = tool.execute(&ctx, json!({"maxResults": 100})).await;
    assert!(result.is_err());
}
