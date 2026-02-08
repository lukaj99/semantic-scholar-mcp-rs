//! Input models for MCP tool parameters.
//!
//! These map to the Pydantic input schemas in the Python version.

use serde::{Deserialize, Serialize};

use super::{
    ExportFormat, PearlGrowingStrategy, ResponseFormat, SearchDirection, TrendGranularity,
};

/// Input for exhaustive paper search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExhaustiveSearchInput {
    /// Search query (e.g., "transformer attention mechanisms").
    pub query: String,

    /// Minimum publication year (inclusive).
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year (inclusive).
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Filter by fields (e.g., ["Computer Science", "Medicine"]).
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,

    /// Minimum citation count filter.
    #[serde(default)]
    pub min_citations: Option<i32>,

    /// Only return papers with open access PDFs.
    #[serde(default)]
    pub open_access_only: bool,

    /// Maximum papers to return (use -1 for unlimited).
    #[serde(default = "default_max_results")]
    pub max_results: i32,

    /// Include SPECTER2 embeddings in results.
    #[serde(default)]
    pub include_embeddings: bool,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_max_results() -> i32 {
    100
}

/// Input for paper recommendations.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendationsInput {
    /// Paper IDs to use as positive examples.
    pub positive_paper_ids: Vec<String>,

    /// Paper IDs to use as negative examples.
    #[serde(default)]
    pub negative_paper_ids: Option<Vec<String>>,

    /// Maximum recommendations to return.
    #[serde(default = "default_limit")]
    pub limit: i32,

    /// Filter by fields of study.
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_limit() -> i32 {
    100
}

/// Input for citation snowball search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationSnowballInput {
    /// Starting paper IDs.
    pub seed_paper_ids: Vec<String>,

    /// Direction: citations, references, or both.
    #[serde(default)]
    pub direction: SearchDirection,

    /// Traversal depth (1 = direct, 2 = citations of citations).
    #[serde(default = "default_depth")]
    pub depth: i32,

    /// Maximum citations/references per paper.
    #[serde(default = "default_max_per_paper")]
    pub max_per_paper: i32,

    /// Minimum citations for included papers.
    #[serde(default)]
    pub min_citations: Option<i32>,

    /// Remove duplicates from results.
    #[serde(default = "default_true")]
    pub deduplicate: bool,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_depth() -> i32 {
    1
}

fn default_max_per_paper() -> i32 {
    100
}

fn default_true() -> bool {
    true
}

/// Input for batch paper metadata retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchMetadataInput {
    /// Paper IDs (Semantic Scholar, DOI:, ARXIV:, PMID:).
    pub paper_ids: Vec<String>,

    /// Fields to retrieve.
    #[serde(default)]
    pub fields: Option<Vec<String>>,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for author search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorSearchInput {
    /// Author name to search for.
    pub query: String,

    /// Maximum authors to return.
    #[serde(default = "default_author_limit")]
    pub limit: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_author_limit() -> i32 {
    10
}

/// Input for author papers retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorPapersInput {
    /// Semantic Scholar author ID.
    pub author_id: String,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Maximum papers to return.
    #[serde(default = "default_limit")]
    pub limit: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for PRISMA-compliant systematic review search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismaSearchInput {
    /// Search queries to run (will be deduplicated).
    pub queries: Vec<String>,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Minimum citation count.
    #[serde(default)]
    pub min_citations: Option<i32>,

    /// Maximum results per query.
    #[serde(default = "default_prisma_max")]
    pub max_results_per_query: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_prisma_max() -> i32 {
    500
}

/// Input for screening export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScreeningExportInput {
    /// Paper IDs to export for screening.
    pub paper_ids: Vec<String>,

    /// Include abstracts in export.
    #[serde(default = "default_true")]
    pub include_abstract: bool,

    /// Include AI-generated TLDRs.
    #[serde(default)]
    pub include_tldr: bool,
}

/// Input for reference export.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReferenceExportInput {
    /// Paper IDs to export.
    pub paper_ids: Vec<String>,

    /// Export format.
    #[serde(default)]
    pub format: ExportFormat,

    /// Include abstracts.
    #[serde(default = "default_true")]
    pub include_abstract: bool,
}

/// Input for semantic similarity search.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SemanticSearchInput {
    /// Seed paper to find similar papers.
    pub seed_paper_id: String,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Filter by fields of study.
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,

    /// Maximum similar papers to return.
    #[serde(default = "default_limit")]
    pub limit: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for literature review pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LiteratureReviewInput {
    /// Initial search query.
    pub query: String,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Minimum citations for included papers.
    #[serde(default)]
    pub min_citations: Option<i32>,

    /// Include recommendations from top papers.
    #[serde(default = "default_true")]
    pub include_recommendations: bool,

    /// Include citation network expansion.
    #[serde(default = "default_true")]
    pub include_citations: bool,

    /// Maximum total papers.
    #[serde(default = "default_review_max")]
    pub max_papers: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_review_max() -> i32 {
    200
}

/// Input for author network discovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorNetworkInput {
    /// Semantic Scholar author ID.
    pub author_id: String,

    /// Collaboration depth (1 = direct, 2 = collaborators of collaborators).
    #[serde(default = "default_network_depth")]
    pub depth: i32,

    /// Minimum shared papers to include.
    #[serde(default = "default_min_shared")]
    pub min_shared_papers: i32,

    /// Maximum collaborators to return.
    #[serde(default = "default_max_collaborators")]
    pub max_collaborators: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_network_depth() -> i32 {
    1
}

fn default_min_shared() -> i32 {
    2
}

fn default_max_collaborators() -> i32 {
    50
}

/// Input for research trend analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendAnalysisInput {
    /// Research topic to analyze.
    pub query: String,

    /// Start year for analysis.
    pub year_start: i32,

    /// End year for analysis.
    pub year_end: i32,

    /// Time granularity.
    #[serde(default)]
    pub granularity: TrendGranularity,

    /// Maximum papers per period.
    #[serde(default = "default_limit")]
    pub max_papers_per_period: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for venue analytics.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VenueAnalyticsInput {
    /// Venue name (e.g., "NeurIPS", "Nature Medicine").
    pub venue_query: String,

    /// Start year for analysis.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// End year for analysis.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Maximum papers to analyze.
    #[serde(default = "default_prisma_max")]
    pub max_papers: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for hot papers detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotPapersInput {
    /// Search query for candidate papers.
    pub query: String,

    /// Time window in months for velocity calculation.
    #[serde(default = "default_time_window")]
    pub time_window_months: i32,

    /// Minimum citations in the time window.
    #[serde(default = "default_min_recent_citations")]
    pub min_recent_citations: i32,

    /// Maximum papers to analyze.
    #[serde(default = "default_hot_papers_max")]
    pub max_papers: i32,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_time_window() -> i32 {
    24
}

fn default_min_recent_citations() -> i32 {
    10
}

fn default_hot_papers_max() -> i32 {
    50
}

/// Input for pearl growing search expansion.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PearlGrowingInput {
    /// Initial seed paper IDs.
    pub seed_paper_ids: Vec<String>,

    /// Number of growth iterations.
    #[serde(default = "default_iterations")]
    pub iterations: i32,

    /// Growth strategy.
    #[serde(default)]
    pub strategy: PearlGrowingStrategy,

    /// Maximum new papers per iteration.
    #[serde(default = "default_hot_papers_max")]
    pub max_papers_per_iteration: i32,

    /// Remove duplicates.
    #[serde(default = "default_true")]
    pub deduplicate: bool,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_iterations() -> i32 {
    2
}

/// Input for field-weighted citation impact.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldWeightedImpactInput {
    /// Paper IDs to analyze.
    pub paper_ids: Vec<String>,

    /// Sample size for baseline estimation.
    #[serde(default = "default_baseline_sample")]
    pub baseline_sample_size: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_baseline_sample() -> i32 {
    100
}

/// Input for highly cited papers detection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HighlyCitedPapersInput {
    /// Paper IDs to evaluate.
    pub paper_ids: Vec<String>,

    /// Top X percentile threshold.
    #[serde(default = "default_percentile")]
    pub percentile_threshold: f64,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_percentile() -> f64 {
    1.0
}

/// Input for citation half-life calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationHalfLifeInput {
    /// Paper ID to analyze.
    pub paper_id: String,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for co-citation analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CocitationAnalysisInput {
    /// Focal paper ID.
    pub paper_id: String,

    /// Minimum co-citation count.
    #[serde(default = "default_min_cocitations")]
    pub min_cocitations: i32,

    /// Maximum citing papers to analyze.
    #[serde(default = "default_limit")]
    pub max_citing_papers: i32,

    /// Maximum results.
    #[serde(default = "default_hot_papers_max")]
    pub max_results: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_min_cocitations() -> i32 {
    5
}

/// Input for bibliographic coupling.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BibliographicCouplingInput {
    /// Focal paper ID.
    pub paper_id: String,

    /// Minimum shared references.
    #[serde(default = "default_min_shared_refs")]
    pub min_shared_refs: i32,

    /// Maximum references to check.
    #[serde(default = "default_hot_papers_max")]
    pub max_refs_to_check: i32,

    /// Maximum results.
    #[serde(default = "default_hot_papers_max")]
    pub max_results: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_min_shared_refs() -> i32 {
    3
}

/// Input for ORCID author lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OrcidAuthorLookupInput {
    /// ORCID iD (e.g., "0000-0002-1825-0097").
    pub orcid: String,

    /// Include author's papers.
    #[serde(default)]
    pub include_papers: bool,

    /// Maximum papers to return.
    #[serde(default = "default_limit")]
    pub max_papers: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for PRISMA flow diagram generation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismaFlowDiagramInput {
    /// Identification phase data.
    pub identification: PrismaIdentificationData,

    /// Screening phase data.
    pub screening: PrismaScreeningData,

    /// Eligibility phase data (optional).
    #[serde(default)]
    pub eligibility: Option<PrismaEligibilityData>,

    /// Included phase data (optional).
    #[serde(default)]
    pub included: Option<PrismaIncludedData>,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// PRISMA identification phase data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrismaIdentificationData {
    /// Database search results.
    pub databases: Vec<DatabaseSearchData>,

    /// Other sources (optional).
    #[serde(default)]
    pub other_sources: Option<Vec<OtherSourceData>>,
}

/// Database search metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSearchData {
    /// Database name.
    pub name: String,

    /// Search query.
    #[serde(default)]
    pub query: Option<String>,

    /// Number of results.
    pub results: i32,

    /// Search date.
    #[serde(default)]
    pub date: Option<String>,
}

/// Other source metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtherSourceData {
    /// Source name.
    pub name: String,

    /// Description.
    #[serde(default)]
    pub description: Option<String>,

    /// Number of records.
    pub records: i32,
}

/// PRISMA screening phase data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismaScreeningData {
    /// Records after deduplication.
    pub records_after_dedup: i32,

    /// Records screened.
    pub records_screened: i32,

    /// Records excluded.
    pub records_excluded: i32,

    /// Exclusion reasons with counts.
    #[serde(default)]
    pub exclusion_reasons: Option<std::collections::HashMap<String, i32>>,
}

/// PRISMA eligibility phase data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismaEligibilityData {
    /// Reports sought for retrieval.
    #[serde(default)]
    pub reports_sought: i32,

    /// Reports not retrieved.
    #[serde(default)]
    pub reports_not_retrieved: i32,

    /// Reports assessed for eligibility.
    #[serde(default)]
    pub reports_assessed: i32,

    /// Reports excluded.
    #[serde(default)]
    pub reports_excluded: i32,

    /// Exclusion reasons.
    #[serde(default)]
    pub exclusion_reasons: Option<std::collections::HashMap<String, i32>>,
}

/// PRISMA included phase data.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrismaIncludedData {
    /// Studies included.
    #[serde(default)]
    pub studies_included: i32,

    /// Reports included.
    #[serde(default)]
    pub reports_included: i32,
}

/// Input for bulk boolean search (up to 10M papers).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BulkBooleanSearchInput {
    /// Boolean query: +term (AND), -term (NOT), |term (OR), "phrase", term*, term~N.
    pub query: String,

    /// Filter by fields of study.
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Minimum citation count.
    #[serde(default)]
    pub min_citations: Option<i32>,

    /// Filter by venue name.
    #[serde(default)]
    pub venue: Option<String>,

    /// Publication types: JournalArticle, Conference, Review, etc.
    #[serde(default)]
    pub publication_types: Option<Vec<String>>,

    /// Only return papers with open access PDFs.
    #[serde(default)]
    pub open_access_only: bool,

    /// Sort order: "citationCount:desc", "publicationDate:asc", "paperId:asc".
    #[serde(default)]
    pub sort: Option<String>,

    /// Maximum papers to return.
    #[serde(default = "default_bulk_max")]
    pub max_results: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_bulk_max() -> i32 {
    1000
}

/// Input for snippet search (full-text search with highlights).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SnippetSearchInput {
    /// Plain text search query.
    pub query: String,

    /// Filter to specific papers (up to ~100).
    #[serde(default)]
    pub paper_ids: Option<Vec<String>>,

    /// Filter by author names (fuzzy match, AND logic, max 10).
    #[serde(default)]
    pub authors: Option<Vec<String>>,

    /// Filter by fields of study.
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,

    /// Minimum publication year.
    #[serde(default)]
    pub year_start: Option<i32>,

    /// Maximum publication year.
    #[serde(default)]
    pub year_end: Option<i32>,

    /// Minimum citation count.
    #[serde(default)]
    pub min_citations: Option<i32>,

    /// Filter by venue name.
    #[serde(default)]
    pub venue: Option<String>,

    /// Maximum snippets to return.
    #[serde(default = "default_snippet_limit")]
    pub limit: i32,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

fn default_snippet_limit() -> i32 {
    100
}

/// Input for paper autocomplete (title suggestions).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperAutocompleteInput {
    /// Partial title text to autocomplete.
    pub query: String,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for paper title match (exact title search).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperTitleMatchInput {
    /// Exact or near-exact paper title.
    pub title: String,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for paper authors (detailed author info for a paper).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperAuthorsInput {
    /// Paper ID (Semantic Scholar ID, DOI:, ARXIV:, etc.).
    pub paper_id: String,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

/// Input for batch author metadata retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorBatchInput {
    /// Author IDs (up to 1000).
    pub author_ids: Vec<String>,

    /// Output format.
    #[serde(default)]
    pub response_format: ResponseFormat,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exhaustive_search_defaults() {
        let json = r#"{"query": "machine learning"}"#;
        let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.query, "machine learning");
        assert_eq!(input.max_results, 100);
        assert!(!input.open_access_only);
        assert!(!input.include_embeddings);
        assert!(input.response_format.is_markdown());
    }

    #[test]
    fn test_citation_snowball_defaults() {
        let json = r#"{"seedPaperIds": ["abc123"]}"#;
        let input: CitationSnowballInput = serde_json::from_str(json).unwrap();

        assert_eq!(input.seed_paper_ids, vec!["abc123"]);
        assert_eq!(input.depth, 1);
        assert_eq!(input.max_per_paper, 100);
        assert!(input.deduplicate);
    }

    #[test]
    fn test_response_format_json() {
        let json = r#"{"query": "test", "responseFormat": "json"}"#;
        let input: ExhaustiveSearchInput = serde_json::from_str(json).unwrap();

        assert!(input.response_format.is_json());
    }
}
