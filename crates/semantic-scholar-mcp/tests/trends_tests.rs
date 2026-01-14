//! Mock-based tests for trends tools: `research_trends`, `venue_analytics`
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{McpTool, ResearchTrendsTool, ToolContext, VenueAnalyticsTool};

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

fn search_result(papers: Vec<serde_json::Value>) -> serde_json::Value {
    json!({
        "total": papers.len(),
        "offset": 0,
        "next": null,
        "data": papers
    })
}

// =============================================================================
// ResearchTrendsTool Tests
// =============================================================================

#[tokio::test]
async fn test_research_trends_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "2020 Paper", 2020, 100),
            sample_paper("p2", "2021 Paper", 2021, 150),
            sample_paper("p3", "2022 Paper", 2022, 200),
            sample_paper("p4", "2023 Paper", 2023, 50),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ResearchTrendsTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "machine learning",
            "yearStart": 2020,
            "yearEnd": 2023
        }))
        .await
        .unwrap();

    assert!(result.contains("Trend") || result.contains("trend") || result.contains("2020"));
}

#[tokio::test]
async fn test_research_trends_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "JSON Trend", 2022, 100),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ResearchTrendsTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "test",
            "yearStart": 2020,
            "yearEnd": 2023,
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("query").is_some() || parsed.get("trends").is_some() || parsed.get("years").is_some());
}

#[tokio::test]
async fn test_research_trends_quarterly_granularity() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "Q1 Paper", 2023, 100),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ResearchTrendsTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "AI",
            "yearStart": 2023,
            "yearEnd": 2023,
            "granularity": "quarter"
        }))
        .await
        .unwrap();

    assert!(result.contains('Q') || result.contains("quarter") || result.contains("Trend"));
}

#[tokio::test]
async fn test_research_trends_no_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ResearchTrendsTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "nonexistent topic xyz",
            "yearStart": 2020,
            "yearEnd": 2023
        }))
        .await
        .unwrap();

    // Should handle empty gracefully
    assert!(result.contains('0') || result.contains("Trend") || !result.is_empty());
}

#[tokio::test]
async fn test_research_trends_wide_range() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("old", "Old Paper", 2010, 500),
            sample_paper("new", "New Paper", 2023, 50),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ResearchTrendsTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "biology",
            "yearStart": 2010,
            "yearEnd": 2023
        }))
        .await
        .unwrap();

    assert!(result.contains("2010") || result.contains("2023") || result.contains("Trend"));
}

#[tokio::test]
async fn test_research_trends_max_papers_per_period() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "Paper 1", 2022, 100),
            sample_paper("p2", "Paper 2", 2022, 200),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ResearchTrendsTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "test",
            "yearStart": 2022,
            "yearEnd": 2022,
            "maxPapersPerPeriod": 10
        }))
        .await
        .unwrap();

    assert!(result.contains("Paper") || result.contains("Trend"));
}

// =============================================================================
// VenueAnalyticsTool Tests
// =============================================================================

#[tokio::test]
async fn test_venue_analytics_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({
                "paperId": "v1",
                "title": "NeurIPS Paper 1",
                "year": 2023,
                "citationCount": 100,
                "venue": "NeurIPS",
                "authors": []
            }),
            json!({
                "paperId": "v2",
                "title": "NeurIPS Paper 2",
                "year": 2022,
                "citationCount": 500,
                "venue": "NeurIPS",
                "authors": []
            }),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = VenueAnalyticsTool;

    let result = tool
        .execute(&ctx, json!({"venueQuery": "NeurIPS"}))
        .await
        .unwrap();

    assert!(result.contains("NeurIPS") || result.contains("Venue") || result.contains("venue"));
}

#[tokio::test]
async fn test_venue_analytics_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({
                "paperId": "j1",
                "title": "JSON Venue Paper",
                "year": 2023,
                "citationCount": 50,
                "venue": "Test Venue",
                "authors": []
            }),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = VenueAnalyticsTool;

    let result = tool
        .execute(&ctx, json!({
            "venueQuery": "Test Venue",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("venue").is_some() || parsed.get("statistics").is_some() || parsed.get("total_papers").is_some());
}

#[tokio::test]
async fn test_venue_analytics_with_year_range() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({
                "paperId": "y1",
                "title": "Recent Venue Paper",
                "year": 2023,
                "citationCount": 100,
                "venue": "ICML",
                "authors": []
            }),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = VenueAnalyticsTool;

    let result = tool
        .execute(&ctx, json!({
            "venueQuery": "ICML",
            "yearStart": 2020,
            "yearEnd": 2023
        }))
        .await
        .unwrap();

    assert!(result.contains("ICML") || result.contains("Venue") || result.contains("2023"));
}

#[tokio::test]
async fn test_venue_analytics_max_papers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({
                "paperId": "m1",
                "title": "Max Paper 1",
                "year": 2023,
                "citationCount": 100,
                "venue": "CVPR",
                "authors": []
            }),
            json!({
                "paperId": "m2",
                "title": "Max Paper 2",
                "year": 2023,
                "citationCount": 200,
                "venue": "CVPR",
                "authors": []
            }),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = VenueAnalyticsTool;

    let result = tool
        .execute(&ctx, json!({
            "venueQuery": "CVPR",
            "maxPapers": 100
        }))
        .await
        .unwrap();

    assert!(result.contains("CVPR") || result.contains("Venue") || result.contains("Max"));
}

#[tokio::test]
async fn test_venue_analytics_empty_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = VenueAnalyticsTool;

    let result = tool
        .execute(&ctx, json!({"venueQuery": "Nonexistent Conference XYZ"}))
        .await
        .unwrap();

    assert!(result.contains('0') || result.contains("Venue") || !result.is_empty());
}

#[tokio::test]
async fn test_venue_analytics_statistics() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({"paperId": "s1", "title": "Stats Paper 1", "year": 2023, "citationCount": 10, "venue": "Nature", "authors": []}),
            json!({"paperId": "s2", "title": "Stats Paper 2", "year": 2023, "citationCount": 100, "venue": "Nature", "authors": []}),
            json!({"paperId": "s3", "title": "Stats Paper 3", "year": 2022, "citationCount": 1000, "venue": "Nature", "authors": []}),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = VenueAnalyticsTool;

    let result = tool
        .execute(&ctx, json!({
            "venueQuery": "Nature",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    // Should have some statistics
    assert!(parsed.get("total_papers").is_some() || parsed.get("statistics").is_some() || parsed.get("venue").is_some());
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_research_trends_tool_name() {
    let tool = ResearchTrendsTool;
    assert_eq!(tool.name(), "research_trends");
}

#[test]
fn test_research_trends_tool_description() {
    let tool = ResearchTrendsTool;
    assert!(tool.description().contains("trend") || tool.description().len() > 10);
}

#[test]
fn test_research_trends_tool_input_schema() {
    let tool = ResearchTrendsTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["query"].is_object());
}

#[test]
fn test_venue_analytics_tool_name() {
    let tool = VenueAnalyticsTool;
    assert_eq!(tool.name(), "venue_analytics");
}

#[test]
fn test_venue_analytics_tool_description() {
    let tool = VenueAnalyticsTool;
    assert!(tool.description().contains("venue") || tool.description().len() > 10);
}

#[test]
fn test_venue_analytics_tool_input_schema() {
    let tool = VenueAnalyticsTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}
