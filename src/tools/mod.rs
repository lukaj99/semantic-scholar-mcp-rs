//! MCP tool implementations.
//!
//! Each tool module provides functions that:
//! 1. Parse and validate input parameters
//! 2. Call the Semantic Scholar API client
//! 3. Format results as Markdown or JSON

mod advanced;
mod bibliometrics;
mod discovery;
mod enrichment;
mod export;
mod networks;
mod semantic;
mod systematic;
mod trends;

pub use advanced::*;
pub use bibliometrics::*;
pub use discovery::*;
pub use enrichment::*;
pub use export::*;
pub use networks::*;
pub use semantic::*;
pub use systematic::*;
pub use trends::*;

use std::sync::Arc;

use crate::client::SemanticScholarClient;
use crate::error::ToolResult;

/// Tool execution context.
pub struct ToolContext {
    /// API client.
    pub client: Arc<SemanticScholarClient>,
}

impl ToolContext {
    /// Create a new tool context.
    #[must_use]
    pub fn new(client: Arc<SemanticScholarClient>) -> Self {
        Self { client }
    }
}

/// Trait for MCP tools.
#[async_trait::async_trait]
pub trait McpTool: Send + Sync {
    /// Tool name (e.g., "exhaustive_search").
    fn name(&self) -> &'static str;

    /// Tool description for LLM.
    fn description(&self) -> &'static str;

    /// JSON Schema for input parameters.
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the tool with given input.
    async fn execute(
        &self,
        ctx: &ToolContext,
        input: serde_json::Value,
    ) -> ToolResult<String>;
}

/// Register all tools.
#[must_use]
pub fn register_all_tools() -> Vec<Box<dyn McpTool>> {
    vec![
        // Discovery tools (3)
        Box::new(discovery::ExhaustiveSearchTool),
        Box::new(discovery::RecommendationsTool),
        Box::new(discovery::CitationSnowballTool),

        // Enrichment tools (3)
        Box::new(enrichment::BatchMetadataTool),
        Box::new(enrichment::AuthorSearchTool),
        Box::new(enrichment::AuthorPapersTool),

        // Export tools (1)
        Box::new(export::ReferenceExportTool),

        // Systematic review tools (3)
        Box::new(systematic::PrismaSearchTool),
        Box::new(systematic::ScreeningExportTool),
        Box::new(systematic::PrismaFlowDiagramTool),

        // Semantic tools (2)
        Box::new(semantic::SemanticSearchTool),
        Box::new(semantic::LiteratureReviewPipelineTool),

        // Network tools (1)
        Box::new(networks::AuthorNetworkTool),

        // Trend tools (2)
        Box::new(trends::ResearchTrendsTool),
        Box::new(trends::VenueAnalyticsTool),

        // Bibliometrics tools (6)
        Box::new(bibliometrics::FieldWeightedImpactTool),
        Box::new(bibliometrics::HighlyCitedPapersTool),
        Box::new(bibliometrics::CitationHalfLifeTool),
        Box::new(bibliometrics::CocitationAnalysisTool),
        Box::new(bibliometrics::BibliographicCouplingTool),
        Box::new(bibliometrics::HotPapersTool),

        // Advanced tools (2)
        Box::new(advanced::PearlGrowingTool),
        Box::new(advanced::OrcidAuthorLookupTool),
    ]
}
