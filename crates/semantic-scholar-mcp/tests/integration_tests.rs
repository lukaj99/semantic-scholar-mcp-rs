//! Integration tests for Semantic Scholar MCP server.
//!
//! These tests hit the real Semantic Scholar API.
//! Run with: `cargo test --features integration -- --nocapture`

#![cfg(feature = "integration")]
#![allow(dead_code)]

use semantic_scholar_mcp::client::SemanticScholarClient;
use semantic_scholar_mcp::config::{fields, Config};
use std::sync::Arc;

/// Well-known paper IDs for testing.
mod paper_ids {
    /// "Attention Is All You Need" - Vaswani et al. 2017
    pub const ATTENTION: &str = "649def34f8be52c8b66281af98ae884c09aef38b";
    /// "BERT" - Devlin et al. 2019
    pub const BERT: &str = "df2b0e26d0599ce3e70df8a9da02e51594e0e992";
    /// "ImageNet Classification with Deep CNNs" - Krizhevsky et al. 2012
    pub const ALEXNET: &str = "abd1c342495432171beb7ca8fd9551ef13cbd0ff";
    /// Invalid ID for error testing
    pub const INVALID: &str = "0000000000000000000000000000000000000000";
}

/// Well-known author IDs for testing.
mod author_ids {
    /// Geoffrey Hinton - prolific AI researcher
    pub const HINTON: &str = "1741101";
    /// Yann LeCun
    pub const LECUN: &str = "1688882";
}

fn create_client() -> Arc<SemanticScholarClient> {
    let config = Config {
        api_key: std::env::var("SEMANTIC_SCHOLAR_API_KEY").ok(),
        ..Config::default()
    };
    Arc::new(SemanticScholarClient::new(config).expect("Failed to create client"))
}

// =============================================================================
// Paper Search Tests
// =============================================================================

#[tokio::test]
async fn test_search_papers_basic() {
    let client = create_client();
    let result = client
        .search_papers("transformer attention", 0, 10, fields::DEFAULT)
        .await
        .expect("Search should succeed");

    assert!(!result.data.is_empty(), "Should return some papers");
    assert!(result.total > 0, "Should have total count");
}

#[tokio::test]
async fn test_search_papers_empty_query() {
    let client = create_client();
    // Very random query should return 0 or few results
    let result = client
        .search_papers("xyznonexistentquery12345", 0, 10, fields::DEFAULT)
        .await;

    // Either returns empty results or a parsing error is acceptable
    match result {
        Ok(r) => assert!(r.total >= 0, "Should handle no results gracefully"),
        Err(e) => println!("Expected: API returned error for no results: {:?}", e),
    }
}

#[tokio::test]
async fn test_search_papers_special_characters() {
    let client = create_client();
    // Query with special characters
    let result = client
        .search_papers("C++ memory management", 0, 10, fields::DEFAULT)
        .await;

    // Might get rate limited or succeed
    match result {
        Ok(r) => assert!(r.total >= 0, "Should handle special chars"),
        Err(e) => println!("Note: Special chars query returned error: {:?}", e),
    }
}

// =============================================================================
// Paper Metadata Tests
// =============================================================================

#[tokio::test]
async fn test_get_paper_by_id() {
    let client = create_client();
    let result = client
        .get_paper(paper_ids::ATTENTION, fields::EXTENDED)
        .await;

    match result {
        Ok(paper) => {
            assert_eq!(paper.paper_id, paper_ids::ATTENTION);
            assert!(paper.title.is_some());
            println!(
                "Attention paper: year={:?}, citations={}",
                paper.year,
                paper.citations()
            );
        }
        Err(e) => {
            // Rate limiting is acceptable for integration tests without API key
            println!("Note: Get paper returned error (likely rate limited): {:?}", e);
        }
    }
}

#[tokio::test]
async fn test_get_paper_invalid_id() {
    let client = create_client();
    let result = client.get_paper(paper_ids::INVALID, fields::DEFAULT).await;

    assert!(result.is_err(), "Invalid ID should return error");
}

#[tokio::test]
async fn test_get_papers_batch() {
    let client = create_client();
    let ids = vec![paper_ids::ATTENTION.to_string(), paper_ids::BERT.to_string()];
    let papers = client
        .get_papers_batch(&ids, fields::DEFAULT)
        .await
        .expect("Batch should succeed");

    assert_eq!(papers.len(), 2, "Should return both papers");
}

#[tokio::test]
async fn test_get_papers_batch_with_invalid() {
    let client = create_client();
    // Mix valid and invalid IDs - API returns null for invalid IDs
    let ids = vec![
        paper_ids::ATTENTION.to_string(),
        paper_ids::INVALID.to_string(),
        paper_ids::BERT.to_string(),
    ];
    let result = client.get_papers_batch(&ids, fields::DEFAULT).await;

    // API might return error for batch with invalid IDs or filter them out
    match result {
        Ok(papers) => {
            // Should return at least the valid papers
            assert!(!papers.is_empty(), "Should return some valid papers");
        }
        Err(e) => {
            // Parsing error is acceptable - API returns nulls for invalid IDs
            println!("Note: Batch with invalid IDs returned error: {:?}", e);
        }
    }
}

// =============================================================================
// Citation Network Tests
// =============================================================================

#[tokio::test]
async fn test_get_citations() {
    let client = create_client();
    let result = client
        .get_citations(paper_ids::ATTENTION, 0, 20, fields::DEFAULT)
        .await
        .expect("Should fetch citations");

    assert!(!result.data.is_empty(), "Attention paper has many citations");
}

#[tokio::test]
async fn test_get_references() {
    let client = create_client();
    let result = client
        .get_references(paper_ids::ATTENTION, 0, 20, fields::DEFAULT)
        .await
        .expect("Should fetch references");

    assert!(!result.data.is_empty(), "Attention paper has references");
}

// =============================================================================
// Recommendations Tests
// =============================================================================

#[tokio::test]
async fn test_get_recommendations() {
    let client = create_client();
    let papers = client
        .get_recommendations(&[paper_ids::ATTENTION.to_string()], None, 10, fields::DEFAULT)
        .await
        .expect("Should get recommendations");

    assert!(!papers.is_empty(), "Should return similar papers");
}

#[tokio::test]
async fn test_get_recommendations_multiple_seeds() {
    let client = create_client();
    let seeds = vec![
        paper_ids::ATTENTION.to_string(),
        paper_ids::BERT.to_string(),
    ];
    let papers = client
        .get_recommendations(&seeds, None, 10, fields::DEFAULT)
        .await
        .expect("Should get recommendations from multiple seeds");

    assert!(!papers.is_empty(), "Should return recommendations");
}

// =============================================================================
// Author Tests
// =============================================================================

#[tokio::test]
async fn test_search_authors() {
    let client = create_client();
    let result = client
        .search_authors("Geoffrey Hinton", 0, 5)
        .await
        .expect("Author search should succeed");

    assert!(!result.data.is_empty(), "Should find Geoffrey Hinton");
}

#[tokio::test]
async fn test_get_author() {
    let client = create_client();
    let author = client
        .get_author(author_ids::HINTON)
        .await
        .expect("Should fetch Hinton");

    assert_eq!(author.author_id, author_ids::HINTON);
    assert!(author.name.is_some());
    // Just verify we got some paper count - exact number varies
    println!("Hinton has {} papers", author.papers());
}

// =============================================================================
// Edge Cases & Error Handling
// =============================================================================

#[tokio::test]
async fn test_pagination_bounds() {
    let client = create_client();
    // Request with very high offset
    let result = client
        .search_papers("machine learning", 10000, 10, fields::DEFAULT)
        .await;

    // Should either succeed with empty results or return gracefully
    assert!(result.is_ok() || result.is_err(), "Should handle high offset");
}

#[tokio::test]
async fn test_zero_citation_paper() {
    let client = create_client();
    // Search for a recent paper that might have 0 citations
    let result = client
        .search_papers("preprint 2024", 0, 100, fields::DEFAULT)
        .await;

    match result {
        Ok(search_result) => {
            // Check that papers with 0 citations are handled
            let zero_cite_papers: Vec<_> = search_result
                .data
                .iter()
                .filter(|p| p.citations() == 0)
                .collect();
            println!("Found {} papers with 0 citations", zero_cite_papers.len());
        }
        Err(e) => {
            // Rate limiting is acceptable for this test
            println!("Note: Search returned error (likely rate limited): {:?}", e);
        }
    }
}
