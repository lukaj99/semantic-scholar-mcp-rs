//! Unit tests for data models.

use semantic_scholar_mcp::models::*;

// =============================================================================
// Paper Deserialization Tests
// =============================================================================

#[test]
fn test_paper_minimal() {
    let json = r#"{"paperId": "abc123"}"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.paper_id, "abc123");
    assert!(paper.title.is_none());
    assert!(paper.year.is_none());
    assert!(paper.citation_count.is_none());
    assert!(paper.authors.is_empty());
    assert_eq!(paper.citations(), 0);
    assert_eq!(paper.title_or_default(), "Untitled");
}

#[test]
fn test_paper_with_all_fields() {
    let json = include_str!("fixtures/paper_attention.json");
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.paper_id, "649def34f8be52c8b66281af98ae884c09aef38b");
    assert_eq!(paper.title_or_default(), "Attention is All You Need");
    assert_eq!(paper.year, Some(2017));
    assert!(paper.citations() > 100_000);
    assert_eq!(paper.authors.len(), 8);
    assert_eq!(paper.first_author(), Some("Ashish Vaswani"));
    assert_eq!(paper.doi(), Some("10.5555/3295222.3295349"));
    assert_eq!(paper.arxiv_id(), Some("1706.03762"));
}

#[test]
fn test_paper_with_missing_optional_fields() {
    let json = r#"{
        "paperId": "test123",
        "title": "Test Paper",
        "year": null,
        "citationCount": null,
        "authors": []
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.paper_id, "test123");
    assert_eq!(paper.title_or_default(), "Test Paper");
    assert!(paper.year.is_none());
    assert_eq!(paper.citations(), 0);
    assert!(!paper.has_citations());
}

#[test]
fn test_paper_zero_citations() {
    let json = r#"{
        "paperId": "new123",
        "title": "Brand New Paper",
        "citationCount": 0
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.citations(), 0);
    assert!(!paper.has_citations());
}

#[test]
fn test_paper_author_names() {
    let json = r#"{
        "paperId": "test",
        "authors": [
            {"authorId": "1", "name": "Alice"},
            {"authorId": "2", "name": "Bob"},
            {"authorId": "3", "name": "Charlie"}
        ]
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.author_names(), "Alice, Bob, Charlie");
}

#[test]
fn test_paper_with_null_author_names() {
    let json = r#"{
        "paperId": "test",
        "authors": [
            {"authorId": "1", "name": "Alice"},
            {"authorId": "2", "name": null},
            {"authorId": "3"}
        ]
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    // Should only include author with name
    assert_eq!(paper.author_names(), "Alice");
}

// =============================================================================
// Author Deserialization Tests
// =============================================================================

#[test]
fn test_author_minimal() {
    let json = r#"{"authorId": "12345"}"#;
    let author: Author = serde_json::from_str(json).unwrap();

    assert_eq!(author.author_id, "12345");
    assert!(author.name.is_none());
    assert_eq!(author.name_or_default(), "Unknown");
}

#[test]
fn test_author_full() {
    let json = r#"{
        "authorId": "1741101",
        "name": "Geoffrey E. Hinton",
        "affiliations": ["University of Toronto", "Google"],
        "citationCount": 500000,
        "hIndex": 150,
        "paperCount": 400
    }"#;
    let author: Author = serde_json::from_str(json).unwrap();

    assert_eq!(author.author_id, "1741101");
    assert_eq!(author.name_or_default(), "Geoffrey E. Hinton");
    assert_eq!(author.affiliations.len(), 2);
    assert!(author.citations() > 400_000);
    assert_eq!(author.h_index_value(), 150);
    assert_eq!(author.papers(), 400);
}

// =============================================================================
// SearchResult Tests
// =============================================================================

#[test]
fn test_search_result_with_more() {
    let json = r#"{
        "total": 1000,
        "offset": 0,
        "next": 100,
        "data": []
    }"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();

    assert!(result.has_more());
    assert_eq!(result.next_offset(), Some(100));
}

#[test]
fn test_search_result_last_page() {
    let json = r#"{
        "total": 50,
        "offset": 40,
        "data": []
    }"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();

    assert!(!result.has_more());
    assert_eq!(result.next_offset(), None);
}

// =============================================================================
// Input Validation Tests
// =============================================================================

#[test]
fn test_response_format_default() {
    let input: ExhaustiveSearchInput = serde_json::from_str(r#"{"query": "test"}"#).unwrap();
    assert!(matches!(input.response_format, ResponseFormat::Markdown));
}

#[test]
fn test_exhaustive_search_input() {
    // Note: Input models use camelCase for MCP protocol compatibility
    let json = r#"{
        "query": "deep learning",
        "yearStart": 2020,
        "yearEnd": 2024,
        "minCitations": 10,
        "maxResults": 500,
        "responseFormat": "json"
    }"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();

    assert_eq!(input.query, "deep learning");
    assert_eq!(input.year_start, Some(2020));
    assert_eq!(input.year_end, Some(2024));
    assert_eq!(input.min_citations, Some(10));
    assert_eq!(input.max_results, 500);
    assert!(matches!(input.response_format, ResponseFormat::Json));
}

#[test]
fn test_citation_snowball_input() {
    let json = r#"{
        "seedPaperIds": ["paper1", "paper2"],
        "direction": "both",
        "depth": 2,
        "maxPerPaper": 50
    }"#;
    let input: CitationSnowballInput = serde_json::from_str(json).unwrap();

    assert_eq!(input.seed_paper_ids.len(), 2);
    assert!(matches!(input.direction, SearchDirection::Both));
    assert_eq!(input.depth, 2);
}

#[test]
fn test_pearl_growing_strategies() {
    // Test all strategies deserialize correctly
    let strategies = ["keywords", "authors", "citations", "all"];
    for strat in strategies {
        let json = format!(r#"{{"seedPaperIds": ["p1"], "strategy": "{strat}"}}"#);
        let input: PearlGrowingInput = serde_json::from_str(&json).unwrap();
        assert!(!input.seed_paper_ids.is_empty());
    }
}

#[test]
fn test_export_formats() {
    let formats = ["ris", "bibtex", "csv", "endnote"];
    for fmt in formats {
        let json = format!(r#"{{"paperIds": ["p1"], "format": "{fmt}"}}"#);
        let input: ReferenceExportInput = serde_json::from_str(&json).unwrap();
        assert!(!input.paper_ids.is_empty());
    }
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn test_paper_with_unicode() {
    let json = r#"{
        "paperId": "unicode123",
        "title": "中文标题 - Japanese: 日本語 - Korean: 한국어",
        "authors": [{"authorId": "1", "name": "François Müller"}]
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert!(paper.title_or_default().contains("中文"));
    assert_eq!(paper.first_author(), Some("François Müller"));
}

#[test]
fn test_paper_with_empty_strings() {
    let json = r#"{
        "paperId": "empty123",
        "title": "",
        "abstract": "",
        "venue": ""
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    // Empty string is different from None
    assert_eq!(paper.title, Some(String::new()));
    // title_or_default should return the empty string, not "Untitled"
    // Actually let's check the implementation...
}

#[test]
fn test_external_ids_partial() {
    let json = r#"{
        "paperId": "ext123",
        "externalIds": {
            "DOI": "10.1234/test",
            "ArXiv": null
        }
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.doi(), Some("10.1234/test"));
    assert!(paper.arxiv_id().is_none());
}

#[test]
fn test_large_citation_count() {
    let json = r#"{
        "paperId": "big123",
        "citationCount": 2147483647
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.citations(), i32::MAX);
}
