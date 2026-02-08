//! Enrichment tools: batch_metadata, author_search, author_papers, paper_autocomplete,
//! paper_title_match, paper_authors, author_batch.

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{
    AuthorBatchInput, AuthorPapersInput, AuthorSearchInput, BatchMetadataInput, PaperAuthorsInput,
    PaperAutocompleteInput, PaperTitleMatchInput, ResponseFormat,
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

        let all_results = ctx
            .client
            .get_papers_batch_with_nulls(&params.paper_ids, &field_list)
            .await
            .map_err(ToolError::from)?;

        // Identify which IDs were not found
        let mut not_found: Vec<&str> = Vec::new();
        let mut papers = Vec::new();
        for (id, result) in params.paper_ids.iter().zip(all_results.iter()) {
            match result {
                Some(paper) => papers.push(paper.clone()),
                None => not_found.push(id),
            }
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = formatters::format_papers_markdown(&papers);
                if !not_found.is_empty() {
                    output.push_str(&format!(
                        "\n\n---\n\n**Not found ({}):** {}\n",
                        not_found.len(),
                        not_found.join(", ")
                    ));
                }
                Ok(output)
            }
            ResponseFormat::Json => {
                let compact = papers.iter().map(formatters::compact_paper).collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&json!({
                    "found": compact,
                    "not_found": not_found
                }))?)
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
            .search_authors(&params.query, 0, params.limit, fields::AUTHOR)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => Ok(formatters::format_authors_markdown(&result.data)),
            ResponseFormat::Json => {
                let compact =
                    result.data.iter().map(formatters::compact_author).collect::<Vec<_>>();
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
        let author = ctx.client.get_author(&params.author_id).await.map_err(ToolError::from)?;

        // Fetch papers using the dedicated /author/{id}/papers endpoint
        let mut all_papers = Vec::new();
        let mut offset = 0;
        let limit = 100;
        let max_results = params.limit;

        loop {
            if all_papers.len() as i32 >= max_results {
                break;
            }

            let result = ctx
                .client
                .get_author_papers(&params.author_id, offset, limit, fields::DEFAULT)
                .await
                .map_err(ToolError::from)?;

            for paper in result.data {
                // Apply client-side year filters
                if let Some(min_year) = params.year_start {
                    if paper.year.unwrap_or(0) < min_year {
                        continue;
                    }
                }
                if let Some(max_year) = params.year_end {
                    if paper.year.unwrap_or(i32::MAX) > max_year {
                        continue;
                    }
                }

                all_papers.push(paper);

                if all_papers.len() as i32 >= max_results {
                    break;
                }
            }

            if result.next.is_none() {
                break;
            }
            offset = result.next.unwrap_or(offset + limit);
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = formatters::format_author_markdown(&author);
                output.push_str(&format!("\n\n## Papers ({} found)\n\n", all_papers.len()));

                if params.year_start.is_some() || params.year_end.is_some() {
                    output.push_str(&format!(
                        "**Period:** {} - {}\n\n",
                        params
                            .year_start
                            .map(|y| y.to_string())
                            .unwrap_or_else(|| "any".to_string()),
                        params.year_end.map(|y| y.to_string()).unwrap_or_else(|| "any".to_string())
                    ));
                }

                output.push_str(&formatters::format_papers_markdown(&all_papers));
                Ok(output)
            }
            ResponseFormat::Json => {
                let compact_papers: Vec<_> =
                    all_papers.iter().map(formatters::compact_paper).collect();
                Ok(serde_json::to_string_pretty(&json!({
                    "author": formatters::compact_author(&author),
                    "papers": compact_papers,
                    "count": all_papers.len()
                }))?)
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

        let matches =
            ctx.client.autocomplete_papers(&params.query).await.map_err(ToolError::from)?;

        if matches.is_empty() {
            return Ok(format!(
                "# Paper Autocomplete\n\n**Query:** `{}`\n\nNo suggestions found.",
                params.query
            ));
        }

        // Enrich with paper details since autocomplete only returns IDs
        let ids: Vec<String> = matches.iter().map(|m| m.id.clone()).collect();
        let papers = ctx
            .client
            .get_papers_batch(&ids, fields::MINIMAL)
            .await
            .unwrap_or_default();

        // Build a lookup map for enrichment
        let paper_map: std::collections::HashMap<&str, &crate::models::Paper> =
            papers.iter().map(|p| (p.paper_id.as_str(), p)).collect();

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Paper Autocomplete\n\n**Query:** `{}`\n**Suggestions:** {}\n\n---\n\n",
                    params.query,
                    matches.len()
                );

                for (i, m) in matches.iter().enumerate() {
                    let title = paper_map
                        .get(m.id.as_str())
                        .and_then(|p| p.title.as_deref())
                        .or(m.match_.as_deref())
                        .unwrap_or("Unknown");
                    let year = paper_map
                        .get(m.id.as_str())
                        .and_then(|p| p.year)
                        .map(|y| format!(" ({})", y))
                        .unwrap_or_default();
                    output.push_str(&format!(
                        "{}. **{}**{}\n   - ID: `{}`\n\n",
                        i + 1,
                        title,
                        year,
                        m.id
                    ));
                }
                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string_pretty(&json!({
                "query": params.query,
                "suggestions": matches.iter().map(|m| {
                    let paper = paper_map.get(m.id.as_str());
                    json!({
                        "id": m.id,
                        "title": paper.and_then(|p| p.title.as_deref()).or(m.match_.as_deref()),
                        "year": paper.and_then(|p| p.year),
                        "citations": paper.map(|p| p.citations())
                    })
                }).collect::<Vec<_>>()
            }))?),
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
                    let mut output =
                        format!("# Paper Title Match\n\n**Query:** `{}`\n\n---\n\n", params.title);
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

        let authors =
            ctx.client.get_paper_authors(&params.paper_id).await.map_err(ToolError::from)?;

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
                let compact = authors.iter().map(formatters::compact_author).collect::<Vec<_>>();
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

        let authors =
            ctx.client.get_authors_batch(&params.author_ids).await.map_err(ToolError::from)?;

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
                let compact = authors.iter().map(formatters::compact_author).collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&json!({
                    "requested": params.author_ids.len(),
                    "found": authors.len(),
                    "authors": compact
                }))?)
            }
        }
    }
}
