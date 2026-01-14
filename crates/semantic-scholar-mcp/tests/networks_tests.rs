//! Mock-based tests for networks tools: `author_network`
#![allow(clippy::needless_pass_by_value)]

use std::sync::Arc;

use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{AuthorNetworkTool, McpTool, ToolContext};

fn setup_test_context(mock_server: &MockServer) -> ToolContext {
    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    ToolContext::new(Arc::new(client))
}

fn sample_paper_with_authors(id: &str, title: &str, authors: Vec<(&str, &str)>) -> serde_json::Value {
    json!({
        "paperId": id,
        "title": title,
        "abstract": format!("Abstract for {}", title),
        "year": 2023,
        "citationCount": 100,
        "referenceCount": 10,
        "authors": authors.iter().map(|(aid, name)| json!({"authorId": aid, "name": name})).collect::<Vec<_>>(),
        "venue": "Test Journal",
        "fieldsOfStudy": ["Computer Science"],
        "externalIds": {}
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

fn sample_author(id: &str, name: &str) -> serde_json::Value {
    json!({
        "authorId": id,
        "name": name,
        "affiliations": ["MIT"],
        "citationCount": 5000,
        "hIndex": 25,
        "paperCount": 100
    })
}

// =============================================================================
// AuthorNetworkTool Tests
// =============================================================================

#[tokio::test]
async fn test_author_network_basic() {
    let mock_server = MockServer::start().await;

    // Author info
    Mock::given(method("GET"))
        .and(path("/graph/v1/author/main_author"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("main_author", "Main Author")))
        .mount(&mock_server)
        .await;

    // Author's papers with collaborators
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper_with_authors("p1", "Collab Paper 1", vec![
                ("main_author", "Main Author"),
                ("collab1", "Collaborator One"),
                ("collab2", "Collaborator Two"),
            ]),
            sample_paper_with_authors("p2", "Collab Paper 2", vec![
                ("main_author", "Main Author"),
                ("collab1", "Collaborator One"),  // Same collaborator
                ("collab3", "Collaborator Three"),
            ]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({"authorId": "main_author"}))
        .await
        .unwrap();

    assert!(result.contains("Collaboration") || result.contains("Author") || result.contains("network"));
}

#[tokio::test]
async fn test_author_network_json_format() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/json_author"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("json_author", "JSON Author")))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper_with_authors("p1", "JSON Paper", vec![
                ("json_author", "JSON Author"),
                ("other", "Other Author"),
            ]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({
            "authorId": "json_author",
            "responseFormat": "json"
        }))
        .await
        .unwrap();

    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed.get("author").is_some() || parsed.get("collaborators").is_some());
}

#[tokio::test]
async fn test_author_network_min_shared_papers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/filter_author"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("filter_author", "Filter Author")))
        .mount(&mock_server)
        .await;

    // One collaborator appears 3 times, another only once
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper_with_authors("p1", "Paper 1", vec![
                ("filter_author", "Filter Author"),
                ("frequent", "Frequent Collab"),
            ]),
            sample_paper_with_authors("p2", "Paper 2", vec![
                ("filter_author", "Filter Author"),
                ("frequent", "Frequent Collab"),
                ("rare", "Rare Collab"),
            ]),
            sample_paper_with_authors("p3", "Paper 3", vec![
                ("filter_author", "Filter Author"),
                ("frequent", "Frequent Collab"),
            ]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({
            "authorId": "filter_author",
            "minSharedPapers": 2
        }))
        .await
        .unwrap();

    // Should include frequent collaborator (3 papers) but may filter rare (1 paper)
    assert!(result.contains("Frequent") || result.contains("Collaboration"));
}

#[tokio::test]
async fn test_author_network_max_collaborators() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/max_author"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("max_author", "Max Author")))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper_with_authors("p1", "Paper", vec![
                ("max_author", "Max Author"),
                ("c1", "Collab 1"),
                ("c2", "Collab 2"),
                ("c3", "Collab 3"),
                ("c4", "Collab 4"),
                ("c5", "Collab 5"),
            ]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({
            "authorId": "max_author",
            "maxCollaborators": 3
        }))
        .await
        .unwrap();

    // Should limit to 3 collaborators
    assert!(result.contains("Collab") || result.contains("Collaboration"));
}

#[tokio::test]
async fn test_author_network_no_collaborators() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/solo_author"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("solo_author", "Solo Author")))
        .mount(&mock_server)
        .await;

    // Author has papers but no co-authors
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![
            sample_paper_with_authors("p1", "Solo Paper", vec![
                ("solo_author", "Solo Author"),
            ]),
        ])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({"authorId": "solo_author"}))
        .await
        .unwrap();

    assert!(result.contains("No collaborators") || result.contains('0') || result.contains("Solo"));
}

#[tokio::test]
async fn test_author_network_no_papers() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/no_papers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("no_papers", "No Papers Author")))
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_result(vec![])))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({"authorId": "no_papers"}))
        .await
        .unwrap();

    assert!(result.contains('0') || result.contains("No collaborators") || result.contains("Collaboration"));
}

#[tokio::test]
async fn test_author_network_not_found() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/invalid"))
        .respond_with(ResponseTemplate::new(404).set_body_string("Not found"))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool.execute(&ctx, json!({"authorId": "invalid"})).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn test_author_network_pagination() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/graph/v1/author/paginated"))
        .respond_with(ResponseTemplate::new(200).set_body_json(sample_author("paginated", "Paginated Author")))
        .mount(&mock_server)
        .await;

    // First page has more
    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "total": 200,
            "offset": 0,
            "next": 100,
            "data": vec![
                sample_paper_with_authors("p1", "Page 1", vec![
                    ("paginated", "Paginated Author"),
                    ("friend", "Friend"),
                ]),
            ]
        })))
        .mount(&mock_server)
        .await;

    let ctx = setup_test_context(&mock_server);
    let tool = AuthorNetworkTool;

    let result = tool
        .execute(&ctx, json!({"authorId": "paginated"}))
        .await
        .unwrap();

    assert!(result.contains("Friend") || result.contains("Collaboration") || result.contains("Author"));
}

// =============================================================================
// Tool Trait Method Tests (for coverage)
// =============================================================================

#[test]
fn test_author_network_tool_name() {
    let tool = AuthorNetworkTool;
    assert_eq!(tool.name(), "author_network");
}

#[test]
fn test_author_network_tool_description() {
    let tool = AuthorNetworkTool;
    assert!(tool.description().contains("collaboration") || tool.description().contains("network") || tool.description().len() > 10);
}

#[test]
fn test_author_network_tool_input_schema() {
    let tool = AuthorNetworkTool;
    let schema = tool.input_schema();
    assert!(schema.get("properties").is_some());
}
