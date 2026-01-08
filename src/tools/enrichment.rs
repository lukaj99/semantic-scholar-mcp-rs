//! Enrichment tools: batch_metadata, author_search, author_papers, paper_autocomplete,
//! paper_title_match, paper_authors, author_batch.

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{
    AuthorBatchInput, AuthorPapersInput, AuthorSearchInput, BatchMetadataInput,
    PaperAutocompleteInput, PaperAuthorsInput, PaperTitleMatchInput, ResponseFormat,
};

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
                "paperIds": {
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
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["paperIds"]
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
                "responseFormat": {
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
                "authorId": {
                    "type": "string",
                    "description": "Semantic Scholar author ID"
                },
                "yearStart": {
                    "type": "integer",
                    "description": "Minimum publication year"
                },
                "yearEnd": {
                    "type": "integer",
                    "description": "Maximum publication year"
                },
                "limit": {
                    "type": "integer",
                    "default": 100,
                    "maximum": 1000
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["authorId"]
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

/// Paper autocomplete tool.
pub struct PaperAutocompleteTool;

#[async_trait::async_trait]
impl McpTool for PaperAutocompleteTool {
    fn name(&self) -> &'static str {
        "paper_autocomplete"
    }

    fn description(&self) -> &'static str {
        "Get paper title suggestions for partial queries. \
         Useful for finding exact papers when you only remember part of the title."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Partial paper title to autocomplete"
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["query"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: PaperAutocompleteInput = serde_json::from_value(input)?;

        let matches = ctx
            .client
            .autocomplete_papers(&params.query)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => {
                if matches.is_empty() {
                    return Ok(format!(
                        "# Paper Autocomplete\n\n**Query:** `{}`\n\nNo suggestions found.",
                        params.query
                    ));
                }

                let mut output = format!(
                    "# Paper Autocomplete\n\n**Query:** `{}`\n**Suggestions:** {}\n\n---\n\n",
                    params.query,
                    matches.len()
                );

                for (i, m) in matches.iter().enumerate() {
                    let title = m.match_.as_deref().unwrap_or("Unknown");
                    output.push_str(&format!(
                        "{}. **{}**\n   - ID: `{}`\n\n",
                        i + 1,
                        title,
                        m.id
                    ));
                }
                Ok(output)
            }
            ResponseFormat::Json => {
                Ok(serde_json::to_string_pretty(&json!({
                    "query": params.query,
                    "suggestions": matches.iter().map(|m| json!({
                        "id": m.id,
                        "title": m.match_
                    })).collect::<Vec<_>>()
                }))?)
            }
        }
    }
}

/// Paper title match tool.
pub struct PaperTitleMatchTool;

#[async_trait::async_trait]
impl McpTool for PaperTitleMatchTool {
    fn name(&self) -> &'static str {
        "paper_title_match"
    }

    fn description(&self) -> &'static str {
        "Find a paper by exact or near-exact title match. \
         Returns the best matching paper for the given title."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Exact or near-exact paper title"
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["title"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: PaperTitleMatchInput = serde_json::from_value(input)?;

        let paper = ctx
            .client
            .search_paper_by_title(&params.title, fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => {
                if let Some(p) = paper {
                    let mut output = format!(
                        "# Paper Title Match\n\n**Query:** `{}`\n\n---\n\n",
                        params.title
                    );
                    output.push_str(&formatters::format_paper_markdown(&p, 1));
                    Ok(output)
                } else {
                    Ok(format!(
                        "# Paper Title Match\n\n**Query:** `{}`\n\n❌ No exact match found.",
                        params.title
                    ))
                }
            }
            ResponseFormat::Json => {
                if let Some(p) = paper {
                    Ok(serde_json::to_string_pretty(&json!({
                        "query": params.title,
                        "matched": true,
                        "paper": formatters::compact_paper(&p)
                    }))?)
                } else {
                    Ok(serde_json::to_string_pretty(&json!({
                        "query": params.title,
                        "matched": false,
                        "paper": null
                    }))?)
                }
            }
        }
    }
}

/// Paper authors tool.
pub struct PaperAuthorsTool;

#[async_trait::async_trait]
impl McpTool for PaperAuthorsTool {
    fn name(&self) -> &'static str {
        "paper_authors"
    }

    fn description(&self) -> &'static str {
        "Get detailed author information for a specific paper. \
         Returns full author profiles including affiliations, h-index, and citation counts."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperId": {
                    "type": "string",
                    "description": "Paper ID (S2, DOI:, ARXIV:, PMID:)"
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["paperId"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: PaperAuthorsInput = serde_json::from_value(input)?;

        let authors = ctx
            .client
            .get_paper_authors(&params.paper_id)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Paper Authors\n\n**Paper ID:** `{}`\n**Authors:** {}\n\n---\n\n",
                    params.paper_id,
                    authors.len()
                );
                output.push_str(&formatters::format_authors_markdown(&authors));
                Ok(output)
            }
            ResponseFormat::Json => {
                let compact = authors
                    .iter()
                    .map(formatters::compact_author)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&json!({
                    "paperId": params.paper_id,
                    "authors": compact
                }))?)
            }
        }
    }
}

/// Author batch metadata tool.
pub struct AuthorBatchTool;

#[async_trait::async_trait]
impl McpTool for AuthorBatchTool {
    fn name(&self) -> &'static str {
        "author_batch"
    }

    fn description(&self) -> &'static str {
        "Get detailed metadata for multiple authors efficiently. \
         Accepts up to 1000 author IDs per call."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "authorIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 1000,
                    "description": "Semantic Scholar author IDs"
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["authorIds"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: AuthorBatchInput = serde_json::from_value(input)?;

        let authors = ctx
            .client
            .get_authors_batch(&params.author_ids)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Author Batch Metadata\n\n**Requested:** {}\n**Found:** {}\n\n---\n\n",
                    params.author_ids.len(),
                    authors.len()
                );
                output.push_str(&formatters::format_authors_markdown(&authors));
                Ok(output)
            }
            ResponseFormat::Json => {
                let compact = authors
                    .iter()
                    .map(formatters::compact_author)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&json!({
                    "requested": params.author_ids.len(),
                    "found": authors.len(),
                    "authors": compact
                }))?)
            }
        }
    }
}
