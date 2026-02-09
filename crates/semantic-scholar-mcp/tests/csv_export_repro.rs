use semantic_scholar_mcp::tools::{McpTool, ToolContext, ReferenceExportTool};
use semantic_scholar_mcp::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn test_csv_escaping_bug() {
    let mock_server = MockServer::start().await;

    // Paper with comma in ID and special characters in title
    let batch_json = json!([
        {
            "paperId": "id,with,comma",
            "title": "Title with \"quotes\" and , commas",
            "year": 2023,
            "citationCount": 10,
            "authors": [{"authorId": "a1", "name": "Author, Test"}],
            "venue": "Nature",
            "externalIds": {"DOI": "10.1234/56,78"}
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
    let tool = ReferenceExportTool;

    let input = json!({
        "paperIds": ["id,with,comma"],
        "format": "csv"
    });

    let result = tool.execute(&ctx, input).await.unwrap();
    println!("CSV Output:\n{}", result);

    let lines: Vec<&str> = result.lines().collect();
    assert_eq!(lines.len(), 2);
    
    let data_line = lines[1];
    // Use a more sophisticated split that respects quotes, or just check the number of fields
    // Actually, csv_escape uses quotes for fields with commas.
    // Our id "id,with,comma" should become "\"id,with,comma\""
    // Our doi "10.1234/56,78" should become "\"10.1234/56,78\""
    
    // For simplicity, let's count occurrences of "," that are NOT inside quotes.
    // Or just check that the line starts with a quote.
    assert!(data_line.starts_with('"'), "CSV line should start with a quote for escaped paper_id");
    
    // If we split by "," and it's escaped, we should get exactly 8 parts if we don't count commas inside quotes.
    // But since I don't want to write a full CSV parser here, I'll just check the count of parts 
    // after escaping. It should be 8 if the parser was smart, but simple split(',') will give 
    // more parts, but they will be inside quotes.
    
    // Wait, if I escape correctly, and I use a real CSV parser, I get 8.
    // If I use split(','), I get more, but the parts containing commas will start and end with quotes.
    
    // Let's just check that it contains the expected escaped strings.
    assert!(data_line.contains("\"id,with,comma\""));
    assert!(data_line.contains("\"10.1234/56,78\""));
}
