//! Enrichment tools: batch_metadata, author_search, author_papers.

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{AuthorPapersInput, AuthorSearchInput, BatchMetadataInput, ResponseFormat};

/// Batch metadata retrieval tool.
pub struct BatchMetadataTool;

#[async_trait::async_trait]
impl McpTool for BatchMetadataTool {
    fn name(&self) -> &'static str {
        "batch_metadata"
    }

    fn description(&self) -> &'static str {
        "Retrieve detailed metadata for multiple papers efficiently. \
         Accepts up to 500 paper IDs per call."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paper_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 500,
                    "description": "Paper IDs (S2, DOI:, ARXIV:, PMID:)"
                },
                "fields": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Fields to retrieve"
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["paper_ids"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: BatchMetadataInput = serde_json::from_value(input)?;

        let field_list: Vec<&str> = params
            .fields
            .as_ref()
            .map(|f| f.iter().map(String::as_str).collect())
            .unwrap_or_else(|| fields::DEFAULT.to_vec());

        let papers = ctx
            .client
            .get_papers_batch(&params.paper_ids, &field_list)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => Ok(formatters::format_papers_markdown(&papers)),
            ResponseFormat::Json => {
                let compact = papers
                    .iter()
                    .map(formatters::compact_paper)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&compact)?)
            }
        }
    }
}

/// Author search tool.
pub struct AuthorSearchTool;

#[async_trait::async_trait]
impl McpTool for AuthorSearchTool {
    fn name(&self) -> &'static str {
        "author_search"
    }

    fn description(&self) -> &'static str {
        "Search for authors by name."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Author name to search for"
                },
                "limit": {
                    "type": "integer",
                    "default": 10,
                    "maximum": 100
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: AuthorSearchInput = serde_json::from_value(input)?;

        let result = ctx
            .client
            .search_authors(&params.query, 0, params.limit)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => Ok(formatters::format_authors_markdown(&result.data)),
            ResponseFormat::Json => {
                let compact = result
                    .data
                    .iter()
                    .map(formatters::compact_author)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&compact)?)
            }
        }
    }
}

/// Author papers tool.
pub struct AuthorPapersTool;

#[async_trait::async_trait]
impl McpTool for AuthorPapersTool {
    fn name(&self) -> &'static str {
        "author_papers"
    }

    fn description(&self) -> &'static str {
        "Get all papers by a specific author with optional year filtering."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "author_id": {
                    "type": "string",
                    "description": "Semantic Scholar author ID"
                },
                "year_start": {
                    "type": "integer",
                    "description": "Minimum publication year"
                },
                "year_end": {
                    "type": "integer",
                    "description": "Maximum publication year"
                },
                "limit": {
                    "type": "integer",
                    "default": 100,
                    "maximum": 1000
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["author_id"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: AuthorPapersInput = serde_json::from_value(input)?;

        // Get author info first
        let author = ctx
            .client
            .get_author(&params.author_id)
            .await
            .map_err(ToolError::from)?;

        // TODO: Implement paginated author papers fetch
        // For now, return author info with a placeholder
        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = formatters::format_author_markdown(&author);
                output.push_str("\n\n*Note: Paper list not yet implemented*");
                Ok(output)
            }
            ResponseFormat::Json => {
                Ok(serde_json::to_string_pretty(&formatters::compact_author(&author))?)
            }
        }
    }
}
