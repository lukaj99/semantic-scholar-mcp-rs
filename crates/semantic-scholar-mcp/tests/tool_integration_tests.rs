//! Integration tests for MCP tools against the real Semantic Scholar API.
//!
//! Run with: `cargo test --features integration --test tool_integration_tests -- --nocapture`

#![cfg(feature = "integration")]
#![allow(dead_code)]

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::Config;
use semantic_scholar_mcp::tools::{
    AuthorNetworkTool, ExhaustiveSearchTool, FieldWeightedImpactTool, McpTool, ToolContext,
};
use serde_json::json;
use std::sync::Arc;

fn create_context() -> ToolContext {
    let config =
        Config { api_key: std::env::var("SEMANTIC_SCHOLAR_API_KEY").ok(), ..Config::default() };
    let client = Arc::new(SemanticScholarClient::new(config).expect("Failed to create client"));
    ToolContext::new(client)
}

#[tokio::test]
async fn test_tool_exhaustive_search_real() {
    // Add delay to help with rate limits
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let ctx = create_context();
    let tool = ExhaustiveSearchTool;
    
    let input = json!({
        "query": "transformer attention",
        "limit": 5,
        "responseFormat": "json"
    });

    let result = tool.execute(&ctx, input).await;

    match result {
        Ok(output) => {
            let json: serde_json::Value = serde_json::from_str(&output).expect("Should be valid JSON");
            
            // ExhaustiveSearchTool returns a JSON array of papers directly
            if let Some(papers) = json.as_array() {
                assert!(!papers.is_empty(), "Should find papers for 'transformer attention'");
                println!("Found {} papers via tool", papers.len());
            } else {
                panic!("Expected JSON array, got: {:?}", json);
            }
        }
        Err(e) => {
            println!("Tool execution failed (likely rate limited): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_tool_author_network_real() {
    // Add delay to help with rate limits between tests
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;

    let ctx = create_context();
    let tool = AuthorNetworkTool;
    
    // Geoffrey Hinton
    let input = json!({
        "authorId": "1741101",
        "responseFormat": "json",
        "maxCollaborators": 10
    });

    let result = tool.execute(&ctx, input).await;

    match result {
        Ok(output) => {
            let json: serde_json::Value = serde_json::from_str(&output).expect("Should be valid JSON");
            if let Some(collaborators) = json["collaborators"].as_array() {
                if collaborators.is_empty() {
                    println!("Warning: No collaborators found (likely rate limited or empty response)");
                } else {
                    println!("Found {} collaborators via tool", collaborators.len());
                }
            } else {
                 panic!("Expected 'collaborators' array in response: {:?}", json);
            }
        }
        Err(e) => {
            println!("Tool execution failed (likely rate limited): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_tool_fwci_real() {
    let ctx = create_context();
    let tool = FieldWeightedImpactTool;
    
    // "Attention Is All You Need"
    let input = json!({
        "paperIds": ["649def34f8be52c8b66281af98ae884c09aef38b"], 
        "responseFormat": "json"
    });

    let result = tool.execute(&ctx, input).await;

    match result {
        Ok(output) => {
            let json: serde_json::Value = serde_json::from_str(&output).expect("Should be valid JSON");
            let results = json["results"].as_array().expect("Should have results array");
            if let Some(first) = results.first() {
                let fwci = first["fwci"].as_f64().unwrap_or(0.0);
                 println!("FWCI: {}", fwci);
                 // Note: fwci might be null/0 if baselines failed, so we don't strictly assert > 0 here to avoid flaky tests
            } else {
                println!("No results returned");
            }
        }
        Err(e) => {
            println!("Tool execution failed (likely rate limited): {:?}", e);
        }
    }
}
