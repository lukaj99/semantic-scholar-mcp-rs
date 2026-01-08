//! Semantic search and literature review tools.

use std::collections::HashMap;

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{LiteratureReviewInput, ResponseFormat, SemanticSearchInput};

/// Semantic similarity search tool.
pub struct SemanticSearchTool;

#[async_trait::async_trait]
impl McpTool for SemanticSearchTool {
    fn name(&self) -> &'static str {
        "semantic_search"
    }

    fn description(&self) -> &'static str {
        "Find semantically similar papers using SPECTER2 embeddings. \
         Uses the recommendations API for embedding-based similarity."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "seed_paper_id": {
                    "type": "string",
                    "description": "Paper ID to find similar papers for"
                },
                "year_start": {
                    "type": "integer",
                    "description": "Minimum publication year"
                },
                "year_end": {
                    "type": "integer",
                    "description": "Maximum publication year"
                },
                "fields_of_study": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by fields of study"
                },
                "limit": {
                    "type": "integer",
                    "default": 100
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["seed_paper_id"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: SemanticSearchInput = serde_json::from_value(input)?;

        // Get recommendations using the seed paper
        let papers = ctx
            .client
            .get_recommendations(&[params.seed_paper_id.clone()], None, params.limit, fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        // Apply year filter
        let filtered: Vec<_> = papers
            .into_iter()
            .filter(|paper| {
                if let Some(min_year) = params.year_start {
                    if paper.year.unwrap_or(0) < min_year {
                        return false;
                    }
                }
                if let Some(max_year) = params.year_end {
                    if paper.year.unwrap_or(i32::MAX) > max_year {
                        return false;
                    }
                }
                true
            })
            .collect();

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Semantic Search Results\n\n\
                     **Seed paper:** {}\n\
                     **Similar papers found:** {}\n",
                    params.seed_paper_id,
                    filtered.len()
                );

                if params.year_start.is_some() || params.year_end.is_some() {
                    output.push_str(&format!(
                        "**Year filter:** {}-{}\n",
                        params.year_start.map(|y| y.to_string()).unwrap_or_else(|| "any".to_string()),
                        params.year_end.map(|y| y.to_string()).unwrap_or_else(|| "any".to_string())
                    ));
                }

                output.push_str("\n---\n\n");

                for (i, paper) in filtered.iter().enumerate() {
                    output.push_str(&format!("**Rank {}**\n", i + 1));
                    output.push_str(&formatters::format_paper_markdown(paper, i + 1));
                    output.push('\n');
                }

                if filtered.is_empty() {
                    output.push_str("*No similar papers found matching the criteria.*");
                }

                Ok(output)
            }
            ResponseFormat::Json => {
                let compact: Vec<_> = filtered.iter().map(formatters::compact_paper).collect();
                Ok(serde_json::to_string(&json!({
                    "seed_paper_id": params.seed_paper_id,
                    "total_found": filtered.len(),
                    "filters": {
                        "year_start": params.year_start,
                        "year_end": params.year_end,
                        "fields_of_study": params.fields_of_study
                    },
                    "similar_papers": compact
                }))?)
            }
        }
    }
}

/// Literature review pipeline tool.
pub struct LiteratureReviewPipelineTool;

#[async_trait::async_trait]
impl McpTool for LiteratureReviewPipelineTool {
    fn name(&self) -> &'static str {
        "literature_review_pipeline"
    }

    fn description(&self) -> &'static str {
        "Automated literature review combining search, recommendations, and citations. \
         Performs deduplication and ranks by citation count."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Initial search query"
                },
                "year_start": {
                    "type": "integer",
                    "description": "Minimum publication year"
                },
                "year_end": {
                    "type": "integer",
                    "description": "Maximum publication year"
                },
                "min_citations": {
                    "type": "integer",
                    "description": "Minimum citation count"
                },
                "include_recommendations": {
                    "type": "boolean",
                    "default": true
                },
                "include_citations": {
                    "type": "boolean",
                    "default": true
                },
                "max_papers": {
                    "type": "integer",
                    "default": 200
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
        let params: LiteratureReviewInput = serde_json::from_value(input)?;

        let mut all_papers: HashMap<String, crate::models::Paper> = HashMap::new();
        let mut sources = json!({
            "search": 0,
            "recommendations": 0,
            "citations": 0
        });
        let mut total_found = json!({
            "search": 0,
            "recommendations": 0,
            "citations": 0
        });

        // Step 1: Initial query search
        let mut offset = 0;
        let limit = 100;
        let max_search = params.max_papers.min(100);
        let mut search_papers = Vec::new();

        loop {
            if search_papers.len() >= max_search as usize {
                break;
            }

            let result = ctx
                .client
                .search_papers(&params.query, offset, limit, fields::DEFAULT)
                .await
                .map_err(ToolError::from)?;

            for paper in result.data {
                // Apply filters
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
                if let Some(min_cites) = params.min_citations {
                    if paper.citations() < min_cites {
                        continue;
                    }
                }
                search_papers.push(paper);
            }

            if result.next.is_none() {
                break;
            }
            offset = result.next.unwrap_or(offset + limit);
        }

        for paper in &search_papers {
            total_found["search"] = json!(total_found["search"].as_i64().unwrap_or(0) + 1);
            if !all_papers.contains_key(&paper.paper_id) {
                all_papers.insert(paper.paper_id.clone(), paper.clone());
                sources["search"] = json!(sources["search"].as_i64().unwrap_or(0) + 1);
            }
        }

        // Step 2: Get recommendations from top results
        if params.include_recommendations && !search_papers.is_empty() {
            let top_ids: Vec<String> = search_papers
                .iter()
                .take(5)
                .map(|p| p.paper_id.clone())
                .collect();

            if !top_ids.is_empty() {
                if let Ok(rec_papers) = ctx
                    .client
                    .get_recommendations(&top_ids, None, 50, fields::DEFAULT)
                    .await
                {
                    for paper in rec_papers {
                        total_found["recommendations"] =
                            json!(total_found["recommendations"].as_i64().unwrap_or(0) + 1);
                        if !all_papers.contains_key(&paper.paper_id) {
                            all_papers.insert(paper.paper_id.clone(), paper);
                            sources["recommendations"] =
                                json!(sources["recommendations"].as_i64().unwrap_or(0) + 1);
                        }
                    }
                }
            }
        }

        // Step 3: Expand via citations
        if params.include_citations && !search_papers.is_empty() {
            let top_ids: Vec<String> = search_papers
                .iter()
                .take(3)
                .map(|p| p.paper_id.clone())
                .collect();

            for seed_id in top_ids {
                // Get citations
                if let Ok(citations) = ctx
                    .client
                    .get_citations(&seed_id, 0, 20, fields::DEFAULT)
                    .await
                {
                    for ctx_paper in citations.data {
                        if let Some(paper) = ctx_paper.paper {
                            total_found["citations"] =
                                json!(total_found["citations"].as_i64().unwrap_or(0) + 1);
                            if !all_papers.contains_key(&paper.paper_id) {
                                all_papers.insert(paper.paper_id.clone(), paper);
                                sources["citations"] =
                                    json!(sources["citations"].as_i64().unwrap_or(0) + 1);
                            }
                        }
                    }
                }

                // Get references
                if let Ok(refs) = ctx
                    .client
                    .get_references(&seed_id, 0, 20, fields::DEFAULT)
                    .await
                {
                    for ctx_paper in refs.data {
                        if let Some(paper) = ctx_paper.paper {
                            total_found["citations"] =
                                json!(total_found["citations"].as_i64().unwrap_or(0) + 1);
                            if !all_papers.contains_key(&paper.paper_id) {
                                all_papers.insert(paper.paper_id.clone(), paper);
                                sources["citations"] =
                                    json!(sources["citations"].as_i64().unwrap_or(0) + 1);
                            }
                        }
                    }
                }
            }
        }

        // Apply final filters and sort
        let mut paper_list: Vec<_> = all_papers
            .into_values()
            .filter(|paper| {
                if let Some(min_year) = params.year_start {
                    if paper.year.unwrap_or(0) < min_year {
                        return false;
                    }
                }
                if let Some(max_year) = params.year_end {
                    if paper.year.unwrap_or(i32::MAX) > max_year {
                        return false;
                    }
                }
                if let Some(min_cites) = params.min_citations {
                    if paper.citations() < min_cites {
                        return false;
                    }
                }
                true
            })
            .collect();

        // Sort by citations descending
        paper_list.sort_by(|a, b| b.citations().cmp(&a.citations()));

        // Calculate stats
        let total_before_dedup = total_found["search"].as_i64().unwrap_or(0)
            + total_found["recommendations"].as_i64().unwrap_or(0)
            + total_found["citations"].as_i64().unwrap_or(0);
        let total_unique = paper_list.len();
        let duplicates_removed = total_before_dedup as usize - total_unique;

        // Limit to max_papers
        paper_list.truncate(params.max_papers as usize);

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Literature Review Results\n\n\
                     **Query:** {}\n\
                     **Total unique papers:** {}\n\n\
                     ## Sources\n\
                     - Search: {} papers\n\
                     - Recommendations: {} papers\n\
                     - Citations: {} papers\n\
                     - Duplicates removed: {}\n\n\
                     ---\n\n\
                     ## Papers (sorted by citations)\n\n",
                    params.query,
                    paper_list.len(),
                    sources["search"],
                    sources["recommendations"],
                    sources["citations"],
                    duplicates_removed
                );

                for (i, paper) in paper_list.iter().take(50).enumerate() {
                    output.push_str(&format!(
                        "### {}. {}\n",
                        i + 1,
                        paper.title_or_default()
                    ));
                    output.push_str(&formatters::format_paper_markdown(paper, i + 1));
                }

                if paper_list.len() > 50 {
                    output.push_str(&format!("\n*... and {} more papers*", paper_list.len() - 50));
                }

                Ok(output)
            }
            ResponseFormat::Json => {
                let compact: Vec<_> = paper_list.iter().map(formatters::compact_paper).collect();
                Ok(serde_json::to_string(&json!({
                    "query": params.query,
                    "total_unique": total_unique,
                    "duplicates_removed": duplicates_removed,
                    "sources": sources,
                    "filters": {
                        "year_start": params.year_start,
                        "year_end": params.year_end,
                        "min_citations": params.min_citations
                    },
                    "papers": compact
                }))?)
            }
        }
    }
}
