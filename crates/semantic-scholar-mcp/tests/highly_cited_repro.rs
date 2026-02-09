use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;
use semantic_scholar_mcp::tools::{McpTool, ToolContext, HighlyCitedPapersTool};
use semantic_scholar_mcp::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use std::sync::Arc;

#[tokio::test]
async fn test_highly_cited_threshold_bug() {
    let mock_server = MockServer::start().await;

    // Mock search for threshold (page 1)
    let search_json_1 = json!({
        "total": 1000,
        "token": "page2",
        "data": (0..100).map(|i| json!({
            "paperId": format!("p{}", i),
            "citationCount": 1000 - i
        })).collect::<Vec<_>>()
    });

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search/bulk"))
        // Match first call (no token)
        .and(wiremock::matchers::query_param_is_missing("token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_json_1))
        .mount(&mock_server)
        .await;

    // Mock search for threshold (page 2)
    let search_json_2 = json!({
        "total": 1000,
        "data": (100..200).map(|i| json!({
            "paperId": format!("p{}", i),
            "citationCount": 1000 - i
        })).collect::<Vec<_>>()
    });

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/search/bulk"))
        .and(wiremock::matchers::query_param("token", "page2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(search_json_2))
        .mount(&mock_server)
        .await;

    // Mock batch metadata
    let batch_json = json!([
        {
            "paperId": "target",
            "title": "Target Paper",
            "year": 2023,
            "citationCount": 850,
            "fieldsOfStudy": ["Computer Science"]
        }
    ]);

    Mock::given(method("POST"))
        .and(path("/graph/v1/paper/batch"))
        .respond_with(ResponseTemplate::new(200).set_body_json(batch_json))
        .mount(&mock_server)
        .await;

    let config = Config::for_testing(&mock_server.uri());
    let client = SemanticScholarClient::new(config).unwrap();
    let ctx = ToolContext::new(Arc::new(client));
    let tool = HighlyCitedPapersTool;

    let input = json!({
        "paperIds": ["target"],
        "percentileThreshold": 15.0, // Top 15% of 1000 = rank 150 (page 2)
        "responseFormat": "json"
    });

    let result = tool.execute(&ctx, input).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    
    let paper_result = &parsed["results"][0];
    // Threshold should be cites[149] = 1000 - 149 = 851.
    assert!(paper_result["threshold"].as_i64().unwrap() > 0, "Threshold should be determined via pagination");
    assert_eq!(paper_result["threshold"], 851);
    assert_eq!(paper_result["is_highly_cited"], false); // 850 < 851
}
