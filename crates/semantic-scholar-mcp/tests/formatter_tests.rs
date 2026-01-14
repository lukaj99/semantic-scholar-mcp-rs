//! Comprehensive formatter tests for markdown and JSON output.
//!
//! Tests output correctness, edge cases, and special character handling.

use semantic_scholar_mcp::formatters;
use semantic_scholar_mcp::models::{Author, AuthorRef, ExternalIds, OpenAccessPdf, Paper, Tldr};
use serde_json::json;

// =============================================================================
// Helper Functions
// =============================================================================

fn make_paper(id: &str, title: &str, year: i32, citations: i32) -> Paper {
    Paper {
        paper_id: id.to_string(),
        title: Some(title.to_string()),
        year: Some(year),
        citation_count: Some(citations),
        ..Default::default()
    }
}

fn make_author(id: &str, name: &str, h_index: i32, citations: i32) -> Author {
    Author {
        author_id: id.to_string(),
        name: Some(name.to_string()),
        h_index: Some(h_index),
        citation_count: Some(citations),
        paper_count: Some(50),
        affiliations: vec![],
        homepage: None,
        external_ids: None,
    }
}

// =============================================================================
// Markdown Paper Formatting Tests
// =============================================================================

#[test]
fn test_format_paper_markdown_basic() {
    let paper = make_paper("abc123", "Test Paper Title", 2024, 100);
    let output = formatters::format_paper_markdown(&paper, 1);

    assert!(output.contains("## 1. Test Paper Title"));
    assert!(output.contains("**Year**: 2024"));
    assert!(output.contains("**Citations**: 100"));
}

#[test]
fn test_format_paper_markdown_with_authors() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.authors = vec![
        AuthorRef { author_id: Some("a1".into()), name: Some("John Doe".into()) },
        AuthorRef { author_id: Some("a2".into()), name: Some("Jane Smith".into()) },
    ];

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("**Authors**: John Doe, Jane Smith"));
}

#[test]
fn test_format_paper_markdown_many_authors() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.authors = (0..20)
        .map(|i| AuthorRef { author_id: Some(format!("a{i}")), name: Some(format!("Author {i}")) })
        .collect();

    let output = formatters::format_paper_markdown(&paper, 1);
    // Should include author names
    assert!(output.contains("Author"));
}

#[test]
fn test_format_paper_markdown_no_authors() {
    let paper = make_paper("abc123", "Test", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 1);

    // May or may not have authors line depending on implementation
    // Just ensure it doesn't crash
    assert!(output.contains("Test"));
}

#[test]
fn test_format_paper_markdown_with_venue() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.venue = Some("Nature".to_string());

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("**Venue**: Nature"));
}

#[test]
fn test_format_paper_markdown_with_fields_of_study() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.fields_of_study = Some(vec!["Computer Science".into(), "AI".into()]);

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("**Fields**: Computer Science, AI"));
}

#[test]
fn test_format_paper_markdown_empty_fields_of_study() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.fields_of_study = Some(vec![]);

    let output = formatters::format_paper_markdown(&paper, 1);
    // Should not include empty fields line - or handle gracefully
    assert!(output.contains("Test"));
}

#[test]
fn test_format_paper_markdown_with_doi() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.external_ids =
        Some(ExternalIds { doi: Some("10.1234/test".to_string()), ..Default::default() });

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("[DOI](https://doi.org/10.1234/test)"));
}

#[test]
fn test_format_paper_markdown_with_arxiv() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.external_ids =
        Some(ExternalIds { arxiv: Some("2401.12345".to_string()), ..Default::default() });

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("[arXiv](https://arxiv.org/abs/2401.12345)"));
}

#[test]
fn test_format_paper_markdown_s2_link() {
    let paper = make_paper("abc123", "Test", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 1);

    assert!(output.contains("[S2](https://www.semanticscholar.org/paper/abc123)"));
}

#[test]
fn test_format_paper_markdown_with_pdf() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.open_access_pdf = Some(OpenAccessPdf {
        url: Some("https://example.com/paper.pdf".to_string()),
        status: None,
    });

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("[Open Access](https://example.com/paper.pdf)"));
}

#[test]
fn test_format_paper_markdown_with_tldr() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.tldr = Some(Tldr { text: Some("This is a short summary.".to_string()), model: None });

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("> **TLDR**: This is a short summary."));
}

#[test]
fn test_format_paper_markdown_with_abstract() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.r#abstract = Some("This is the abstract text.".to_string());

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("**Abstract**: This is the abstract text."));
}

#[test]
fn test_format_paper_markdown_long_abstract_truncated() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.r#abstract = Some("x".repeat(500));

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("..."));
    // Abstract should be truncated at 300 chars + "..."
}

#[test]
fn test_format_paper_markdown_no_year() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.year = None;

    let output = formatters::format_paper_markdown(&paper, 1);
    // Should not have "None" in output
    assert!(!output.contains("**Year**: None"));
}

#[test]
fn test_format_paper_markdown_zero_citations() {
    let paper = make_paper("abc123", "Test", 2024, 0);
    let output = formatters::format_paper_markdown(&paper, 1);

    assert!(output.contains("**Citations**: 0"));
}

#[test]
fn test_format_paper_markdown_missing_title() {
    let paper = Paper { paper_id: "abc123".to_string(), title: None, ..Default::default() };

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("Untitled"));
}

#[test]
fn test_format_paper_markdown_unicode_title() {
    let paper = make_paper("abc123", "机器学习论文: 深度神经网络", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 1);

    assert!(output.contains("机器学习论文"));
}

#[test]
fn test_format_paper_markdown_special_chars_in_title() {
    let paper = make_paper("abc123", "C++ & Python: A Comparison", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 1);

    assert!(output.contains("C++ & Python"));
}

#[test]
fn test_format_paper_markdown_newlines_in_abstract() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.r#abstract = Some("Line 1\nLine 2\nLine 3".to_string());

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("Line 1"));
}

#[test]
fn test_format_paper_markdown_index_zero() {
    let paper = make_paper("abc123", "Test", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 0);

    assert!(output.contains("## 0. Test"));
}

#[test]
fn test_format_paper_markdown_large_index() {
    let paper = make_paper("abc123", "Test", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 9999);

    assert!(output.contains("## 9999. Test"));
}

// =============================================================================
// Markdown Papers List Tests
// =============================================================================

#[test]
fn test_format_papers_markdown_empty() {
    let papers: Vec<Paper> = vec![];
    let output = formatters::format_papers_markdown(&papers);

    assert!(output.contains("No papers found"));
}

#[test]
fn test_format_papers_markdown_single() {
    let papers = vec![make_paper("abc123", "Test Paper", 2024, 100)];
    let output = formatters::format_papers_markdown(&papers);

    assert!(output.contains("# Papers (1 results)"));
    assert!(output.contains("## 1. Test Paper"));
}

#[test]
fn test_format_papers_markdown_multiple() {
    let papers = vec![
        make_paper("abc1", "Paper One", 2024, 100),
        make_paper("abc2", "Paper Two", 2023, 50),
        make_paper("abc3", "Paper Three", 2022, 25),
    ];
    let output = formatters::format_papers_markdown(&papers);

    assert!(output.contains("# Papers (3 results)"));
    assert!(output.contains("## 1. Paper One"));
    assert!(output.contains("## 2. Paper Two"));
    assert!(output.contains("## 3. Paper Three"));
}

#[test]
fn test_format_papers_markdown_separators() {
    let papers =
        vec![make_paper("abc1", "Paper One", 2024, 100), make_paper("abc2", "Paper Two", 2023, 50)];
    let output = formatters::format_papers_markdown(&papers);

    // Should have separators between papers
    assert!(output.contains("---"));
}

// =============================================================================
// Markdown Author Formatting Tests
// =============================================================================

#[test]
fn test_format_author_markdown_basic() {
    let author = make_author("auth123", "John Doe", 25, 5000);
    let output = formatters::format_author_markdown(&author);

    assert!(output.contains("John Doe"));
    assert!(output.contains("**h-index**: 25"));
    assert!(output.contains("**Citations**: 5000"));
}

#[test]
fn test_format_author_markdown_with_affiliations() {
    let mut author = make_author("auth123", "John Doe", 25, 5000);
    author.affiliations = vec!["MIT".into(), "Stanford".into()];

    let output = formatters::format_author_markdown(&author);
    assert!(output.contains("**Affiliations**: MIT, Stanford"));
}

#[test]
fn test_format_author_markdown_s2_profile() {
    let author = make_author("auth123", "John Doe", 25, 5000);
    let output = formatters::format_author_markdown(&author);

    assert!(output.contains("https://www.semanticscholar.org/author/auth123"));
}

#[test]
fn test_format_author_markdown_missing_name() {
    let author = Author {
        author_id: "auth123".to_string(),
        name: None,
        h_index: Some(25),
        citation_count: Some(5000),
        paper_count: Some(50),
        affiliations: vec![],
        homepage: None,
        external_ids: None,
    };

    let output = formatters::format_author_markdown(&author);
    assert!(output.contains("Unknown"));
}

#[test]
fn test_format_author_markdown_zero_metrics() {
    let author = make_author("auth123", "New Author", 0, 0);
    let output = formatters::format_author_markdown(&author);

    assert!(output.contains("**h-index**: 0"));
    assert!(output.contains("**Citations**: 0"));
}

#[test]
fn test_format_author_markdown_with_homepage() {
    let mut author = make_author("auth123", "John Doe", 25, 5000);
    author.homepage = Some("https://johndoe.com".to_string());

    let output = formatters::format_author_markdown(&author);
    assert!(output.contains("https://johndoe.com"));
}

// =============================================================================
// Markdown Authors List Tests
// =============================================================================

#[test]
fn test_format_authors_markdown_empty() {
    let authors: Vec<Author> = vec![];
    let output = formatters::format_authors_markdown(&authors);

    assert!(output.contains("No authors found"));
}

#[test]
fn test_format_authors_markdown_multiple() {
    let authors = vec![make_author("a1", "Alice", 30, 10000), make_author("a2", "Bob", 20, 5000)];
    let output = formatters::format_authors_markdown(&authors);

    assert!(output.contains("# Authors (2 results)"));
    assert!(output.contains("Alice"));
    assert!(output.contains("Bob"));
}

// =============================================================================
// JSON Compact Paper Tests
// =============================================================================

#[test]
fn test_compact_paper_basic() {
    let paper = make_paper("abc123", "Test Paper", 2024, 100);
    let compact = formatters::compact_paper(&paper);

    assert_eq!(compact["id"], "abc123");
    assert_eq!(compact["title"], "Test Paper");
    assert_eq!(compact["year"], 2024);
    assert_eq!(compact["citations"], 100);
}

#[test]
fn test_compact_paper_with_authors() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.authors = vec![
        AuthorRef { author_id: Some("a1".into()), name: Some("John".into()) },
        AuthorRef { author_id: Some("a2".into()), name: Some("Jane".into()) },
    ];

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["authors"], json!(["John", "Jane"]));
}

#[test]
fn test_compact_paper_author_without_name() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.authors = vec![
        AuthorRef { author_id: Some("a1".into()), name: None },
        AuthorRef { author_id: Some("a2".into()), name: Some("Jane".into()) },
    ];

    let compact = formatters::compact_paper(&paper);
    // Only named authors included
    assert_eq!(compact["authors"], json!(["Jane"]));
}

#[test]
fn test_compact_paper_with_venue() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.venue = Some("Nature".to_string());

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["venue"], "Nature");
}

#[test]
fn test_compact_paper_without_venue() {
    let paper = make_paper("abc123", "Test", 2024, 10);
    let compact = formatters::compact_paper(&paper);

    assert!(compact.get("venue").is_none());
}

#[test]
fn test_compact_paper_with_doi() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.external_ids =
        Some(ExternalIds { doi: Some("10.1234/test".to_string()), ..Default::default() });

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["doi"], "10.1234/test");
}

#[test]
fn test_compact_paper_with_arxiv() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.external_ids =
        Some(ExternalIds { arxiv: Some("2401.12345".to_string()), ..Default::default() });

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["arxiv"], "2401.12345");
}

#[test]
fn test_compact_paper_with_pdf() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.open_access_pdf = Some(OpenAccessPdf {
        url: Some("https://example.com/paper.pdf".to_string()),
        status: None,
    });

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["pdf"], "https://example.com/paper.pdf");
}

#[test]
fn test_compact_paper_with_tldr() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.tldr = Some(Tldr { text: Some("Short summary".to_string()), model: None });

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["tldr"], "Short summary");
}

#[test]
fn test_compact_paper_with_fields() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.fields_of_study = Some(vec!["CS".into(), "AI".into()]);

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["fields"], json!(["CS", "AI"]));
}

#[test]
fn test_compact_paper_empty_fields() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.fields_of_study = Some(vec![]);

    let compact = formatters::compact_paper(&paper);
    assert!(compact.get("fields").is_none());
}

#[test]
fn test_compact_paper_null_year() {
    let paper = Paper {
        paper_id: "abc123".to_string(),
        title: Some("Test".to_string()),
        year: None,
        citation_count: Some(10),
        ..Default::default()
    };

    let compact = formatters::compact_paper(&paper);
    assert!(compact["year"].is_null());
}

#[test]
fn test_compact_paper_missing_title() {
    let paper = Paper { paper_id: "abc123".to_string(), title: None, ..Default::default() };

    let compact = formatters::compact_paper(&paper);
    assert_eq!(compact["title"], "Untitled");
}

// =============================================================================
// JSON Compact Author Tests
// =============================================================================

#[test]
fn test_compact_author_basic() {
    let author = make_author("auth123", "John Doe", 25, 5000);
    let compact = formatters::compact_author(&author);

    assert_eq!(compact["id"], "auth123");
    assert_eq!(compact["name"], "John Doe");
    assert_eq!(compact["hIndex"], 25);
    assert_eq!(compact["citations"], 5000);
}

#[test]
fn test_compact_author_with_affiliations() {
    let mut author = make_author("auth123", "John Doe", 25, 5000);
    author.affiliations = vec!["MIT".into(), "Stanford".into()];

    let compact = formatters::compact_author(&author);
    assert_eq!(compact["affiliations"], json!(["MIT", "Stanford"]));
}

#[test]
fn test_compact_author_without_affiliations() {
    let author = make_author("auth123", "John Doe", 25, 5000);
    let compact = formatters::compact_author(&author);

    assert!(compact.get("affiliations").is_none());
}

#[test]
fn test_compact_author_with_homepage() {
    let mut author = make_author("auth123", "John Doe", 25, 5000);
    author.homepage = Some("https://johndoe.com".to_string());

    let compact = formatters::compact_author(&author);
    assert_eq!(compact["homepage"], "https://johndoe.com");
}

#[test]
fn test_compact_author_missing_name() {
    let author = Author {
        author_id: "auth123".to_string(),
        name: None,
        h_index: Some(25),
        citation_count: Some(5000),
        paper_count: Some(50),
        affiliations: vec![],
        homepage: None,
        external_ids: None,
    };

    let compact = formatters::compact_author(&author);
    assert_eq!(compact["name"], "Unknown");
}

// =============================================================================
// Edge Cases and Special Characters
// =============================================================================

#[test]
fn test_paper_with_html_in_title() {
    let paper = make_paper("abc123", "Test <script>alert('xss')</script> Paper", 2024, 10);
    let output = formatters::format_paper_markdown(&paper, 1);

    // Output should contain the title as-is (markdown doesn't execute scripts)
    assert!(output.contains("<script>"));
}

#[test]
fn test_paper_with_markdown_in_abstract() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.r#abstract = Some("This is **bold** and _italic_ text.".to_string());

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("**bold**"));
}

#[test]
fn test_paper_with_url_in_abstract() {
    let mut paper = make_paper("abc123", "Test", 2024, 10);
    paper.r#abstract = Some("See https://example.com for more info.".to_string());

    let output = formatters::format_paper_markdown(&paper, 1);
    assert!(output.contains("https://example.com"));
}

#[test]
fn test_author_with_unicode_name() {
    let author = make_author("auth123", "张三 (Zhang San)", 25, 5000);
    let output = formatters::format_author_markdown(&author);

    assert!(output.contains("张三"));
}

#[test]
fn test_compact_paper_serialization() {
    let paper = make_paper("abc123", "Test", 2024, 10);
    let compact = formatters::compact_paper(&paper);

    // Should be valid JSON
    let json_str = serde_json::to_string(&compact).unwrap();
    assert!(json_str.contains("abc123"));
}

#[test]
fn test_compact_author_serialization() {
    let author = make_author("auth123", "John", 25, 5000);
    let compact = formatters::compact_author(&author);

    // Should be valid JSON
    let json_str = serde_json::to_string(&compact).unwrap();
    assert!(json_str.contains("John"));
}
