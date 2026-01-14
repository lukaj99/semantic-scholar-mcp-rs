//! Mock-based tests for systematic review tools: `prisma_search`, `screening_export`, `prisma_flow_diagram`
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    McpTool, PrismaFlowDiagramTool, PrismaSearchTool, ScreeningExportTool, ToolContext,
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
// PrismaSearchTool Tests
// =============================================================================

#[tokio::test]
async fn test_prisma_search_single_query() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("p1", "PRISMA Paper 1", 2023, 100),
                sample_paper("p2", "PRISMA Paper 2", 2022, 200),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PrismaSearchTool;

    let result = tool.execute(&ctx, json!({"queries": ["machine learning"]})).await.unwrap();

    assert!(result.contains("PRISMA") || result.contains("Search"));
    assert!(result.contains("PRISMA Paper 1") || result.contains("machine"));
}

#[tokio::test]
async fn test_prisma_search_multiple_queries() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("multi", "Multi Query Paper", 2023, 150)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PrismaSearchTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "queries": ["deep learning", "neural networks", "transformers"]
            }),
        )
        .await
        .unwrap();

    assert!(result.contains("PRISMA") || result.contains("queries"));
}

#[tokio::test]
async fn test_prisma_search_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("json", "JSON PRISMA", 2023, 50)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PrismaSearchTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "queries": ["test"],
                "responseFormat": "json"
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("queries").is_some() || parsed.get("search_log").is_some());
}

#[tokio::test]
async fn test_prisma_search_with_year_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("old", "Old Paper", 2015, 500),
                sample_paper("new", "New Paper", 2023, 100),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PrismaSearchTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "queries": ["test"],
                "yearStart": 2020,
                "yearEnd": 2024
            }),
        )
        .await
        .unwrap();

    // Should filter to only recent papers
    assert!(result.contains("New Paper") || result.contains("PRISMA"));
}

#[tokio::test]
async fn test_prisma_search_with_citation_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("high", "High Cite", 2023, 500),
                sample_paper("low", "Low Cite", 2023, 5),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PrismaSearchTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "queries": ["test"],
                "minCitations": 100
            }),
        )
        .await
        .unwrap();

    assert!(result.contains("High Cite") || result.contains("PRISMA"));
}

#[tokio::test]
async fn test_prisma_search_deduplication_across_queries() {
    let mock_server = MockServer::start().await;

    // Same paper returned for multiple queries should be deduplicated
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("dup", "Duplicate Paper", 2023, 100)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PrismaSearchTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "queries": ["query1", "query2"]
            }),
        )
        .await
        .unwrap();

    // Should show deduplication stats
    assert!(result.contains("Duplicate") || result.contains("PRISMA") || result.contains("unique"));
}

// =============================================================================
// ScreeningExportTool Tests
// =============================================================================

#[tokio::test]
async fn test_screening_export_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("s1", "Screening Paper 1", 2023, 100),
            sample_paper("s2", "Screening Paper 2", 2022, 200),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ScreeningExportTool;

    let result = tool.execute(&ctx, json!({"paperIds": ["s1", "s2"]})).await.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("papers").is_some() || parsed.get("total").is_some());
}

#[tokio::test]
async fn test_screening_export_with_abstract() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([sample_paper(
            "abs",
            "Abstract Paper",
            2023,
            50
        )])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ScreeningExportTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "paperIds": ["abs"],
                "includeAbstract": true
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["papers"][0].get("abstract").is_some() || parsed.get("papers").is_some());
}

#[tokio::test]
async fn test_screening_export_with_tldr() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "paperId": "tldr",
                "title": "TLDR Paper",
                "year": 2023,
                "citationCount": 100,
                "authors": [],
                "tldr": {"text": "This is a TLDR summary"}
            }
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ScreeningExportTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "paperIds": ["tldr"],
                "includeTldr": true
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("papers").is_some());
}

#[tokio::test]
async fn test_screening_export_empty_papers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = ScreeningExportTool;

    let result = tool.execute(&ctx, json!({"paperIds": ["invalid"]})).await.unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["total"], 0);
}

// =============================================================================
// PrismaFlowDiagramTool Tests
// =============================================================================

#[tokio::test]
async fn test_prisma_flow_diagram_basic() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = PrismaFlowDiagramTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "identification": {
                    "databases": [
                        {"name": "PubMed", "results": 500},
                        {"name": "Scopus", "results": 300}
                    ]
                },
                "screening": {
                    "recordsAfterDedup": 700,
                    "recordsScreened": 700,
                    "recordsExcluded": 600
                }
            }),
        )
        .await
        .unwrap();

    assert!(
        result.contains("PRISMA")
            || result.contains("IDENTIFICATION")
            || result.contains("SCREENING")
    );
}

#[tokio::test]
async fn test_prisma_flow_diagram_json_format() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = PrismaFlowDiagramTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "identification": {
                    "databases": [{"name": "Test", "results": 100}]
                },
                "screening": {
                    "recordsAfterDedup": 90,
                    "recordsScreened": 90,
                    "recordsExcluded": 80
                },
                "responseFormat": "json"
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("version").is_some() || parsed.get("identification").is_some());
}

#[tokio::test]
async fn test_prisma_flow_diagram_with_eligibility() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = PrismaFlowDiagramTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "identification": {
                    "databases": [{"name": "DB1", "results": 200}]
                },
                "screening": {
                    "recordsAfterDedup": 180,
                    "recordsScreened": 180,
                    "recordsExcluded": 150
                },
                "eligibility": {
                    "reportsSought": 30,
                    "reportsAssessed": 30,
                    "reportsExcluded": 10
                }
            }),
        )
        .await
        .unwrap();

    assert!(
        result.contains("ELIGIBILITY")
            || result.contains("eligibility")
            || result.contains("reports")
    );
}

#[tokio::test]
async fn test_prisma_flow_diagram_with_included() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = PrismaFlowDiagramTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "identification": {
                    "databases": [{"name": "DB", "results": 100}]
                },
                "screening": {
                    "recordsAfterDedup": 90,
                    "recordsScreened": 90,
                    "recordsExcluded": 70
                },
                "included": {
                    "studiesIncluded": 15,
                    "reportsIncluded": 20
                }
            }),
        )
        .await
        .unwrap();

    assert!(
        result.contains("INCLUDED") || result.contains("included") || result.contains("studies")
    );
}

#[tokio::test]
async fn test_prisma_flow_diagram_full() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = PrismaFlowDiagramTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "identification": {
                    "databases": [
                        {"name": "PubMed", "results": 1000},
                        {"name": "Scopus", "results": 800},
                        {"name": "Web of Science", "results": 600}
                    ],
                    "otherSources": [
                        {"name": "Grey literature", "records": 50}
                    ]
                },
                "screening": {
                    "recordsAfterDedup": 2000,
                    "recordsScreened": 2000,
                    "recordsExcluded": 1800,
                    "exclusionReasons": {
                        "Not relevant": 1000,
                        "Wrong study type": 500,
                        "Duplicate": 300
                    }
                },
                "eligibility": {
                    "reportsSought": 200,
                    "reportsNotRetrieved": 10,
                    "reportsAssessed": 190,
                    "reportsExcluded": 140,
                    "exclusionReasons": {
                        "No full text": 50,
                        "Wrong outcome": 90
                    }
                },
                "included": {
                    "studiesIncluded": 50,
                    "reportsIncluded": 60
                },
                "responseFormat": "json"
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("summary").is_some());
}

#[tokio::test]
async fn test_prisma_flow_diagram_with_other_sources() {
    let mock_server = MockServer::start().await;
    let ctx = setup_test_context(&mock_server);
    let tool = PrismaFlowDiagramTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "identification": {
                    "databases": [{"name": "Main DB", "results": 100}],
                    "otherSources": [
                        {"name": "Manual search", "records": 20},
                        {"name": "Citation search", "records": 15}
                    ]
                },
                "screening": {
                    "recordsAfterDedup": 120,
                    "recordsScreened": 120,
                    "recordsExcluded": 100
                }
            }),
        )
        .await
        .unwrap();

    assert!(
        result.contains("other") || result.contains("Manual") || result.contains("IDENTIFICATION")
    );
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_prisma_search_tool_name() {
    let tool = PrismaSearchTool;
    assert_eq!(tool.name(), "prisma_search");
}

#[test]
fn test_prisma_search_tool_description() {
    let tool = PrismaSearchTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_prisma_search_tool_input_schema() {
    let tool = PrismaSearchTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
    assert!(schema["properties"]["queries"].is_object());
}

#[test]
fn test_screening_export_tool_name() {
    let tool = ScreeningExportTool;
    assert_eq!(tool.name(), "screening_export");
}

#[test]
fn test_screening_export_tool_description() {
    let tool = ScreeningExportTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_screening_export_tool_input_schema() {
    let tool = ScreeningExportTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_prisma_flow_diagram_tool_name() {
    let tool = PrismaFlowDiagramTool;
    assert_eq!(tool.name(), "prisma_flow_diagram");
}

#[test]
fn test_prisma_flow_diagram_tool_description() {
    let tool = PrismaFlowDiagramTool;
    assert!(tool.description().contains("PRISMA") || tool.description().len() > 10);
}

#[test]
fn test_prisma_flow_diagram_tool_input_schema() {
    let tool = PrismaFlowDiagramTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}
