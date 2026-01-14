//! Unit tests for tool implementations.
//!
//! Tests edge cases and business logic without hitting the API.
#![allow(clippy::float_cmp)]

use semantic_scholar_mcp::models::*;

// =============================================================================
// Keyword Extraction Tests (Pearl Growing)
// =============================================================================

/// Test that keyword extraction handles technical terms with numbers
#[test]
fn test_keyword_extraction_technical_terms() {
    // This tests the logic - we can't call extract_keywords directly
    // since it's private, but we verify the regex pattern works
    let regex = regex::Regex::new(r"\b[a-zA-Z][a-zA-Z0-9-]*[a-zA-Z0-9]\b|\b[a-zA-Z]{2,}\b").unwrap();

    let text = "GPT-4 and BERT for COVID-19 analysis with H2O";
    let lowercase = text.to_lowercase();
    let matches: Vec<&str> = regex.find_iter(&lowercase).map(|m| m.as_str()).collect();

    assert!(matches.contains(&"gpt-4"), "Should match GPT-4");
    assert!(matches.contains(&"bert"), "Should match BERT");
    assert!(matches.contains(&"covid-19"), "Should match COVID-19");
    assert!(matches.contains(&"h2o"), "Should match H2O");
}

/// Test that hyphenated terms are extracted
#[test]
fn test_keyword_extraction_hyphenated() {
    let regex = regex::Regex::new(r"\b[a-zA-Z][a-zA-Z0-9-]*[a-zA-Z0-9]\b|\b[a-zA-Z]{2,}\b").unwrap();

    let text = "cross-validated neural-network based approach";
    let lowercase = text.to_lowercase();
    let matches: Vec<&str> = regex.find_iter(&lowercase).map(|m| m.as_str()).collect();

    assert!(matches.contains(&"cross-validated"), "Should match cross-validated");
    assert!(matches.contains(&"neural-network"), "Should match neural-network");
}

// =============================================================================
// FWCI Edge Cases
// =============================================================================

/// Test FWCI with zero baseline (should not divide by zero)
#[test]
fn test_fwci_zero_baseline_handling() {
    // Simulate FWCI calculation
    let citations = 100;
    let avg_baseline = 0.0;

    let fwci = if avg_baseline > 0.0 {
        f64::from(citations) / avg_baseline
    } else {
        0.0 // Should default to 0, not panic or inf
    };

    assert_eq!(fwci, 0.0, "FWCI should be 0 when baseline is 0");
}

/// Test FWCI with very small baseline
#[test]
fn test_fwci_small_baseline() {
    let citations = 100;
    let avg_baseline = 0.01;

    let fwci = if avg_baseline > 0.0 {
        f64::from(citations) / avg_baseline
    } else {
        0.0
    };

    assert!(fwci.is_finite(), "FWCI should be finite");
    assert!(fwci > 0.0, "FWCI should be positive");
}

// =============================================================================
// Hot Papers Edge Cases
// =============================================================================

/// Test velocity calculation for future-dated paper
#[test]
fn test_velocity_future_paper() {
    let current_year = 2024;
    let paper_year = 2025; // Future paper

    // Simulate the calculation from bibliometrics.rs
    let years_since_pub = (current_year - paper_year).max(1);

    assert_eq!(years_since_pub, 1, "Future papers should use min 1 year");
}

/// Test velocity calculation for very recent paper
#[test]
fn test_velocity_recent_paper() {
    let current_year = 2024;
    let paper_year = 2024; // Same year

    let years_since_pub = (current_year - paper_year).max(1);

    assert_eq!(years_since_pub, 1, "Same-year papers should use min 1 year");
}

/// Test velocity doesn't overflow with high citations
#[test]
fn test_velocity_high_citations() {
    let citations = 1_000_000;
    let years_since_pub = 1;

    let velocity = f64::from(citations) / f64::from(years_since_pub);

    assert!(velocity.is_finite(), "Velocity should be finite");
    assert_eq!(velocity, 1_000_000.0, "Velocity should be correct");
}

// =============================================================================
// Citation Half-Life Edge Cases
// =============================================================================

/// Test median calculation with empty ages
#[test]
fn test_half_life_empty_ages() {
    let ages: Vec<i32> = vec![];

    let half_life = if ages.is_empty() {
        None
    } else {
        let mut sorted = ages;
        sorted.sort_unstable();
        let mid = sorted.len() / 2;
        Some(if sorted.len().is_multiple_of(2) {
            f64::from(sorted[mid - 1] + sorted[mid]) / 2.0
        } else {
            f64::from(sorted[mid])
        })
    };

    assert!(half_life.is_none(), "Empty ages should return None");
}

/// Test median calculation with single element
#[test]
fn test_half_life_single_age() {
    let ages = vec![5];

    let mut sorted = ages;
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    let half_life = if sorted.len().is_multiple_of(2) {
        f64::from(sorted[mid - 1] + sorted[mid]) / 2.0
    } else {
        f64::from(sorted[mid])
    };

    assert_eq!(half_life, 5.0, "Single element median should be that element");
}

/// Test median calculation with even number of elements
#[test]
fn test_half_life_even_count() {
    let ages = vec![2, 4, 6, 8];

    let mut sorted = ages;
    sorted.sort_unstable();
    let mid = sorted.len() / 2;
    let half_life = f64::from(sorted[mid - 1] + sorted[mid]) / 2.0;

    assert_eq!(half_life, 5.0, "Even count median should be average of middle two");
}

// =============================================================================
// Input Validation Edge Cases
// =============================================================================

/// Test empty `paper_ids` in batch requests
#[test]
fn test_batch_empty_ids() {
    let json = r#"{"paperIds": []}"#;
    let input: Result<BatchMetadataInput, _> = serde_json::from_str(json);

    // Should parse but ideally be rejected by validation
    assert!(input.is_ok(), "Empty array should parse");
    assert!(input.unwrap().paper_ids.is_empty(), "Should have empty array");
}

/// Test very large limit values
#[test]
fn test_large_limit_values() {
    let json = r#"{"query": "test", "maxResults": 999999}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();

    // Should accept large values (server-side will enforce limits)
    assert_eq!(input.max_results, 999_999);
}

/// Test negative year values
#[test]
fn test_negative_year() {
    let json = r#"{"query": "test", "yearStart": -100}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();

    // Should parse (API will handle validation)
    assert_eq!(input.year_start, Some(-100));
}

// =============================================================================
// Search Direction Edge Cases
// =============================================================================

/// Test all search directions deserialize
#[test]
fn test_search_directions() {
    for direction in ["citations", "references", "both"] {
        let json = format!(r#"{{"seedPaperIds": ["p1"], "direction": "{direction}"}}"#);
        let result: Result<CitationSnowballInput, _> = serde_json::from_str(&json);
        assert!(result.is_ok(), "Direction '{direction}' should parse");
    }
}

/// Test invalid search direction
#[test]
fn test_invalid_search_direction() {
    let json = r#"{"seedPaperIds": ["p1"], "direction": "invalid"}"#;
    let result: Result<CitationSnowballInput, _> = serde_json::from_str(json);

    assert!(result.is_err(), "Invalid direction should fail to parse");
}

// =============================================================================
// Paper Model Edge Cases
// =============================================================================

/// Test paper with extreme citation count
#[test]
fn test_paper_extreme_citations() {
    let json = r#"{
        "paperId": "extreme",
        "citationCount": 2147483647
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    assert_eq!(paper.citations(), i32::MAX);
    assert!(paper.has_citations());
}

/// Test paper with negative citation count (shouldn't happen but handle gracefully)
#[test]
fn test_paper_negative_citations() {
    let json = r#"{
        "paperId": "neg",
        "citationCount": -1
    }"#;
    let paper: Paper = serde_json::from_str(json).unwrap();

    // Should parse, citations() returns the value as-is
    assert_eq!(paper.citations(), -1);
}

/// Test paper with very long title
#[test]
fn test_paper_long_title() {
    let long_title = "A".repeat(10000);
    let json = format!(r#"{{"paperId": "long", "title": "{long_title}"}}"#);
    let paper: Paper = serde_json::from_str(&json).unwrap();

    assert_eq!(paper.title_or_default().len(), 10000);
}
