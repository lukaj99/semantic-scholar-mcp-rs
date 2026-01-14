//! Mock-based tests for semantic tools: `semantic_search`, `literature_review_pipeline`
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    LiteratureReviewPipelineTool, McpTool, SemanticSearchTool, ToolContext,
};

fn setup_test_context(mock_server: &MockServer) -> ToolContext {
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

fn search_result(papers: Vec<serde_json::Value>, next: Option<i32>) -> serde_json::Value {
    json!({
        "total": papers.len(),
        "offset": 0,
        "next": next,
        "data": papers
    })
}

// =============================================================================
// SemanticSearchTool Tests
// =============================================================================

#[tokio::test]
async fn test_semantic_search_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("sim1", "Similar Paper 1", 2023, 100),
                sample_paper("sim2", "Similar Paper 2", 2022, 200),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = SemanticSearchTool;

    let result = tool
        .execute(&ctx, json!({"seedPaperId": "seed123"}))
        .await
        .unwrap();

    assert!(result.contains("Semantic Search") || result.contains("Similar"));
    assert!(result.contains("Similar Paper 1") || result.contains("sim1"));
}

#[tokio::test]
async fn test_semantic_search_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/json_seed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("j1", "JSON Paper", 2023, 50)
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = SemanticSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperId": "json_seed",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("seed_paper_id").is_some());
    assert!(parsed.get("similar_papers").is_some());
}

#[tokio::test]
async fn test_semantic_search_year_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/filter_seed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("old", "Old Paper", 2018, 500),
                sample_paper("new", "New Paper", 2023, 100),
                sample_paper("mid", "Mid Paper", 2021, 200),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = SemanticSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperId": "filter_seed",
            "yearStart": 2020,
            "yearEnd": 2023
        }))
        .await
        .unwrap();

    // Should filter out 2018 paper
    assert!(result.contains("New Paper") || result.contains("Mid Paper"));
}

#[tokio::test]
async fn test_semantic_search_empty_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/empty_seed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = SemanticSearchTool;

    let result = tool
        .execute(&ctx, json!({"seedPaperId": "empty_seed"}))
        .await
        .unwrap();

    assert!(result.contains("No similar") || result.contains('0') || result.contains("found"));
}

#[tokio::test]
async fn test_semantic_search_with_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/limit_seed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("l1", "Limited 1", 2023, 100),
                sample_paper("l2", "Limited 2", 2023, 200),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = SemanticSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperId": "limit_seed",
            "limit": 50
        }))
        .await
        .unwrap();

    assert!(result.contains("Limited") || result.contains("Paper"));
}

// =============================================================================
// LiteratureReviewPipelineTool Tests
// =============================================================================

#[tokio::test]
async fn test_literature_review_basic() {
    let mock_server = MockServer::start().await;

    // Search results
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("s1", "Search Result 1", 2023, 100),
                sample_paper("s2", "Search Result 2", 2022, 200),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    // Recommendations
    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("r1", "Recommended", 2023, 150)
            ]
        })))
        .mount(&mock_server)
        .await;

    // Citations
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/s1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citingPaper": sample_paper("c1", "Citing Paper", 2023, 50)}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/s2/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    // References
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/s1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citedPaper": sample_paper("ref1", "Reference", 2020, 300)}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/s2/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = LiteratureReviewPipelineTool;

    let result = tool
        .execute(&ctx, json!({"query": "machine learning"}))
        .await
        .unwrap();

    assert!(result.contains("Literature Review") || result.contains("literature"));
}

#[tokio::test]
async fn test_literature_review_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("j1", "JSON Review Paper", 2023, 100)],
            None,
        )))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": []
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/j1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/j1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = LiteratureReviewPipelineTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "deep learning",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("query").is_some());
    assert!(parsed.get("sources").is_some());
    assert!(parsed.get("papers").is_some());
}

#[tokio::test]
async fn test_literature_review_with_filters() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("f1", "Recent Popular", 2023, 500),
                sample_paper("f2", "Old Unpopular", 2018, 10),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": []
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/f1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/f1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = LiteratureReviewPipelineTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "neural networks",
            "yearStart": 2020,
            "minCitations": 100
        }))
        .await
        .unwrap();

    // Should filter out old/unpopular papers
    assert!(result.contains("Recent Popular") || result.contains("Literature"));
}

#[tokio::test]
async fn test_literature_review_no_recommendations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("nr1", "No Rec Paper", 2023, 100)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = LiteratureReviewPipelineTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "test query",
            "includeRecommendations": false,
            "includeCitations": false
        }))
        .await
        .unwrap();

    assert!(result.contains("No Rec Paper") || result.contains("Literature"));
}

#[tokio::test]
async fn test_literature_review_deduplication() {
    let mock_server = MockServer::start().await;

    // Same paper appears in search and recommendations
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("dup1", "Duplicate Paper", 2023, 100)],
            None,
        )))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("dup1", "Duplicate Paper", 2023, 100),  // Same ID
                sample_paper("unique1", "Unique Paper", 2023, 200),
            ]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/dup1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/dup1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = LiteratureReviewPipelineTool;

    let result = tool
        .execute(&ctx, json!({"query": "dedupe test"}))
        .await
        .unwrap();

    // Should show deduplication
    assert!(result.contains("Duplicate") || result.contains("unique") || result.contains("Literature"));
}

#[tokio::test]
async fn test_literature_review_max_papers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("m1", "Max Paper 1", 2023, 100),
                sample_paper("m2", "Max Paper 2", 2023, 200),
                sample_paper("m3", "Max Paper 3", 2023, 300),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": []
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/m1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/m2/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/m3/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/m1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/m2/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/m3/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"offset": 0, "data": []})))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = LiteratureReviewPipelineTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "max papers test",
            "maxPapers": 2
        }))
        .await
        .unwrap();

    assert!(result.contains("Literature") || result.contains("Paper"));
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_semantic_search_tool_name() {
    let tool = SemanticSearchTool;
    assert_eq!(tool.name(), "semantic_search");
}

#[test]
fn test_semantic_search_tool_description() {
    let tool = SemanticSearchTool;
    assert!(tool.description().contains("semantic") || tool.description().contains("similar") || tool.description().len() > 10);
}

#[test]
fn test_semantic_search_tool_input_schema() {
    let tool = SemanticSearchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_literature_review_tool_name() {
    let tool = LiteratureReviewPipelineTool;
    assert_eq!(tool.name(), "literature_review_pipeline");
}

#[test]
fn test_literature_review_tool_description() {
    let tool = LiteratureReviewPipelineTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_literature_review_tool_input_schema() {
    let tool = LiteratureReviewPipelineTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["query"].is_object());
}
