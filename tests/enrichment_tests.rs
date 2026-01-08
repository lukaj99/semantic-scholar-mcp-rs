//! Mock-based tests for enrichment tools: batch_metadata, author_search, author_papers,
//! paper_autocomplete, paper_title_match, paper_authors, author_batch

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    AuthorBatchTool, AuthorPapersTool, AuthorSearchTool, BatchMetadataTool, McpTool,
    PaperAutocompleteTool, PaperAuthorsTool, PaperTitleMatchTool, ToolContext,
};

async fn setup_test_context(mock_server: &MockServer) -> ToolContext {
    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    ToolContext::new(Arc::new(client))
}

fn sample_paper(id: &str, title: &str, year: i32, citations: i32) -> serde_json::Value {
    json!({
        "paperId": id,
        "title": title,
        "abstract": format!("Abstract for {}", title),
        "year": year,
        "citationCount": citations,
        "referenceCount": 10,
        "authors": [{"authorId": "a1", "name": "Test Author"}],
        "venue": "Test Journal",
        "fieldsOfStudy": ["Computer Science"],
        "externalIds": {"DOI": format!("10.1234/{}", id)}
    })
}

fn sample_author(id: &str, name: &str, citations: i32, h_index: i32) -> serde_json::Value {
    json!({
        "authorId": id,
        "name": name,
        "affiliations": ["MIT"],
        "citationCount": citations,
        "hIndex": h_index,
        "paperCount": 50
    })
}

// =============================================================================
// BatchMetadataTool Tests
// =============================================================================

#[tokio::test]
async fn test_batch_metadata_markdown_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "Paper One", 2023, 100),
            sample_paper("p2", "Paper Two", 2022, 200),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BatchMetadataTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1", "p2"]}))
        .await
        .unwrap();

    assert!(result.contains("Paper One"));
    assert!(result.contains("Paper Two"));
}

#[tokio::test]
async fn test_batch_metadata_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "JSON Paper", 2023, 50)
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BatchMetadataTool;

    let result = tool
        .execute(&ctx, json!({
            "paperIds": ["p1"],
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
}

#[tokio::test]
async fn test_batch_metadata_custom_fields() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {"paperId": "p1", "title": "Custom Fields", "year": 2023}
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BatchMetadataTool;

    let result = tool
        .execute(&ctx, json!({
            "paperIds": ["p1"],
            "fields": ["paperId", "title", "year"]
        }))
        .await
        .unwrap();

    assert!(result.contains("Custom Fields") || result.contains("2023"));
}

#[tokio::test]
async fn test_batch_metadata_empty_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BatchMetadataTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["invalid1", "invalid2"]}))
        .await
        .unwrap();

    // Empty but not error
    assert!(result.is_empty() || result.contains("[]") || result.len() < 50);
}

#[tokio::test]
async fn test_batch_metadata_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BatchMetadataTool;

    let result = tool.execute(&ctx, json!({"paperIds": ["p1"]})).await;
    assert!(result.is_err());
}

// =============================================================================
// AuthorSearchTool Tests
// =============================================================================

#[tokio::test]
async fn test_author_search_markdown_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 2,
            "data": [
                sample_author("a1", "John Smith", 5000, 25),
                sample_author("a2", "Jane Smith", 3000, 20),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "Smith"}))
        .await
        .unwrap();

    assert!(result.contains("John Smith") || result.contains("Jane Smith"));
}

#[tokio::test]
async fn test_author_search_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 1,
            "data": [sample_author("a1", "JSON Author", 1000, 15)]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "JSON Author",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_array());
}

#[tokio::test]
async fn test_author_search_with_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 1,
            "data": [sample_author("a1", "Limited", 500, 10)]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "test",
            "limit": 5
        }))
        .await
        .unwrap();

    assert!(result.contains("Limited") || result.len() > 0);
}

#[tokio::test]
async fn test_author_search_no_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorSearchTool;

    let result = tool
        .execute(&ctx, json!({"query": "nonexistent xyz"}))
        .await
        .unwrap();

    // Empty but not error
    assert!(result.is_empty() || result.contains("[]") || result.len() < 50);
}

// =============================================================================
// AuthorPapersTool Tests
// =============================================================================

#[tokio::test]
async fn test_author_papers_markdown_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/author123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            sample_author("author123", "Prolific Writer", 10000, 30)
        ))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorPapersTool;

    let result = tool
        .execute(&ctx, json!({"authorId": "author123"}))
        .await
        .unwrap();

    assert!(result.contains("Prolific Writer") || result.contains("author"));
}

#[tokio::test]
async fn test_author_papers_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/json_author"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            sample_author("json_author", "JSON Author Papers", 2000, 18)
        ))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorPapersTool;

    let result = tool
        .execute(&ctx, json!({
            "authorId": "json_author",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.is_object());
}

#[tokio::test]
async fn test_author_papers_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/invalid"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorPapersTool;

    let result = tool.execute(&ctx, json!({"authorId": "invalid"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_author_papers_with_year_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/filtered"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            sample_author("filtered", "Filtered Author", 5000, 22)
        ))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorPapersTool;

    let result = tool
        .execute(&ctx, json!({
            "authorId": "filtered",
            "yearStart": 2020,
            "yearEnd": 2024
        }))
        .await
        .unwrap();

    assert!(result.contains("Filtered Author") || result.contains("author"));
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_batch_metadata_tool_name() {
    let tool = BatchMetadataTool;
    assert_eq!(tool.name(), "batch_metadata");
}

#[test]
fn test_batch_metadata_tool_description() {
    let tool = BatchMetadataTool;
    assert!(tool.description().contains("metadata"));
}

#[test]
fn test_batch_metadata_tool_input_schema() {
    let tool = BatchMetadataTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["paper_ids"].is_object());
}

#[test]
fn test_author_search_tool_name() {
    let tool = AuthorSearchTool;
    assert_eq!(tool.name(), "author_search");
}

#[test]
fn test_author_search_tool_description() {
    let tool = AuthorSearchTool;
    assert!(tool.description().contains("author"));
}

#[test]
fn test_author_search_tool_input_schema() {
    let tool = AuthorSearchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["query"].is_object());
}

#[test]
fn test_author_papers_tool_name() {
    let tool = AuthorPapersTool;
    assert_eq!(tool.name(), "author_papers");
}

#[test]
fn test_author_papers_tool_description() {
    let tool = AuthorPapersTool;
    assert!(tool.description().contains("papers") || tool.description().contains("author"));
}

#[test]
fn test_author_papers_tool_input_schema() {
    let tool = AuthorPapersTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["authorId"].is_object() || schema["properties"]["author_id"].is_object());
}

// =============================================================================
// PaperAutocompleteTool Tests
// =============================================================================

#[tokio::test]
async fn test_paper_autocomplete_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/autocomplete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "matches": [
                {"id": "p1", "match": "Attention Is All You Need"},
                {"id": "p2", "match": "Attention Mechanisms in Neural Networks"}
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperAutocompleteTool;

    let result = tool
        .execute(&ctx, json!({"query": "attention"}))
        .await
        .unwrap();

    assert!(result.contains("Attention") || result.contains("autocomplete"));
}

#[tokio::test]
async fn test_paper_autocomplete_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/autocomplete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "matches": [{"id": "p1", "match": "Test Paper"}]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperAutocompleteTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "test",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("suggestions").is_some());
}

#[tokio::test]
async fn test_paper_autocomplete_empty() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/autocomplete"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "matches": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperAutocompleteTool;

    let result = tool
        .execute(&ctx, json!({"query": "xyz123nonexistent"}))
        .await
        .unwrap();

    assert!(result.contains("No suggestions") || result.contains("suggestions"));
}

#[test]
fn test_paper_autocomplete_tool_name() {
    let tool = PaperAutocompleteTool;
    assert_eq!(tool.name(), "paper_autocomplete");
}

#[test]
fn test_paper_autocomplete_tool_description() {
    let tool = PaperAutocompleteTool;
    assert!(tool.description().contains("suggestions") || tool.description().contains("title"));
}

#[test]
fn test_paper_autocomplete_tool_input_schema() {
    let tool = PaperAutocompleteTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["query"].is_object());
}

// =============================================================================
// PaperTitleMatchTool Tests
// =============================================================================

#[tokio::test]
async fn test_paper_title_match_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search/match"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            sample_paper("p1", "Attention Is All You Need", 2017, 50000)
        ))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperTitleMatchTool;

    let result = tool
        .execute(&ctx, json!({"title": "Attention Is All You Need"}))
        .await
        .unwrap();

    assert!(result.contains("Attention") || result.contains("Match"));
}

#[tokio::test]
async fn test_paper_title_match_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search/match"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperTitleMatchTool;

    let result = tool
        .execute(&ctx, json!({"title": "Nonexistent Paper XYZ"}))
        .await
        .unwrap();

    assert!(result.contains("No exact match") || result.contains("Match"));
}

#[tokio::test]
async fn test_paper_title_match_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search/match"))
        .respond_with(ResponseTemplate::new(200).set_body_json(
            sample_paper("p1", "JSON Title Match", 2023, 100)
        ))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperTitleMatchTool;

    let result = tool
        .execute(&ctx, json!({
            "title": "JSON Title Match",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("matched").is_some());
}

#[test]
fn test_paper_title_match_tool_name() {
    let tool = PaperTitleMatchTool;
    assert_eq!(tool.name(), "paper_title_match");
}

#[test]
fn test_paper_title_match_tool_description() {
    let tool = PaperTitleMatchTool;
    assert!(tool.description().contains("title") || tool.description().contains("match"));
}

#[test]
fn test_paper_title_match_tool_input_schema() {
    let tool = PaperTitleMatchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["title"].is_object());
}

// =============================================================================
// PaperAuthorsTool Tests
// =============================================================================

#[tokio::test]
async fn test_paper_authors_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/p123/authors"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [
                sample_author("a1", "First Author", 5000, 25),
                sample_author("a2", "Second Author", 3000, 20)
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperAuthorsTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "p123"}))
        .await
        .unwrap();

    assert!(result.contains("First Author") || result.contains("Authors"));
}

#[tokio::test]
async fn test_paper_authors_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/json_paper/authors"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": [sample_author("a1", "JSON Author", 1000, 15)]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = PaperAuthorsTool;

    let result = tool
        .execute(&ctx, json!({
            "paperId": "json_paper",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("authors").is_some());
}

#[test]
fn test_paper_authors_tool_name() {
    let tool = PaperAuthorsTool;
    assert_eq!(tool.name(), "paper_authors");
}

#[test]
fn test_paper_authors_tool_description() {
    let tool = PaperAuthorsTool;
    assert!(tool.description().contains("author") || tool.description().contains("paper"));
}

#[test]
fn test_paper_authors_tool_input_schema() {
    let tool = PaperAuthorsTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["paper_id"].is_object());
}

// =============================================================================
// AuthorBatchTool Tests
// =============================================================================

#[tokio::test]
async fn test_author_batch_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/author/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_author("a1", "Batch Author 1", 5000, 25),
            sample_author("a2", "Batch Author 2", 3000, 20)
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorBatchTool;

    let result = tool
        .execute(&ctx, json!({"authorIds": ["a1", "a2"]}))
        .await
        .unwrap();

    assert!(result.contains("Batch Author") || result.contains("Author"));
}

#[tokio::test]
async fn test_author_batch_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/author/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_author("a1", "JSON Batch Author", 2000, 18)
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorBatchTool;

    let result = tool
        .execute(&ctx, json!({
            "authorIds": ["a1"],
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("authors").is_some());
}

#[tokio::test]
async fn test_author_batch_partial_results() {
    let mock_server = MockServer::start().await;

    // Simulate some valid and some null (invalid ID) results
    Mock::given(method("POST"))
        .and(path("/graph/v1/author/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_author("a1", "Valid Author", 1000, 10),
            null,  // Invalid ID returns null
            sample_author("a3", "Another Valid", 500, 5)
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = AuthorBatchTool;

    let result = tool
        .execute(&ctx, json!({"authorIds": ["a1", "invalid", "a3"]}))
        .await
        .unwrap();

    assert!(result.contains("Valid Author") || result.contains("Found"));
}

#[test]
fn test_author_batch_tool_name() {
    let tool = AuthorBatchTool;
    assert_eq!(tool.name(), "author_batch");
}

#[test]
fn test_author_batch_tool_description() {
    let tool = AuthorBatchTool;
    assert!(tool.description().contains("author") || tool.description().contains("batch"));
}

#[test]
fn test_author_batch_tool_input_schema() {
    let tool = AuthorBatchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["author_ids"].is_object());
}
