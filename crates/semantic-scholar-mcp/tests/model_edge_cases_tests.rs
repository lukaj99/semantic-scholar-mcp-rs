//! Comprehensive model edge case tests.
//!
//! Tests serialization, deserialization, and model behavior edge cases.

use semantic_scholar_mcp::models::*;
use serde_json::json;

// =============================================================================
// Paper Model Edge Cases
// =============================================================================

#[test]
fn test_paper_minimal_json() {
    let json = r#"{"paperId": "abc123"}"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.paper_id, "abc123");
    assert!(paper.title.is_none());
    assert!(paper.authors.is_empty());
    assert_eq!(paper.citations(), 0);
}

#[test]
fn test_paper_all_fields_json() {
    let json = r#"{
        "paperId": "abc123",
        "title": "Test Paper",
        "abstract": "This is an abstract.",
        "year": 2024,
        "citationCount": 100,
        "referenceCount": 50,
        "fieldsOfStudy": ["Computer Science", "Medicine"],
        "authors": [
            {"authorId": "a1", "name": "John Doe"},
            {"authorId": "a2", "name": "Jane Smith"}
        ],
        "venue": "Nature",
        "publicationDate": "2024-01-15",
        "openAccessPdf": {
            "url": "https://example.com/paper.pdf",
            "status": "BRONZE"
        },
        "externalIds": {
            "DOI": "10.1234/test",
            "ArXiv": "2401.12345",
            "PubMed": "12345678"
        },
        "tldr": {
            "text": "Summary text",
            "model": "tldr-model"
        },
        "influentialCitationCount": 10,
        "isOpenAccess": true,
        "corpusId": 987654321
    }"#;

    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.title_or_default(), "Test Paper");
    assert_eq!(paper.citations(), 100);
    assert_eq!(paper.authors.len(), 2);
    assert_eq!(paper.doi(), Some("10.1234/test"));
    assert_eq!(paper.arxiv_id(), Some("2401.12345"));
    assert_eq!(paper.pdf_url(), Some("https://example.com/paper.pdf"));
    assert_eq!(paper.tldr_text(), Some("Summary text"));
}

#[test]
fn test_paper_null_values() {
    let json = r#"{
        "paperId": "abc",
        "title": null,
        "year": null,
        "citationCount": null
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.title_or_default(), "Untitled");
    assert!(paper.year.is_none());
    assert_eq!(paper.citations(), 0);
}

#[test]
fn test_paper_empty_strings() {
    let json = r#"{
        "paperId": "",
        "title": "",
        "venue": ""
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.paper_id, "");
    assert_eq!(paper.title_or_default(), ""); // empty string, not Untitled
    assert!(paper.venue.as_ref().is_some_and(std::string::String::is_empty));
}

#[test]
fn test_paper_extreme_citation_count() {
    let json = r#"{"paperId": "p1", "citationCount": 2147483647}"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.citations(), i32::MAX);
}

#[test]
fn test_paper_negative_citation_count() {
    let json = r#"{"paperId": "p1", "citationCount": -100}"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.citations(), -100);
}

#[test]
fn test_paper_unicode_content() {
    let json = r#"{
        "paperId": "unicode",
        "title": "机器学习与深度神经网络",
        "abstract": "Résumé en français avec émojis 🎉"
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert!(paper.title_or_default().contains("机器学习"));
    assert!(paper.r#abstract.as_ref().unwrap().contains("émojis"));
}

#[test]
fn test_paper_long_abstract() {
    let long_text = "x".repeat(10000);
    let json = json!({"paperId": "p1", "abstract": long_text});
    let paper: Paper = serde_json::from_str(&json.to_string()).unwrap();
    assert_eq!(paper.r#abstract.as_ref().unwrap().len(), 10000);
}

#[test]
fn test_paper_author_names() {
    let json = r#"{
        "paperId": "p1",
        "authors": [
            {"authorId": "a1", "name": "John"},
            {"authorId": "a2", "name": "Jane"},
            {"authorId": "a3", "name": "Bob"}
        ]
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert_eq!(paper.author_names(), "John, Jane, Bob");
    assert_eq!(paper.first_author(), Some("John"));
}

#[test]
fn test_paper_authors_without_names() {
    let json = r#"{
        "paperId": "p1",
        "authors": [
            {"authorId": "a1"},
            {"authorId": "a2", "name": "Jane"},
            {"authorId": "a3", "name": null}
        ]
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    // Only Jane has a name
    assert_eq!(paper.author_names(), "Jane");
}

#[test]
fn test_paper_first_author_none() {
    let paper = Paper {
        paper_id: "p1".to_string(),
        authors: vec![],
        ..Default::default()
    };
    assert_eq!(paper.first_author(), None);
}

#[test]
fn test_paper_has_citations() {
    let paper_with = Paper {
        paper_id: "p1".to_string(),
        citation_count: Some(10),
        ..Default::default()
    };
    let paper_zero = Paper {
        paper_id: "p2".to_string(),
        citation_count: Some(0),
        ..Default::default()
    };
    let paper_none = Paper {
        paper_id: "p3".to_string(),
        citation_count: None,
        ..Default::default()
    };

    assert!(paper_with.has_citations());
    assert!(!paper_zero.has_citations());
    assert!(!paper_none.has_citations());
}

#[test]
fn test_paper_default() {
    let paper = Paper::default();
    assert!(paper.paper_id.is_empty());
    assert!(paper.title.is_none());
    assert!(paper.authors.is_empty());
    assert_eq!(paper.citations(), 0);
}

#[test]
fn test_paper_external_ids_partial() {
    let json = r#"{
        "paperId": "p1",
        "externalIds": {
            "DOI": "10.1234/test"
        }
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert!(paper.doi().is_some());
    assert!(paper.arxiv_id().is_none());
}

#[test]
fn test_paper_external_ids_all_types() {
    let json = r#"{
        "paperId": "p1",
        "externalIds": {
            "DOI": "10.1234/test",
            "ArXiv": "2401.12345",
            "PubMed": "12345678",
            "PubMedCentral": "PMC123456",
            "MAG": "987654",
            "CorpusId": 123456789,
            "DBLP": "journals/corr/abs-2401-12345",
            "ACL": "2024.acl-main.1"
        }
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    let ids = paper.external_ids.as_ref().unwrap();
    assert!(ids.doi.is_some());
    assert!(ids.arxiv.is_some());
    assert!(ids.pubmed.is_some());
    assert!(ids.pmc.is_some());
    assert!(ids.mag.is_some());
    assert!(ids.dblp.is_some());
    assert!(ids.acl.is_some());
}

// =============================================================================
// SearchResult Edge Cases
// =============================================================================

#[test]
fn test_search_result_empty() {
    let json = r#"{"total": 0, "offset": 0, "data": []}"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.total, 0);
    assert!(result.data.is_empty());
    assert!(!result.has_more());
}

#[test]
fn test_search_result_with_next() {
    let json = r#"{"total": 100, "offset": 0, "next": 10, "data": []}"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();
    assert!(result.has_more());
    assert_eq!(result.next_offset(), Some(10));
}

#[test]
fn test_search_result_last_page() {
    let json = r#"{"total": 100, "offset": 90, "data": []}"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();
    assert!(!result.has_more());
    assert_eq!(result.next_offset(), None);
}

#[test]
fn test_search_result_error_message() {
    let json = r#"{"message": "Invalid query"}"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.message, Some("Invalid query".to_string()));
    assert!(result.data.is_empty());
}

#[test]
fn test_search_result_default() {
    let result = SearchResult::default();
    assert_eq!(result.total, 0);
    assert_eq!(result.offset, 0);
    assert!(result.data.is_empty());
}

// =============================================================================
// Author Model Edge Cases
// =============================================================================

#[test]
fn test_author_minimal() {
    let json = r#"{"authorId": "123"}"#;
    let author: Author = serde_json::from_str(json).unwrap();
    assert_eq!(author.author_id, "123");
    assert_eq!(author.name_or_default(), "Unknown");
    assert_eq!(author.h_index_value(), 0);
    assert_eq!(author.citations(), 0);
}

#[test]
fn test_author_full() {
    let json = r#"{
        "authorId": "123",
        "name": "John Doe",
        "affiliations": ["MIT", "Stanford"],
        "homepage": "https://johndoe.com",
        "paperCount": 50,
        "citationCount": 5000,
        "hIndex": 25,
        "externalIds": {
            "ORCID": "0000-0002-1825-0097",
            "DBLP": ["dblp/123"]
        }
    }"#;
    let author: Author = serde_json::from_str(json).unwrap();
    assert_eq!(author.name_or_default(), "John Doe");
    assert_eq!(author.h_index_value(), 25);
    assert_eq!(author.citations(), 5000);
    assert_eq!(author.papers(), 50);
    assert_eq!(author.primary_affiliation(), Some("MIT"));
    assert_eq!(author.orcid(), Some("0000-0002-1825-0097"));
}

#[test]
fn test_author_empty_affiliations() {
    let json = r#"{"authorId": "123", "affiliations": []}"#;
    let author: Author = serde_json::from_str(json).unwrap();
    assert!(author.affiliations.is_empty());
    assert_eq!(author.primary_affiliation(), None);
}

#[test]
fn test_author_null_metrics() {
    let json = r#"{"authorId": "123", "hIndex": null, "citationCount": null}"#;
    let author: Author = serde_json::from_str(json).unwrap();
    assert_eq!(author.h_index_value(), 0);
    assert_eq!(author.citations(), 0);
}

#[test]
fn test_author_unicode_name() {
    let json = r#"{"authorId": "123", "name": "张三 (Zhang San)"}"#;
    let author: Author = serde_json::from_str(json).unwrap();
    assert!(author.name_or_default().contains("张三"));
}

// =============================================================================
// AuthorRef Edge Cases
// =============================================================================

#[test]
fn test_author_ref_minimal() {
    let json = r"{}";
    let author: AuthorRef = serde_json::from_str(json).unwrap();
    assert_eq!(author.id(), "");
    assert_eq!(author.name_or_default(), "Unknown");
}

#[test]
fn test_author_ref_with_data() {
    let json = r#"{"authorId": "123", "name": "Jane"}"#;
    let author: AuthorRef = serde_json::from_str(json).unwrap();
    assert_eq!(author.id(), "123");
    assert_eq!(author.name_or_default(), "Jane");
}

// =============================================================================
// AuthorSearchResult Edge Cases
// =============================================================================

#[test]
fn test_author_search_result() {
    let json = r#"{
        "total": 10,
        "offset": 0,
        "next": 5,
        "data": [{"authorId": "123"}]
    }"#;
    let result: AuthorSearchResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.total, 10);
    assert!(result.has_more());
    assert_eq!(result.data.len(), 1);
}

// =============================================================================
// CitationResult Edge Cases
// =============================================================================

#[test]
fn test_citation_result_empty() {
    let json = r#"{"offset": 0, "data": []}"#;
    let result: CitationResult = serde_json::from_str(json).unwrap();
    assert!(result.data.is_empty());
}

#[test]
fn test_citation_result_with_papers() {
    let json = r#"{
        "offset": 0,
        "next": 10,
        "data": [
            {
                "citingPaper": {"paperId": "citing1", "title": "Citing Paper"},
                "contexts": ["...in the context of..."],
                "intents": ["background"],
                "isInfluential": true
            }
        ]
    }"#;
    let result: CitationResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.data.len(), 1);
    let ctx = &result.data[0];
    assert!(ctx.is_influential);
    assert!(!ctx.contexts.is_empty());
}

// =============================================================================
// Enum Edge Cases
// =============================================================================

#[test]
fn test_response_format_serialize_roundtrip() {
    let formats = [ResponseFormat::Markdown, ResponseFormat::Json];
    for format in formats {
        let serialized = serde_json::to_string(&format).unwrap();
        let deserialized: ResponseFormat = serde_json::from_str(&serialized).unwrap();
        assert_eq!(format.is_json(), deserialized.is_json());
    }
}

#[test]
fn test_search_direction_all_values() {
    for direction in ["citations", "references", "both"] {
        let json = format!("\"{direction}\"");
        let _: SearchDirection = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_export_format_extensions() {
    let formats = [
        (ExportFormat::Ris, "ris"),
        (ExportFormat::Bibtex, "bib"),
        (ExportFormat::Csv, "csv"),
        (ExportFormat::Endnote, "enw"),
    ];
    for (format, expected_ext) in formats {
        assert_eq!(format.extension(), expected_ext);
    }
}

#[test]
fn test_export_format_mime_types() {
    let formats = [
        (ExportFormat::Ris, "application/x-research-info-systems"),
        (ExportFormat::Bibtex, "application/x-bibtex"),
        (ExportFormat::Csv, "text/csv"),
        (ExportFormat::Endnote, "application/x-endnote-refer"),
    ];
    for (format, expected_mime) in formats {
        assert_eq!(format.mime_type(), expected_mime);
    }
}

#[test]
fn test_trend_granularity_values() {
    for granularity in ["year", "quarter"] {
        let json = format!("\"{granularity}\"");
        let _: TrendGranularity = serde_json::from_str(&json).unwrap();
    }
}

#[test]
fn test_pearl_growing_strategy_values() {
    for strategy in ["keywords", "authors", "citations", "all"] {
        let json = format!("\"{strategy}\"");
        let _: PearlGrowingStrategy = serde_json::from_str(&json).unwrap();
    }
}

// =============================================================================
// Complex Nesting Edge Cases
// =============================================================================

#[test]
fn test_paper_with_nested_nulls() {
    let json = r#"{
        "paperId": "p1",
        "openAccessPdf": {
            "url": null,
            "status": null
        },
        "tldr": {
            "text": null,
            "model": null
        }
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();
    assert!(paper.pdf_url().is_none());
    assert!(paper.tldr_text().is_none());
}

#[test]
fn test_search_result_with_complex_papers() {
    let json = r#"{
        "total": 2,
        "offset": 0,
        "data": [
            {
                "paperId": "p1",
                "title": "Paper 1",
                "authors": [{"authorId": "a1", "name": "Author 1"}]
            },
            {
                "paperId": "p2",
                "title": "Paper 2",
                "citationCount": 100
            }
        ]
    }"#;
    let result: SearchResult = serde_json::from_str(json).unwrap();
    assert_eq!(result.data.len(), 2);
    assert_eq!(result.data[0].authors.len(), 1);
    assert_eq!(result.data[1].citations(), 100);
}

// =============================================================================
// Serialization Tests
// =============================================================================

#[test]
fn test_paper_serialize_minimal() {
    let paper = Paper {
        paper_id: "abc123".to_string(),
        ..Default::default()
    };
    let json = serde_json::to_string(&paper).unwrap();
    assert!(json.contains("paperId"));
}

#[test]
fn test_author_serialize_roundtrip() {
    let author = Author {
        author_id: "123".to_string(),
        name: Some("Test Author".to_string()),
        affiliations: vec!["MIT".to_string()],
        homepage: None,
        paper_count: Some(50),
        citation_count: Some(1000),
        h_index: Some(15),
        external_ids: None,
    };

    let json = serde_json::to_string(&author).unwrap();
    let deserialized: Author = serde_json::from_str(&json).unwrap();

    assert_eq!(author.author_id, deserialized.author_id);
    assert_eq!(author.name_or_default(), deserialized.name_or_default());
    assert_eq!(author.h_index_value(), deserialized.h_index_value());
}

#[test]
fn test_search_result_serialize() {
    let result = SearchResult {
        total: 100,
        offset: 0,
        next: Some(10),
        data: vec![],
        message: None,
    };

    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("total"));
    assert!(json.contains("100"));
}
