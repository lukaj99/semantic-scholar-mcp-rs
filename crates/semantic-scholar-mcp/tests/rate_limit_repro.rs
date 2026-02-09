use std::time::{Duration, Instant};
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path};
use semantic_scholar_mcp::{SemanticScholarClient, config::Config};
use tokio::sync::mpsc;

#[tokio::test]
async fn test_concurrent_rate_limiting_bug() {
    let mock_server = MockServer::start().await;

    // Return a simple paper object
    let paper_json = serde_json::json!({
        "paperId": "test",
        "title": "Test Paper"
    });

    Mock::given(method("GET"))
        .and(path("/graph/v1/paper/test"))
        .respond_with(ResponseTemplate::new(200).set_body_json(paper_json))
        .mount(&mock_server)
        .await;

    // Create client with 200ms delay (5 req/s)
    let mut config = Config::new(None, None);
    config.graph_api_url = format!("{}/graph/v1", mock_server.uri());
    config.rate_limit_delay = Duration::from_millis(200);
    
    let client = SemanticScholarClient::new(config).unwrap();

    // Fire 10 concurrent requests
    let num_requests = 10;
    let mut handles = vec![];
    let (tx, mut rx) = mpsc::channel(num_requests);

    let start = Instant::now();
    for i in 0..num_requests {
        let client_clone = client.clone();
        let tx_clone = tx.clone();
        handles.push(tokio::spawn(async move {
            let now = Instant::now();
            let _ = client_clone.get_paper("test", &[]).await;
            tx_clone.send((i, now.elapsed())).await.unwrap();
        }));
    }

    let mut results = vec![];
    for _ in 0..num_requests {
        results.push(rx.recv().await.unwrap());
    }

    let total_duration = start.elapsed();
    println!("Total duration for {} concurrent requests: {:?}", num_requests, total_duration);
    
    // If rate limiting is global and working, 10 requests with 200ms delay should take at least 800ms.
    // (First few might be burst, but subsequent ones must wait).
    assert!(total_duration >= Duration::from_millis(800), "Rate limiting IS NOT global (Still buggy!)");
}
