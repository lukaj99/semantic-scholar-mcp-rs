//! Comprehensive input validation tests for all 23 MCP tools.
//!
//! Tests default values, serialization, edge cases, and invalid inputs.

use semantic_scholar_mcp::models::*;

// =============================================================================
// ExhaustiveSearchInput Tests
// =============================================================================

#[test]
fn test_exhaustive_search_minimal() {
    let json = r#"{"query": "machine learning"}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "machine learning");
    assert_eq!(input.max_results, 100); // default
}

#[test]
fn test_exhaustive_search_all_fields() {
    let json = r#"{
        "query": "deep learning",
        "yearStart": 2020,
        "yearEnd": 2024,
        "fieldsOfStudy": ["Computer Science"],
        "minCitations": 10,
        "openAccessOnly": true,
        "maxResults": 500,
        "includeEmbeddings": true,
        "responseFormat": "json"
    }"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "deep learning");
    assert_eq!(input.year_start, Some(2020));
    assert_eq!(input.year_end, Some(2024));
    assert!(input.open_access_only);
    assert!(input.include_embeddings);
    assert_eq!(input.max_results, 500);
}

#[test]
fn test_exhaustive_search_empty_query() {
    let json = r#"{"query": ""}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "");
}

#[test]
fn test_exhaustive_search_unicode_query() {
    let json = r#"{"query": "机器学习 neural networks"}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert!(input.query.contains("机器学习"));
}

#[test]
fn test_exhaustive_search_special_chars_query() {
    let json = r#"{"query": "C++ memory \"quoted\""}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert!(input.query.contains("C++"));
}

#[test]
fn test_exhaustive_search_negative_year() {
    let json = r#"{"query": "test", "yearStart": -100}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.year_start, Some(-100));
}

#[test]
fn test_exhaustive_search_unlimited_results() {
    let json = r#"{"query": "test", "maxResults": -1}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.max_results, -1);
}

#[test]
fn test_exhaustive_search_zero_results() {
    let json = r#"{"query": "test", "maxResults": 0}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.max_results, 0);
}

#[test]
fn test_exhaustive_search_missing_query() {
    let json = r#"{"maxResults": 100}"#;
    let result: Result<ExhaustiveSearchInput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// =============================================================================
// RecommendationsInput Tests
// =============================================================================

#[test]
fn test_recommendations_minimal() {
    let json = r#"{"positivePaperIds": ["paper1", "paper2"]}"#;
    let input: RecommendationsInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.positive_paper_ids.len(), 2);
    assert_eq!(input.limit, 100);
}

#[test]
fn test_recommendations_with_negative() {
    let json = r#"{
        "positivePaperIds": ["p1"],
        "negativePaperIds": ["n1", "n2"]
    }"#;
    let input: RecommendationsInput = serde_json::from_str(json).unwrap();
    assert!(input.negative_paper_ids.is_some());
    assert_eq!(input.negative_paper_ids.unwrap().len(), 2);
}

#[test]
fn test_recommendations_single_paper() {
    let json = r#"{"positivePaperIds": ["single"]}"#;
    let input: RecommendationsInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.positive_paper_ids.len(), 1);
}

#[test]
fn test_recommendations_empty_positive() {
    let json = r#"{"positivePaperIds": []}"#;
    let input: RecommendationsInput = serde_json::from_str(json).unwrap();
    assert!(input.positive_paper_ids.is_empty());
}

#[test]
fn test_recommendations_missing_positive() {
    let json = r#"{"limit": 50}"#;
    let result: Result<RecommendationsInput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// =============================================================================
// CitationSnowballInput Tests
// =============================================================================

#[test]
fn test_snowball_minimal() {
    let json = r#"{"seedPaperIds": ["paper1"]}"#;
    let input: CitationSnowballInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.depth, 1);
    assert_eq!(input.max_per_paper, 100);
    assert!(input.deduplicate);
}

#[test]
fn test_snowball_all_directions() {
    for direction in ["citations", "references", "both"] {
        let json = format!(r#"{{"seedPaperIds": ["p1"], "direction": "{}"}}"#, direction);
        let input: CitationSnowballInput = serde_json::from_str(&json).unwrap();
        // Direction should parse
        assert!(!input.seed_paper_ids.is_empty());
    }
}

#[test]
fn test_snowball_invalid_direction() {
    let json = r#"{"seedPaperIds": ["p1"], "direction": "invalid"}"#;
    let result: Result<CitationSnowballInput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

#[test]
fn test_snowball_max_depth() {
    let json = r#"{"seedPaperIds": ["p1"], "depth": 3}"#;
    let input: CitationSnowballInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.depth, 3);
}

#[test]
fn test_snowball_zero_depth() {
    let json = r#"{"seedPaperIds": ["p1"], "depth": 0}"#;
    let input: CitationSnowballInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.depth, 0);
}

#[test]
fn test_snowball_deduplicate_false() {
    let json = r#"{"seedPaperIds": ["p1"], "deduplicate": false}"#;
    let input: CitationSnowballInput = serde_json::from_str(json).unwrap();
    assert!(!input.deduplicate);
}

// =============================================================================
// BatchMetadataInput Tests
// =============================================================================

#[test]
fn test_batch_minimal() {
    let json = r#"{"paperIds": ["id1", "id2"]}"#;
    let input: BatchMetadataInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_ids.len(), 2);
}

#[test]
fn test_batch_with_fields() {
    let json = r#"{"paperIds": ["id1"], "fields": ["title", "year", "citations"]}"#;
    let input: BatchMetadataInput = serde_json::from_str(json).unwrap();
    assert!(input.fields.is_some());
    assert_eq!(input.fields.unwrap().len(), 3);
}

#[test]
fn test_batch_doi_prefix() {
    let json = r#"{"paperIds": ["DOI:10.1234/test"]}"#;
    let input: BatchMetadataInput = serde_json::from_str(json).unwrap();
    assert!(input.paper_ids[0].starts_with("DOI:"));
}

#[test]
fn test_batch_arxiv_prefix() {
    let json = r#"{"paperIds": ["ARXIV:2401.12345"]}"#;
    let input: BatchMetadataInput = serde_json::from_str(json).unwrap();
    assert!(input.paper_ids[0].starts_with("ARXIV:"));
}

#[test]
fn test_batch_empty_ids() {
    let json = r#"{"paperIds": []}"#;
    let input: BatchMetadataInput = serde_json::from_str(json).unwrap();
    assert!(input.paper_ids.is_empty());
}

#[test]
fn test_batch_large_count() {
    let ids: Vec<String> = (0..500).map(|i| format!("id{}", i)).collect();
    let json = serde_json::json!({"paperIds": ids}).to_string();
    let input: BatchMetadataInput = serde_json::from_str(&json).unwrap();
    assert_eq!(input.paper_ids.len(), 500);
}

// =============================================================================
// AuthorSearchInput Tests
// =============================================================================

#[test]
fn test_author_search_minimal() {
    let json = r#"{"query": "John Smith"}"#;
    let input: AuthorSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "John Smith");
    assert_eq!(input.limit, 10);
}

#[test]
fn test_author_search_unicode_name() {
    let json = r#"{"query": "李明 Li Ming"}"#;
    let input: AuthorSearchInput = serde_json::from_str(json).unwrap();
    assert!(input.query.contains("李明"));
}

#[test]
fn test_author_search_with_limit() {
    let json = r#"{"query": "Einstein", "limit": 5}"#;
    let input: AuthorSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.limit, 5);
}

// =============================================================================
// AuthorPapersInput Tests
// =============================================================================

#[test]
fn test_author_papers_minimal() {
    let json = r#"{"authorId": "1234567"}"#;
    let input: AuthorPapersInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.author_id, "1234567");
    assert_eq!(input.limit, 100);
}

#[test]
fn test_author_papers_year_range() {
    let json = r#"{"authorId": "123", "yearStart": 2020, "yearEnd": 2024}"#;
    let input: AuthorPapersInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.year_start, Some(2020));
    assert_eq!(input.year_end, Some(2024));
}

// =============================================================================
// PrismaSearchInput Tests
// =============================================================================

#[test]
fn test_prisma_search_minimal() {
    let json = r#"{"queries": ["query1", "query2"]}"#;
    let input: PrismaSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.queries.len(), 2);
    assert_eq!(input.max_results_per_query, 500);
}

#[test]
fn test_prisma_search_single_query() {
    let json = r#"{"queries": ["single query"]}"#;
    let input: PrismaSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.queries.len(), 1);
}

#[test]
fn test_prisma_search_empty_queries() {
    let json = r#"{"queries": []}"#;
    let input: PrismaSearchInput = serde_json::from_str(json).unwrap();
    assert!(input.queries.is_empty());
}

// =============================================================================
// ScreeningExportInput Tests
// =============================================================================

#[test]
fn test_screening_export_minimal() {
    let json = r#"{"paperIds": ["p1", "p2"]}"#;
    let input: ScreeningExportInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_ids.len(), 2);
    assert!(input.include_abstract); // default true
    assert!(!input.include_tldr); // default false
}

#[test]
fn test_screening_export_with_tldr() {
    let json = r#"{"paperIds": ["p1"], "includeTldr": true}"#;
    let input: ScreeningExportInput = serde_json::from_str(json).unwrap();
    assert!(input.include_tldr);
}

// =============================================================================
// ReferenceExportInput Tests
// =============================================================================

#[test]
fn test_reference_export_minimal() {
    let json = r#"{"paperIds": ["p1"]}"#;
    let input: ReferenceExportInput = serde_json::from_str(json).unwrap();
    assert!(input.include_abstract);
}

#[test]
fn test_reference_export_all_formats() {
    for format in ["ris", "bibtex", "csv", "endnote"] {
        let json = format!(r#"{{"paperIds": ["p1"], "format": "{}"}}"#, format);
        let input: ReferenceExportInput = serde_json::from_str(&json).unwrap();
        assert!(!input.paper_ids.is_empty());
    }
}

#[test]
fn test_reference_export_invalid_format() {
    let json = r#"{"paperIds": ["p1"], "format": "invalid"}"#;
    let result: Result<ReferenceExportInput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// =============================================================================
// SemanticSearchInput Tests
// =============================================================================

#[test]
fn test_semantic_search_minimal() {
    let json = r#"{"seedPaperId": "abc123"}"#;
    let input: SemanticSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.seed_paper_id, "abc123");
    assert_eq!(input.limit, 100);
}

#[test]
fn test_semantic_search_with_filters() {
    let json = r#"{
        "seedPaperId": "abc",
        "yearStart": 2020,
        "fieldsOfStudy": ["Medicine"]
    }"#;
    let input: SemanticSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.year_start, Some(2020));
    assert!(input.fields_of_study.is_some());
}

// =============================================================================
// LiteratureReviewInput Tests
// =============================================================================

#[test]
fn test_literature_review_minimal() {
    let json = r#"{"query": "COVID-19 treatments"}"#;
    let input: LiteratureReviewInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "COVID-19 treatments");
    assert!(input.include_recommendations);
    assert!(input.include_citations);
    assert_eq!(input.max_papers, 200);
}

#[test]
fn test_literature_review_without_extras() {
    let json = r#"{
        "query": "test",
        "includeRecommendations": false,
        "includeCitations": false
    }"#;
    let input: LiteratureReviewInput = serde_json::from_str(json).unwrap();
    assert!(!input.include_recommendations);
    assert!(!input.include_citations);
}

// =============================================================================
// AuthorNetworkInput Tests
// =============================================================================

#[test]
fn test_author_network_minimal() {
    let json = r#"{"authorId": "123456"}"#;
    let input: AuthorNetworkInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.author_id, "123456");
    assert_eq!(input.depth, 1);
    assert_eq!(input.min_shared_papers, 2);
    assert_eq!(input.max_collaborators, 50);
}

#[test]
fn test_author_network_depth_2() {
    let json = r#"{"authorId": "123", "depth": 2}"#;
    let input: AuthorNetworkInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.depth, 2);
}

// =============================================================================
// TrendAnalysisInput Tests
// =============================================================================

#[test]
fn test_trend_analysis_minimal() {
    let json = r#"{"query": "AI safety", "yearStart": 2020, "yearEnd": 2024}"#;
    let input: TrendAnalysisInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "AI safety");
    assert_eq!(input.year_start, 2020);
    assert_eq!(input.year_end, 2024);
}

#[test]
fn test_trend_analysis_quarter_granularity() {
    let json = r#"{
        "query": "test",
        "yearStart": 2020,
        "yearEnd": 2024,
        "granularity": "quarter"
    }"#;
    let input: TrendAnalysisInput = serde_json::from_str(json).unwrap();
    // Should parse successfully
    assert!(!input.query.is_empty());
}

#[test]
fn test_trend_analysis_missing_years() {
    let json = r#"{"query": "test"}"#;
    let result: Result<TrendAnalysisInput, _> = serde_json::from_str(json);
    assert!(result.is_err()); // yearStart and yearEnd are required
}

// =============================================================================
// VenueAnalyticsInput Tests
// =============================================================================

#[test]
fn test_venue_analytics_minimal() {
    let json = r#"{"venueQuery": "NeurIPS"}"#;
    let input: VenueAnalyticsInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.venue_query, "NeurIPS");
    assert_eq!(input.max_papers, 500);
}

#[test]
fn test_venue_analytics_with_years() {
    let json = r#"{"venueQuery": "Nature", "yearStart": 2020, "yearEnd": 2024}"#;
    let input: VenueAnalyticsInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.year_start, Some(2020));
    assert_eq!(input.year_end, Some(2024));
}

// =============================================================================
// HotPapersInput Tests
// =============================================================================

#[test]
fn test_hot_papers_minimal() {
    let json = r#"{"query": "large language models"}"#;
    let input: HotPapersInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "large language models");
    assert_eq!(input.time_window_months, 24);
    assert_eq!(input.min_recent_citations, 10);
    assert_eq!(input.max_papers, 50);
}

#[test]
fn test_hot_papers_custom_window() {
    let json = r#"{"query": "test", "timeWindowMonths": 12}"#;
    let input: HotPapersInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.time_window_months, 12);
}

// =============================================================================
// PearlGrowingInput Tests
// =============================================================================

#[test]
fn test_pearl_growing_minimal() {
    let json = r#"{"seedPaperIds": ["paper1", "paper2"]}"#;
    let input: PearlGrowingInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.seed_paper_ids.len(), 2);
    assert_eq!(input.iterations, 2);
    assert!(input.deduplicate);
}

#[test]
fn test_pearl_growing_all_strategies() {
    for strategy in ["keywords", "authors", "citations", "all"] {
        let json = format!(r#"{{"seedPaperIds": ["p1"], "strategy": "{}"}}"#, strategy);
        let input: PearlGrowingInput = serde_json::from_str(&json).unwrap();
        assert!(!input.seed_paper_ids.is_empty());
    }
}

#[test]
fn test_pearl_growing_invalid_strategy() {
    let json = r#"{"seedPaperIds": ["p1"], "strategy": "invalid"}"#;
    let result: Result<PearlGrowingInput, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// =============================================================================
// FieldWeightedImpactInput Tests
// =============================================================================

#[test]
fn test_fwci_minimal() {
    let json = r#"{"paperIds": ["p1", "p2"]}"#;
    let input: FieldWeightedImpactInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_ids.len(), 2);
    assert_eq!(input.baseline_sample_size, 100);
}

#[test]
fn test_fwci_custom_baseline() {
    let json = r#"{"paperIds": ["p1"], "baselineSampleSize": 200}"#;
    let input: FieldWeightedImpactInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.baseline_sample_size, 200);
}

// =============================================================================
// HighlyCitedPapersInput Tests
// =============================================================================

#[test]
fn test_highly_cited_minimal() {
    let json = r#"{"paperIds": ["p1"]}"#;
    let input: HighlyCitedPapersInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_ids.len(), 1);
    assert!((input.percentile_threshold - 1.0).abs() < 0.001);
}

#[test]
fn test_highly_cited_custom_percentile() {
    let json = r#"{"paperIds": ["p1"], "percentileThreshold": 5.0}"#;
    let input: HighlyCitedPapersInput = serde_json::from_str(json).unwrap();
    assert!((input.percentile_threshold - 5.0).abs() < 0.001);
}

// =============================================================================
// CitationHalfLifeInput Tests
// =============================================================================

#[test]
fn test_half_life_minimal() {
    let json = r#"{"paperId": "abc123"}"#;
    let input: CitationHalfLifeInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_id, "abc123");
}

#[test]
fn test_half_life_json_format() {
    let json = r#"{"paperId": "abc123", "responseFormat": "json"}"#;
    let input: CitationHalfLifeInput = serde_json::from_str(json).unwrap();
    assert!(input.response_format.is_json());
}

// =============================================================================
// CocitationAnalysisInput Tests
// =============================================================================

#[test]
fn test_cocitation_minimal() {
    let json = r#"{"paperId": "abc123"}"#;
    let input: CocitationAnalysisInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_id, "abc123");
    assert_eq!(input.min_cocitations, 5);
    assert_eq!(input.max_citing_papers, 100);
    assert_eq!(input.max_results, 50);
}

#[test]
fn test_cocitation_custom_thresholds() {
    let json = r#"{"paperId": "p1", "minCocitations": 10, "maxCitingPapers": 200}"#;
    let input: CocitationAnalysisInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.min_cocitations, 10);
    assert_eq!(input.max_citing_papers, 200);
}

// =============================================================================
// BibliographicCouplingInput Tests
// =============================================================================

#[test]
fn test_bibliographic_coupling_minimal() {
    let json = r#"{"paperId": "abc123"}"#;
    let input: BibliographicCouplingInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.paper_id, "abc123");
    assert_eq!(input.min_shared_refs, 3);
    assert_eq!(input.max_refs_to_check, 50);
    assert_eq!(input.max_results, 50);
}

// =============================================================================
// OrcidAuthorLookupInput Tests
// =============================================================================

#[test]
fn test_orcid_minimal() {
    let json = r#"{"orcid": "0000-0002-1825-0097"}"#;
    let input: OrcidAuthorLookupInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.orcid, "0000-0002-1825-0097");
    assert!(!input.include_papers);
    assert_eq!(input.max_papers, 100);
}

#[test]
fn test_orcid_with_papers() {
    let json = r#"{"orcid": "0000-0002-1825-0097", "includePapers": true, "maxPapers": 50}"#;
    let input: OrcidAuthorLookupInput = serde_json::from_str(json).unwrap();
    assert!(input.include_papers);
    assert_eq!(input.max_papers, 50);
}

#[test]
fn test_orcid_various_formats() {
    // Valid ORCID formats
    for orcid in [
        "0000-0002-1825-0097",
        "0000-0001-5109-3700",
        "0000-0002-1694-233X", // X checksum
    ] {
        let json = format!(r#"{{"orcid": "{}"}}"#, orcid);
        let input: OrcidAuthorLookupInput = serde_json::from_str(&json).unwrap();
        assert_eq!(input.orcid, orcid);
    }
}

// =============================================================================
// PrismaFlowDiagramInput Tests
// =============================================================================

#[test]
fn test_prisma_flow_minimal() {
    let json = r#"{
        "identification": {
            "databases": [
                {"name": "Semantic Scholar", "results": 100}
            ]
        },
        "screening": {
            "recordsAfterDedup": 90,
            "recordsScreened": 90,
            "recordsExcluded": 40
        }
    }"#;
    let input: PrismaFlowDiagramInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.identification.databases.len(), 1);
    assert_eq!(input.screening.records_after_dedup, 90);
}

#[test]
fn test_prisma_flow_full() {
    let json = r#"{
        "identification": {
            "databases": [
                {"name": "S2", "query": "AI", "results": 100, "date": "2024-01-01"}
            ],
            "otherSources": [
                {"name": "Manual", "description": "Hand-picked", "records": 10}
            ]
        },
        "screening": {
            "recordsAfterDedup": 105,
            "recordsScreened": 105,
            "recordsExcluded": 50,
            "exclusionReasons": {"Off-topic": 30, "Not English": 20}
        },
        "eligibility": {
            "reportsSought": 55,
            "reportsNotRetrieved": 5,
            "reportsAssessed": 50,
            "reportsExcluded": 10
        },
        "included": {
            "studiesIncluded": 40,
            "reportsIncluded": 40
        }
    }"#;
    let input: PrismaFlowDiagramInput = serde_json::from_str(json).unwrap();
    assert!(input.eligibility.is_some());
    assert!(input.included.is_some());
    let eligibility = input.eligibility.unwrap();
    assert_eq!(eligibility.reports_sought, 55);
}

// =============================================================================
// ResponseFormat Tests
// =============================================================================

#[test]
fn test_response_format_markdown_default() {
    let format = ResponseFormat::default();
    assert!(format.is_markdown());
    assert!(!format.is_json());
}

#[test]
fn test_response_format_json_explicit() {
    let json = r#""json""#;
    let format: ResponseFormat = serde_json::from_str(json).unwrap();
    assert!(format.is_json());
}

#[test]
fn test_response_format_invalid() {
    let json = r#""xml""#;
    let result: Result<ResponseFormat, _> = serde_json::from_str(json);
    assert!(result.is_err());
}

// =============================================================================
// Edge Cases for All Inputs
// =============================================================================

#[test]
fn test_all_inputs_accept_extra_fields() {
    // JSON should ignore unknown fields by default
    let json = r#"{"query": "test", "unknownField": "ignored"}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert_eq!(input.query, "test");
}

#[test]
fn test_whitespace_in_strings() {
    let json = r#"{"query": "  spaced query  "}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert!(input.query.contains("spaced"));
}

#[test]
fn test_null_vs_missing() {
    // null should be treated as None
    let json = r#"{"query": "test", "yearStart": null}"#;
    let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();
    assert!(input.year_start.is_none());
}
