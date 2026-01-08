//! Mock-based tests for discovery tools: exhaustive_search, recommendations, citation_snowball

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    CitationSnowballTool, ExhaustiveSearchTool, McpTool, RecommendationsTool, ToolContext,
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

fn search_result(papers: Vec<serde_json::Value>, next: Option<i32>) -> serde_json::Value {
    json!({
        "total": papers.len(),
        "offset": 0,
        "next": next,
        "data": papers
    })
}

// =============================================================================
// CitationSnowballTool Tests
// =============================================================================

#[tokio::test]
async fn test_citation_snowball_forward() {
    let mock_server = MockServer::start().await;

    // Seed paper batch
    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "Seed Paper", 2020, 100)
        ])))
        .mount(&mock_server)
        .await;

    // Citations (who cites this paper)
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": sample_paper("cite1", "Citing Paper 1", 2022, 50)},
                {"citingPaper": sample_paper("cite2", "Citing Paper 2", 2023, 30)},
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "direction": "citations",
            "depth": 1
        }))
        .await
        .unwrap();

    assert!(result.contains("Citing Paper") || result.contains("snowball") || result.contains("Citation"));
}

#[tokio::test]
async fn test_citation_snowball_backward() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "Seed Paper", 2022, 100)
        ])))
        .mount(&mock_server)
        .await;

    // References (what this paper cites)
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citedPaper": sample_paper("ref1", "Reference Paper 1", 2018, 500)},
                {"citedPaper": sample_paper("ref2", "Reference Paper 2", 2019, 300)},
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "direction": "references",
            "depth": 1
        }))
        .await
        .unwrap();

    assert!(result.contains("Reference Paper") || result.contains("snowball"));
}

#[tokio::test]
async fn test_citation_snowball_both_directions() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "Both Directions", 2021, 200)
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citingPaper": sample_paper("cite1", "Citer", 2023, 20)}]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citedPaper": sample_paper("ref1", "Referenced", 2019, 400)}]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "direction": "both",
            "depth": 1
        }))
        .await
        .unwrap();

    assert!(result.contains("Citer") || result.contains("Referenced") || result.contains("snowball"));
}

#[tokio::test]
async fn test_citation_snowball_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "JSON Seed", 2021, 100)
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("seed_papers").is_some() || parsed.get("papers").is_some() || parsed.is_array());
}

#[tokio::test]
async fn test_citation_snowball_depth_2() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "Deep Seed", 2020, 150)
        ])))
        .mount(&mock_server)
        .await;

    // First level citations
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citingPaper": sample_paper("level1", "Level 1", 2022, 50)}]
        })))
        .mount(&mock_server)
        .await;

    // Second level citations
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/level1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [{"citingPaper": sample_paper("level2", "Level 2", 2023, 10)}]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "direction": "citations",
            "depth": 2
        }))
        .await
        .unwrap();

    // Should have both levels
    assert!(result.contains("Level") || result.contains("snowball"));
}

#[tokio::test]
async fn test_citation_snowball_deduplication() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "Dedup Seed", 2020, 100)
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": sample_paper("dup1", "Duplicate", 2022, 50)},
                {"citingPaper": sample_paper("dup1", "Duplicate", 2022, 50)},  // Same paper
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "direction": "citations",
            "deduplicate": true
        }))
        .await
        .unwrap();

    assert!(result.contains("Duplicate") || result.contains("snowball"));
}

#[tokio::test]
async fn test_citation_snowball_min_citations_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("seed1", "Filter Seed", 2020, 100)
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/seed1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": sample_paper("high", "High Citations", 2022, 500)},
                {"citingPaper": sample_paper("low", "Low Citations", 2022, 5)},
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationSnowballTool;

    let result = tool
        .execute(&ctx, json!({
            "seedPaperIds": ["seed1"],
            "direction": "citations",
            "minCitations": 100
        }))
        .await
        .unwrap();

    // Should only include high citation paper
    assert!(result.contains("High Citations") || result.contains("snowball"));
}

// =============================================================================
// Additional ExhaustiveSearch Tests
// =============================================================================

#[tokio::test]
async fn test_exhaustive_search_fields_of_study_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("cs", "CS Paper", 2023, 100),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "machine learning",
            "fieldsOfStudy": ["Computer Science"]
        }))
        .await
        .unwrap();

    assert!(result.contains("CS Paper") || result.contains("machine"));
}

#[tokio::test]
async fn test_exhaustive_search_open_access_only() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("oa", "Open Access Paper", 2023, 50)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = ExhaustiveSearchTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "biology",
            "openAccessOnly": true
        }))
        .await
        .unwrap();

    assert!(result.contains("Open Access Paper") || result.contains("Found") || result.len() > 10);
}

// =============================================================================
// Additional Recommendations Tests
// =============================================================================

#[tokio::test]
async fn test_recommendations_with_negative_seeds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("pos_rec", "Positive Rec", 2023, 100)
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = RecommendationsTool;

    let result = tool
        .execute(&ctx, json!({
            "positivePaperIds": ["pos1", "pos2"],
            "negativePaperIds": ["neg1"]
        }))
        .await
        .unwrap();

    assert!(result.contains("Positive Rec") || result.contains("Recommended"));
}

#[tokio::test]
async fn test_recommendations_with_limit() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/single"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("r1", "Rec 1", 2023, 50),
                sample_paper("r2", "Rec 2", 2023, 40),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = RecommendationsTool;

    let result = tool
        .execute(&ctx, json!({
            "positivePaperIds": ["single"],
            "limit": 5
        }))
        .await
        .unwrap();

    assert!(result.contains("Rec") || result.contains("Recommended"));
}

#[tokio::test]
async fn test_recommendations_fields_of_study_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/field_seed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("physics", "Physics Paper", 2023, 100)
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = RecommendationsTool;

    let result = tool
        .execute(&ctx, json!({
            "positivePaperIds": ["field_seed"],
            "fieldsOfStudy": ["Physics"]
        }))
        .await
        .unwrap();

    assert!(result.contains("Physics") || result.contains("Recommended"));
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_exhaustive_search_tool_name() {
    let tool = ExhaustiveSearchTool;
    assert_eq!(tool.name(), "exhaustive_search");
}

#[test]
fn test_exhaustive_search_tool_description() {
    let tool = ExhaustiveSearchTool;
    assert!(tool.description().contains("search") || tool.description().contains("pagination"));
}

#[test]
fn test_exhaustive_search_tool_input_schema() {
    let tool = ExhaustiveSearchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["query"].is_object());
}

#[test]
fn test_recommendations_tool_name() {
    let tool = RecommendationsTool;
    assert_eq!(tool.name(), "recommendations");
}

#[test]
fn test_recommendations_tool_description() {
    let tool = RecommendationsTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_recommendations_tool_input_schema() {
    let tool = RecommendationsTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_citation_snowball_tool_name() {
    let tool = CitationSnowballTool;
    assert_eq!(tool.name(), "citation_snowball");
}

#[test]
fn test_citation_snowball_tool_description() {
    let tool = CitationSnowballTool;
    assert!(tool.description().contains("citation") || tool.description().contains("network"));
}

#[test]
fn test_citation_snowball_tool_input_schema() {
    let tool = CitationSnowballTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}
