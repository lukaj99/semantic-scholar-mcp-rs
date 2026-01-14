//! Property-based tests for Paper model.

use proptest::prelude::*;
use semantic_scholar_mcp::models::Paper;

/// Generate arbitrary Paper structs for testing.
fn arb_paper() -> impl Strategy<Value = Paper> {
    (
        "[a-f0-9]{40}",                              // paper_id (SHA-like)
        proptest::option::of("[A-Za-z0-9 ]{1,100}"), // title
        proptest::option::of("[A-Za-z0-9 .,]{0,500}"), // abstract
        proptest::option::of(1900i32..2030),         // year
        proptest::option::of(0i32..1_000_000),       // citation_count
        proptest::option::of(0i32..10_000),          // reference_count
    )
        .prop_map(
            |(paper_id, title, r#abstract, year, citation_count, reference_count)| Paper {
                paper_id,
                title,
                r#abstract,
                year,
                citation_count,
                reference_count,
                ..Default::default()
            },
        )
}

proptest! {
    /// Paper serialization roundtrip: serialize then deserialize should preserve data.
    #[test]
    fn paper_roundtrip(paper in arb_paper()) {
        let json = serde_json::to_value(&paper).expect("serialize");
        let decoded: Paper = serde_json::from_value(json).expect("deserialize");

        prop_assert_eq!(&paper.paper_id, &decoded.paper_id);
        prop_assert_eq!(&paper.title, &decoded.title);
        prop_assert_eq!(&paper.year, &decoded.year);
        prop_assert_eq!(&paper.citation_count, &decoded.citation_count);
    }

    /// Paper deserialization never panics on arbitrary JSON objects.
    #[test]
    fn paper_from_arbitrary_json_object_never_panics(
        title in proptest::option::of(".*"),
        year in proptest::option::of(any::<i32>()),
        citations in proptest::option::of(any::<i32>()),
    ) {
        let json = serde_json::json!({
            "paperId": "test123",
            "title": title,
            "year": year,
            "citationCount": citations,
        });

        // Should not panic - may succeed or fail gracefully
        let _ = serde_json::from_value::<Paper>(json);
    }

    /// Paper handles extreme citation counts.
    #[test]
    fn paper_handles_extreme_values(
        citations in any::<i32>(),
        references in any::<i32>(),
    ) {
        let json = serde_json::json!({
            "paperId": "extreme_test",
            "citationCount": citations,
            "referenceCount": references,
        });

        let result = serde_json::from_value::<Paper>(json);
        prop_assert!(result.is_ok());

        let paper = result.unwrap();
        prop_assert_eq!(paper.citation_count, Some(citations));
        prop_assert_eq!(paper.reference_count, Some(references));
    }
}

#[test]
fn paper_handles_missing_fields() {
    // Minimal valid paper - only paperId required
    let json = serde_json::json!({"paperId": "abc123"});
    let paper: Paper = serde_json::from_value(json).unwrap();

    assert_eq!(paper.paper_id, "abc123");
    assert!(paper.title.is_none());
    assert!(paper.year.is_none());
}

#[test]
fn paper_handles_null_fields() {
    let json = serde_json::json!({
        "paperId": "abc123",
        "title": null,
        "year": null,
        "citationCount": null,
    });
    let paper: Paper = serde_json::from_value(json).unwrap();

    assert_eq!(paper.paper_id, "abc123");
    assert!(paper.title.is_none());
    assert!(paper.year.is_none());
}
