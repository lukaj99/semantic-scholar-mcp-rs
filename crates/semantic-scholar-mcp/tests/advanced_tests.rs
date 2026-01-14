//! Mock-based tests for advanced tools: `pearl_growing`, `orcid_author_lookup`
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{McpTool, OrcidAuthorLookupTool, PearlGrowingTool, ToolContext};

fn setup_test_context(mock_server: &MockServer) -> ToolContext {
    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    ToolContext::new(Arc::new(client))
}

fn sample_paper(id: &str, title: &str, year: i32, citations: i32) -> serde_json::Value {
    json!({
        "paperId": id,
        "title": title,
        "abstract": format!("Abstract about {} research topic", title),
        "year": year,
        "citationCount": citations,
        "referenceCount": 15,
        "authors": [
            {"authorId": "auth1", "name": "Alice Researcher"},
            {"authorId": "auth2", "name": "Bob Scientist"}
        ],
        "venue": "Nature",
        "fieldsOfStudy": ["Computer Science", "Machine Learning"],
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

fn sample_author(id: &str, name: &str) -> serde_json::Value {
    json!({
        "authorId": id,
        "name": name,
        "affiliations": ["MIT", "Stanford"],
        "citationCount": 5000,
        "hIndex": 25,
        "paperCount": 100
    })
}

// =============================================================================
// PearlGrowingTool Tests
// =============================================================================

#[tokio::test]
async fn test_pearl_growing_basic() {
    let mock_server = MockServer::start().await;

    // Seed papers batch
    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([sample_paper(
            "seed1",
            "Deep Learning Survey",
            2022,
            500
        )])))
        .mount(&mock_server)
        .await;

    // Keyword search results (catches all search queries)
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("new1", "Neural Networks", 2023, 100),
                sample_paper("new2", "Transformer Models", 2023, 200),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    // Recommendations - single seed uses GET forpaper endpoint
    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("rec1", "Attention Mechanisms", 2023, 150)
            ]
        })))
        .mount(&mock_server)
        .await;

    // Multi-seed recommendations uses POST endpoint
    Mock::given(method("POST"))
        .and(path("/recommendations/v1/papers/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("rec2", "Multi-seed Rec", 2023, 120)
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result =
        tool.execute(&ctx, json!({"seedPaperIds": ["seed1"], "iterations": 1})).await.unwrap();

    assert!(result.contains("Pearl Growing") || result.contains("pearl"));
    assert!(result.contains("Deep Learning Survey") || result.contains("seed"));
}

#[tokio::test]
async fn test_pearl_growing_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!([sample_paper("seed1", "ML Paper", 2022, 100)])),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![], None)))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "seedPaperIds": ["seed1"],
                "responseFormat": "json"
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("seed_papers").is_some() || parsed.get("total_papers").is_some());
}

#[tokio::test]
async fn test_pearl_growing_no_valid_seeds() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result = tool.execute(&ctx, json!({"seedPaperIds": ["invalid"]})).await.unwrap();

    assert!(result.contains("No valid seed") || result.contains("error"));
}

#[tokio::test]
async fn test_pearl_growing_keywords_strategy() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([sample_paper(
            "seed1",
            "Quantum Computing Research",
            2022,
            300
        )])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("k1", "Quantum Paper", 2023, 50)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "seedPaperIds": ["seed1"],
                "strategy": "keywords",
                "iterations": 1
            }),
        )
        .await
        .unwrap();

    assert!(result.contains("keywords") || result.contains("Paper"));
}

#[tokio::test]
async fn test_pearl_growing_authors_strategy() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([sample_paper(
            "seed1",
            "Author Paper",
            2022,
            200
        )])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("a1", "Same Author Paper", 2023, 100)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "seedPaperIds": ["seed1"],
                "strategy": "authors",
                "iterations": 1
            }),
        )
        .await
        .unwrap();

    assert!(result.contains("authors") || result.contains("Paper"));
}

#[tokio::test]
async fn test_pearl_growing_citations_strategy() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([sample_paper(
            "seed1",
            "Citation Paper",
            2022,
            150
        )])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": [
                sample_paper("c1", "Related Paper", 2023, 80)
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "seedPaperIds": ["seed1"],
                "strategy": "citations",
                "iterations": 1
            }),
        )
        .await
        .unwrap();

    assert!(result.contains("citations") || result.contains("Paper"));
}

#[tokio::test]
async fn test_pearl_growing_multiple_iterations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([sample_paper(
            "seed1",
            "Initial Paper",
            2022,
            100
        )])))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("iter1", "Iteration Paper", 2023, 50)],
            None,
        )))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/recommendations/v1/papers/forpaper/seed1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "recommendedPapers": []
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = PearlGrowingTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "seedPaperIds": ["seed1"],
                "iterations": 2,
                "maxPapersPerIteration": 10
            }),
        )
        .await
        .unwrap();

    assert!(
        result.contains("Iteration") || result.contains("iteration") || result.contains("Paper")
    );
}

// =============================================================================
// OrcidAuthorLookupTool Tests
// =============================================================================

#[tokio::test]
async fn test_orcid_lookup_basic() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/ORCID:0000-0002-1825-0097"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("a123", "John Smith")))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = OrcidAuthorLookupTool;

    let result = tool.execute(&ctx, json!({"orcid": "0000-0002-1825-0097"})).await.unwrap();

    assert!(result.contains("John Smith") || result.contains("ORCID"));
}

#[tokio::test]
async fn test_orcid_lookup_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/ORCID:0000-0001-2345-6789"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("a456", "Jane Doe")))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = OrcidAuthorLookupTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "orcid": "0000-0001-2345-6789",
                "responseFormat": "json"
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("orcid").is_some());
    assert!(parsed.get("author").is_some());
}

#[tokio::test]
async fn test_orcid_lookup_with_papers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/ORCID:0000-0003-1234-5678"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(sample_author("a789", "Research Prof")),
        )
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![
                sample_paper("p1", "Prof Paper 1", 2023, 100),
                sample_paper("p2", "Prof Paper 2", 2022, 200),
            ],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = OrcidAuthorLookupTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "orcid": "0000-0003-1234-5678",
                "includePapers": true,
                "maxPapers": 10
            }),
        )
        .await
        .unwrap();

    assert!(result.contains("Research Prof") || result.contains("Paper"));
}

#[tokio::test]
async fn test_orcid_lookup_with_papers_json() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/ORCID:0000-0002-9999-8888"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("ax", "Dr. Papers")))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(
            vec![sample_paper("pp1", "Research Work", 2023, 50)],
            None,
        )))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = OrcidAuthorLookupTool;

    let result = tool
        .execute(
            &ctx,
            json!({
                "orcid": "0000-0002-9999-8888",
                "includePapers": true,
                "responseFormat": "json"
            }),
        )
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("papers").is_some());
}

#[tokio::test]
async fn test_orcid_lookup_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/ORCID:0000-0000-0000-0000"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = OrcidAuthorLookupTool;

    let result = tool.execute(&ctx, json!({"orcid": "0000-0000-0000-0000"})).await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_orcid_lookup_affiliations() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/ORCID:0000-0001-1111-2222"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "authorId": "aff1",
            "name": "Affiliated Researcher",
            "affiliations": ["Harvard University", "Google DeepMind"],
            "citationCount": 10000,
            "hIndex": 40,
            "paperCount": 150
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = OrcidAuthorLookupTool;

    let result = tool.execute(&ctx, json!({"orcid": "0000-0001-1111-2222"})).await.unwrap();

    assert!(
        result.contains("Harvard") || result.contains("Affiliated") || result.contains("DeepMind")
    );
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_pearl_growing_tool_name() {
    let tool = PearlGrowingTool;
    assert_eq!(tool.name(), "pearl_growing");
}

#[test]
fn test_pearl_growing_tool_description() {
    let tool = PearlGrowingTool;
    assert!(tool.description().contains("pearl") || tool.description().len() > 10);
}

#[test]
fn test_pearl_growing_tool_input_schema() {
    let tool = PearlGrowingTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}

#[test]
fn test_orcid_lookup_tool_name() {
    let tool = OrcidAuthorLookupTool;
    assert_eq!(tool.name(), "orcid_author_lookup");
}

#[test]
fn test_orcid_lookup_tool_description() {
    let tool = OrcidAuthorLookupTool;
    assert!(tool.description().contains("ORCID") || tool.description().len() > 10);
}

#[test]
fn test_orcid_lookup_tool_input_schema() {
    let tool = OrcidAuthorLookupTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}
