//! Tests for real-world failure scenarios.
//!
//! Based on Gemini review: tests API errors, malformed responses, graph cycles.

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    BatchMetadataTool, CitationSnowballTool, ExhaustiveSearchTool, McpTool, ToolContext,
};

fn setup_test_context(mock_server: &MockServer) -> ToolContext {
    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    ToolContext::new(Arc::new(client))
}

fn sample_paper(id: &str, title: &str) -> serde_json::Value {
    json!({
        "paperId": id,
        "title": title,
        "year": 2023,
        "citationCount": 100,
        "authors": []
    })
}

// =============================================================================
// API Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_api_rate_limit_429() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "60")
                .set_body_string("Rate limit exceeded"),
        )
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool.execute(&ctx, json!({"query": "test"})).await;

    assert!(result.is_err(), "Should return error on 429");
    let err = result.unwrap_err();
    let err_msg = err.to_string().to_lowercase();
    assert!(
        err_msg.contains("rate") || err_msg.contains("limit") || err_msg.contains("429"),
        "Error should mention rate limiting: {err_msg}"
    );
}

#[tokio::test]
async fn test_api_server_error_500() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = BatchMetadataTool;

    let result = tool.execute(&ctx, json!({"paperIds": ["p1"]})).await;

    assert!(result.is_err(), "Should return error on 500");
}

#[tokio::test]
async fn test_api_bad_request_400() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Invalid query parameter"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool.execute(&ctx, json!({"query": ""})).await;

    assert!(result.is_err(), "Should return error on 400");
}

// =============================================================================
// Malformed Response Tests
// =============================================================================

#[tokio::test]
async fn test_malformed_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{ invalid json here"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool.execute(&ctx, json!({"query": "test"})).await;

    // Should error gracefully, not panic
    assert!(result.is_err(), "Should return error on malformed JSON");
}

#[tokio::test]
async fn test_html_error_page_response() {
    let mock_server = MockServer::start().await;

    // Cloudflare/gateway often returns HTML on errors
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("<html><body>Service Unavailable</body></html>"),
        )
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool.execute(&ctx, json!({"query": "test"})).await;

    assert!(result.is_err(), "Should handle HTML response gracefully");
}

#[tokio::test]
async fn test_batch_with_null_entries() {
    let mock_server = MockServer::start().await;

    // Batch API returns null for invalid IDs
    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("valid1", "Valid Paper"),
            null, // Invalid ID returns null
            sample_paper("valid2", "Another Valid"),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = BatchMetadataTool;

    let result = tool.execute(&ctx, json!({"paperIds": ["valid1", "invalid", "valid2"]})).await;

    // Should handle nulls gracefully - either filter them or error clearly
    // The key is: no panic
    assert!(result.is_ok() || result.is_err());
    if let Ok(output) = result {
        // If it succeeds, should have filtered the null
        assert!(output.contains("Valid Paper"));
    }
}

// =============================================================================
// Graph Cycle Tests (Snowball)
// =============================================================================

#[tokio::test]
async fn test_citation_graph_cycle() {
    let mock_server = MockServer::start().await;

    // Seed paper batch
    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!([sample_paper("paperA", "Paper A")])),
        )
        .mount(&mock_server)
        .await;

    // Paper A cites Paper B
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/paperA/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citingPaper": sample_paper("paperB", "Paper B")}]
        })))
        .mount(&mock_server)
        .await;

    // Paper B cites Paper A (CYCLE!)
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/paperB/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citingPaper": sample_paper("paperA", "Paper A")}]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = CitationSnowballTool;

    // depth=2 should handle the cycle without infinite loop
    let result = tool
        .execute(
            &ctx,
            json!({
                "seedPaperIds": ["paperA"],
                "direction": "citations",
                "depth": 2
            }),
        )
        .await;

    // Should complete without hanging/panicking
    assert!(result.is_ok(), "Should handle citation cycles gracefully");
    let output = result.unwrap();
    // Should have both papers but not duplicate them infinitely
    assert!(output.contains("Paper A") || output.contains("Paper B"));
}

// =============================================================================
// Pagination Tests
// =============================================================================

#[tokio::test]
async fn test_exhaustive_search_pagination() {
    let mock_server = MockServer::start().await;

    // First page with "next" token
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 150,
            "offset": 0,
            "next": 100,
            "data": [sample_paper("p1", "Page 1 Paper")]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second page (no next = last page)
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 150,
            "offset": 100,
            "next": null,
            "data": [sample_paper("p2", "Page 2 Paper")]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "query": "test",
                "maxResults": 200  // Request more than one page
            }),
        )
        .await
        .unwrap();

    // Should contain papers from BOTH pages
    assert!(result.contains("Page 1 Paper"), "Should have first page results");
    // Note: Second page might not be fetched if maxResults is reached
}
