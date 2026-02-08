//! Property-based tests for tool input models.

use proptest::prelude::*;
use semantic_scholar_mcp::models::{
    AuthorSearchInput, BatchMetadataInput, ExhaustiveSearchInput, ResponseFormat,
};

/// Generate arbitrary ExhaustiveSearchInput.
fn arb_exhaustive_search() -> impl Strategy<Value = ExhaustiveSearchInput> {
    (
        "[A-Za-z0-9 ]{1,50}",                // query
        proptest::option::of(1900i32..2030), // year_start
        proptest::option::of(1900i32..2030), // year_end
        proptest::option::of(0i32..1000),    // min_citations
        any::<bool>(),                       // open_access_only
        -1i32..1000,                         // max_results
    )
        .prop_map(
            |(query, year_start, year_end, min_citations, open_access_only, max_results)| {
                ExhaustiveSearchInput {
                    query,
                    year_start,
                    year_end,
                    fields_of_study: None,
                    min_citations,
                    open_access_only,
                    max_results,
                    include_embeddings: false,
                    response_format: ResponseFormat::default(),
                }
            },
        )
}

proptest! {
    /// ExhaustiveSearchInput roundtrip serialization.
    #[test]
    fn exhaustive_search_roundtrip(input in arb_exhaustive_search()) {
        let json = serde_json::to_value(&input).expect("serialize");
        let decoded: ExhaustiveSearchInput = serde_json::from_value(json).expect("deserialize");

        prop_assert_eq!(&input.query, &decoded.query);
        prop_assert_eq!(&input.year_start, &decoded.year_start);
        prop_assert_eq!(&input.year_end, &decoded.year_end);
        prop_assert_eq!(&input.max_results, &decoded.max_results);
    }

    /// Input with valid JSON matching schema deserializes correctly.
    #[test]
    fn exhaustive_search_accepts_valid_json(
        query in "[a-zA-Z ]{1,100}",
        year in proptest::option::of(1900i32..2030),
    ) {
        let json = serde_json::json!({
            "query": query,
            "yearStart": year,
        });

        let result = serde_json::from_value::<ExhaustiveSearchInput>(json);
        prop_assert!(result.is_ok());

        let input = result.unwrap();
        prop_assert_eq!(&input.query, &query);
        prop_assert_eq!(&input.year_start, &year);
    }

    /// ExhaustiveSearchInput handles arbitrary max_results including negative.
    #[test]
    fn exhaustive_search_handles_any_max_results(max_results in any::<i32>()) {
        let json = serde_json::json!({
            "query": "test query",
            "maxResults": max_results,
        });

        let result = serde_json::from_value::<ExhaustiveSearchInput>(json);
        prop_assert!(result.is_ok());

        let input = result.unwrap();
        prop_assert_eq!(input.max_results, max_results);
    }

    /// ExhaustiveSearchInput handles all response formats.
    #[test]
    fn exhaustive_search_response_format(use_json in any::<bool>()) {
        let format_str = if use_json { "json" } else { "markdown" };
        let json = serde_json::json!({
            "query": "test",
            "responseFormat": format_str,
        });

        let result = serde_json::from_value::<ExhaustiveSearchInput>(json);
        prop_assert!(result.is_ok());

        let input = result.unwrap();
        if use_json {
            prop_assert!(input.response_format.is_json());
        } else {
            prop_assert!(input.response_format.is_markdown());
        }
    }
}

#[test]
fn batch_metadata_accepts_paper_ids() {
    let json = serde_json::json!({
        "paperIds": ["abc123", "def456", "ghi789"]
    });

    let input: BatchMetadataInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.paper_ids.len(), 3);
}

#[test]
fn batch_metadata_accepts_empty_paper_ids() {
    let json = serde_json::json!({
        "paperIds": []
    });

    let input: BatchMetadataInput = serde_json::from_value(json).unwrap();
    assert!(input.paper_ids.is_empty());
}

#[test]
fn batch_metadata_accepts_fields_option() {
    let json = serde_json::json!({
        "paperIds": ["abc123"],
        "fields": ["title", "abstract", "year"]
    });

    let input: BatchMetadataInput = serde_json::from_value(json).unwrap();
    assert!(input.fields.is_some());
    assert_eq!(input.fields.unwrap().len(), 3);
}

#[test]
fn author_search_accepts_query() {
    let json = serde_json::json!({
        "query": "Albert Einstein"
    });

    let input: AuthorSearchInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.query, "Albert Einstein");
}

#[test]
fn author_search_has_default_limit() {
    let json = serde_json::json!({
        "query": "test"
    });

    let input: AuthorSearchInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.limit, 10); // default_author_limit
}

#[test]
fn author_search_accepts_custom_limit() {
    let json = serde_json::json!({
        "query": "test",
        "limit": 50
    });

    let input: AuthorSearchInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.limit, 50);
}

#[test]
fn input_rejects_missing_required_fields() {
    // ExhaustiveSearchInput requires query
    let json = serde_json::json!({
        "yearStart": 2020
    });

    let result = serde_json::from_value::<ExhaustiveSearchInput>(json);
    assert!(result.is_err());
}

#[test]
fn batch_metadata_rejects_missing_paper_ids() {
    let json = serde_json::json!({
        "fields": ["title"]
    });

    let result = serde_json::from_value::<BatchMetadataInput>(json);
    assert!(result.is_err());
}

#[test]
fn author_search_rejects_missing_query() {
    let json = serde_json::json!({
        "limit": 10
    });

    let result = serde_json::from_value::<AuthorSearchInput>(json);
    assert!(result.is_err());
}

#[test]
fn exhaustive_search_defaults_correctly() {
    let json = serde_json::json!({
        "query": "machine learning"
    });

    let input: ExhaustiveSearchInput = serde_json::from_value(json).unwrap();
    assert_eq!(input.query, "machine learning");
    assert!(input.year_start.is_none());
    assert!(input.year_end.is_none());
    assert!(input.fields_of_study.is_none());
    assert!(input.min_citations.is_none());
    assert!(!input.open_access_only);
    assert_eq!(input.max_results, 100); // default
    assert!(!input.include_embeddings);
    assert!(input.response_format.is_markdown());
}
