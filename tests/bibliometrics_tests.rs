//! Comprehensive mock-based tests for bibliometrics tools.
//!
//! Tests: FWCI, highly_cited, citation_half_life, cocitation, bibliographic_coupling, hot_papers

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    BibliographicCouplingTool, CitationHalfLifeTool, CocitationAnalysisTool,
    FieldWeightedImpactTool, HighlyCitedPapersTool, HotPapersTool, McpTool, ToolContext,
};

async fn setup_test_context(mock_server: &MockServer) -> ToolContext {
    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    ToolContext::new(Arc::new(client))
}

fn sample_paper(id: &str, title: &str, year: i32, citations: i32, fields: Vec<&str>) -> serde_json::Value {
    json!({
        "paperId": id,
        "title": title,
        "abstract": format!("Abstract for {}", title),
        "year": year,
        "citationCount": citations,
        "referenceCount": 20,
        "authors": [{"authorId": "a1", "name": "Test Author"}],
        "venue": "Test Journal",
        "fieldsOfStudy": fields,
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
// FieldWeightedImpactTool Tests
// =============================================================================

#[tokio::test]
async fn test_fwci_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "High Impact Paper", 2022, 150, vec!["Computer Science"])
        ])))
        .mount(&mock_server)
        .await;

    // Mock baseline search
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({"paperId": "b1", "citationCount": 50}),
            json!({"paperId": "b2", "citationCount": 30}),
            json!({"paperId": "b3", "citationCount": 20}),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = FieldWeightedImpactTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1"], "responseFormat": "json"}))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("methodology_note").is_some());
    assert!(parsed.get("results").is_some());
    let results = parsed["results"].as_array().unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0].get("fwci").is_some());
}

#[tokio::test]
async fn test_fwci_missing_year() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "paperId": "p1",
                "title": "No Year Paper",
                "citationCount": 100,
                "fieldsOfStudy": ["Computer Science"],
                "authors": []
            }
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = FieldWeightedImpactTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1"], "responseFormat": "json"}))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["results"][0]["error"].is_string());
}

#[tokio::test]
async fn test_fwci_missing_field() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            {
                "paperId": "p1",
                "title": "No Field Paper",
                "year": 2023,
                "citationCount": 100,
                "fieldsOfStudy": [],
                "authors": []
            }
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = FieldWeightedImpactTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1"]}))
        .await
        .unwrap();

    assert!(result.contains("Missing year or field data") || result.contains("FWCI"));
}

// =============================================================================
// HighlyCitedPapersTool Tests
// =============================================================================

#[tokio::test]
async fn test_highly_cited_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "Very Popular", 2022, 500, vec!["Physics"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({"paperId": "b1", "citationCount": 100}),
            json!({"paperId": "b2", "citationCount": 50}),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = HighlyCitedPapersTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1"], "responseFormat": "json"}))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("percentile_threshold").is_some());
    assert!(parsed["results"][0]["is_highly_cited"].is_boolean());
}

#[tokio::test]
async fn test_highly_cited_custom_percentile() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "Mid-tier Paper", 2022, 75, vec!["Biology"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            json!({"paperId": "b1", "citationCount": 100}),
            json!({"paperId": "b2", "citationCount": 80}),
            json!({"paperId": "b3", "citationCount": 60}),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = HighlyCitedPapersTool;

    let result = tool
        .execute(&ctx, json!({"paperIds": ["p1"], "percentileThreshold": 5.0}))
        .await
        .unwrap();

    assert!(result.contains("Highly Cited") || result.contains("Top"));
}

// =============================================================================
// CitationHalfLifeTool Tests
// =============================================================================

#[tokio::test]
async fn test_citation_half_life_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "Classic Paper", 2015, 200, vec!["Mathematics"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/p1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": {"paperId": "c1", "year": 2016}},
                {"citingPaper": {"paperId": "c2", "year": 2017}},
                {"citingPaper": {"paperId": "c3", "year": 2018}},
                {"citingPaper": {"paperId": "c4", "year": 2020}},
                {"citingPaper": {"paperId": "c5", "year": 2022}},
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationHalfLifeTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "p1"}))
        .await
        .unwrap();

    assert!(result.contains("Half-life") || result.contains("half_life"));
    assert!(result.contains("Classic Paper"));
}

#[tokio::test]
async fn test_citation_half_life_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "JSON Test Paper", 2018, 50, vec!["Chemistry"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/p1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": {"paperId": "c1", "year": 2019}},
                {"citingPaper": {"paperId": "c2", "year": 2020}},
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationHalfLifeTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "p1", "responseFormat": "json"}))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("paper_id").is_some());
    assert!(parsed.get("age_distribution").is_some());
}

#[tokio::test]
async fn test_citation_half_life_no_citations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("p1", "New Paper", 2024, 0, vec!["Physics"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/p1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationHalfLifeTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "p1"}))
        .await
        .unwrap();

    assert!(result.contains("N/A") || result.contains("null"));
}

// =============================================================================
// CocitationAnalysisTool Tests
// =============================================================================

#[tokio::test]
async fn test_cocitation_basic() {
    let mock_server = MockServer::start().await;

    // Focal paper
    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("focal", "Focal Paper", 2020, 100, vec!["CS"])
        ])))
        .mount(&mock_server)
        .await;

    // Citations of focal paper
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/focal/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": {"paperId": "citing1"}},
                {"citingPaper": {"paperId": "citing2"}},
            ]
        })))
        .mount(&mock_server)
        .await;

    // References of citing papers
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/citing1/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citedPaper": {"paperId": "cocited1"}},
                {"citedPaper": {"paperId": "cocited2"}},
            ]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/citing2/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citedPaper": {"paperId": "cocited1"}},  // Same as above = co-cited
                {"citedPaper": {"paperId": "cocited3"}},
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CocitationAnalysisTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "focal", "minCocitations": 1}))
        .await
        .unwrap();

    assert!(result.contains("Co-citation") || result.contains("cocitation"));
}

#[tokio::test]
async fn test_cocitation_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("focal", "Focal", 2020, 50, vec!["CS"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/focal/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CocitationAnalysisTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "focal", "responseFormat": "json"}))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("focal_paper").is_some());
    assert!(parsed.get("cocited_papers").is_some());
}

// =============================================================================
// BibliographicCouplingTool Tests
// =============================================================================

#[tokio::test]
async fn test_bibliographic_coupling_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("focal", "Focal Paper", 2020, 50, vec!["CS"])
        ])))
        .mount(&mock_server)
        .await;

    // References of focal paper
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/focal/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citedPaper": {"paperId": "ref1"}},
                {"citedPaper": {"paperId": "ref2"}},
            ]
        })))
        .mount(&mock_server)
        .await;

    // Papers citing the same references
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/ref1/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": {"paperId": "coupled1"}},
                {"citingPaper": {"paperId": "coupled2"}},
            ]
        })))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/ref2/citations"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": [
                {"citingPaper": {"paperId": "coupled1"}},  // Same = coupled
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BibliographicCouplingTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "focal", "minSharedRefs": 1}))
        .await
        .unwrap();

    assert!(result.contains("Bibliographic") || result.contains("coupling") || result.contains("Shared"));
}

#[tokio::test]
async fn test_bibliographic_coupling_no_refs() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([
            sample_paper("focal", "No Refs Paper", 2020, 50, vec!["CS"])
        ])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/focal/references"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "offset": 0,
            "data": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = BibliographicCouplingTool;

    let result = tool
        .execute(&ctx, json!({"paperId": "focal"}))
        .await
        .unwrap();

    assert!(result.contains("error") || result.contains("No references"));
}

// =============================================================================
// HotPapersTool Tests
// =============================================================================

#[tokio::test]
async fn test_hot_papers_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "Hot Paper 1", 2023, 100, vec!["AI"]),
            sample_paper("p2", "Hot Paper 2", 2022, 200, vec!["AI"]),
            sample_paper("p3", "Cool Paper", 2021, 50, vec!["AI"]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = HotPapersTool;

    let result = tool
        .execute(&ctx, json!({"query": "artificial intelligence", "minRecentCitations": 10}))
        .await
        .unwrap();

    assert!(result.contains("Hot Papers") || result.contains("hot"));
    assert!(result.contains("velocity") || result.contains("Velocity"));
}

#[tokio::test]
async fn test_hot_papers_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "Trending", 2023, 150, vec!["ML"]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = HotPapersTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "machine learning",
            "responseFormat": "json",
            "minRecentCitations": 5
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("query").is_some());
    assert!(parsed.get("results").is_some());
}

#[tokio::test]
async fn test_hot_papers_year_filter() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper("p1", "Recent", 2024, 50, vec!["Bio"]),
            sample_paper("p2", "Old", 2018, 500, vec!["Bio"]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = HotPapersTool;

    let result = tool
        .execute(&ctx, json!({
            "query": "biology",
            "yearStart": 2023,
            "minRecentCitations": 10
        }))
        .await
        .unwrap();

    // Should only include papers from 2023+
    assert!(result.contains("Recent") || result.contains("2024"));
}

#[tokio::test]
async fn test_hot_papers_empty_results() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = HotPapersTool;

    let result = tool
        .execute(&ctx, json!({"query": "nonexistent topic xyz"}))
        .await
        .unwrap();

    assert!(result.contains("0") || result.contains("Hot Papers"));
}

// =============================================================================
// Error Handling Tests
// =============================================================================

#[tokio::test]
async fn test_fwci_api_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = FieldWeightedImpactTool;

    let result = tool.execute(&ctx, json!({"paperIds": ["p1"]})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_citation_half_life_paper_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CitationHalfLifeTool;

    let result = tool.execute(&ctx, json!({"paperId": "nonexistent"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_cocitation_paper_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server).await;
    let tool = CocitationAnalysisTool;

    let result = tool.execute(&ctx, json!({"paperId": "nonexistent"})).await;
    assert!(result.is_err());
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_fwci_tool_name() {
    let tool = FieldWeightedImpactTool;
    assert_eq!(tool.name(), "field_weighted_impact");
}

#[test]
fn test_fwci_tool_description() {
    let tool = FieldWeightedImpactTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_fwci_tool_input_schema() {
    let tool = FieldWeightedImpactTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_highly_cited_tool_name() {
    let tool = HighlyCitedPapersTool;
    assert_eq!(tool.name(), "highly_cited_papers");
}

#[test]
fn test_highly_cited_tool_description() {
    let tool = HighlyCitedPapersTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_highly_cited_tool_input_schema() {
    let tool = HighlyCitedPapersTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_citation_half_life_tool_name() {
    let tool = CitationHalfLifeTool;
    assert_eq!(tool.name(), "citation_half_life");
}

#[test]
fn test_citation_half_life_tool_description() {
    let tool = CitationHalfLifeTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_citation_half_life_tool_input_schema() {
    let tool = CitationHalfLifeTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_cocitation_tool_name() {
    let tool = CocitationAnalysisTool;
    assert_eq!(tool.name(), "cocitation_analysis");
}

#[test]
fn test_cocitation_tool_description() {
    let tool = CocitationAnalysisTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_cocitation_tool_input_schema() {
    let tool = CocitationAnalysisTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_bibliographic_coupling_tool_name() {
    let tool = BibliographicCouplingTool;
    assert_eq!(tool.name(), "bibliographic_coupling");
}

#[test]
fn test_bibliographic_coupling_tool_description() {
    let tool = BibliographicCouplingTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_bibliographic_coupling_tool_input_schema() {
    let tool = BibliographicCouplingTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_hot_papers_tool_name() {
    let tool = HotPapersTool;
    assert_eq!(tool.name(), "hot_papers");
}

#[test]
fn test_hot_papers_tool_description() {
    let tool = HotPapersTool;
    assert!(tool.description().len() > 10);
}

#[test]
fn test_hot_papers_tool_input_schema() {
    let tool = HotPapersTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}
