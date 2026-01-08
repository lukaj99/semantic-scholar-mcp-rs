//! Discovery tools: exhaustive_search, recommendations, citation_snowball.

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{
    CitationSnowballInput, ExhaustiveSearchInput, RecommendationsInput, ResponseFormat,
};

/// Exhaustive paper search tool.
pub struct ExhaustiveSearchTool;

#[async_trait::async_trait]
impl McpTool for ExhaustiveSearchTool {
    fn name(&self) -> &'static str {
        "exhaustive_search"
    }

    fn description(&self) -> &'static str {
        "Search for papers with automatic pagination to retrieve ALL matching results. \
         Use for systematic reviews where comprehensive coverage is needed."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query (e.g., 'transformer attention mechanisms')"
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
                    "description": "Filter by fields (e.g., ['Computer Science'])"
                },
                "min_citations": {
                    "type": "integer",
                    "description": "Minimum citation count"
                },
                "open_access_only": {
                    "type": "boolean",
                    "default": false
                },
                "max_results": {
                    "type": "integer",
                    "default": 100,
                    "description": "Maximum papers to return (-1 for unlimited)"
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
        let params: ExhaustiveSearchInput = serde_json::from_value(input)?;

        let field_list: Vec<&str> = if params.include_embeddings {
            fields::EXTENDED.to_vec()
        } else {
            fields::DEFAULT.to_vec()
        };

        let mut all_papers = Vec::new();
        let mut offset = 0;
        let limit = 100; // API max per page
        let max_results = if params.max_results < 0 {
            i32::MAX
        } else {
            params.max_results
        };

        loop {
            if all_papers.len() as i32 >= max_results {
                break;
            }

            let result = ctx
                .client
                .search_papers(&params.query, offset, limit, &field_list)
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
                if let Some(min_citations) = params.min_citations {
                    if paper.citations() < min_citations {
                        continue;
                    }
                }
                if params.open_access_only && paper.pdf_url().is_none() {
                    continue;
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

        // Format output
        match params.response_format {
            ResponseFormat::Markdown => Ok(formatters::format_papers_markdown(&all_papers)),
            ResponseFormat::Json => {
                let compact = all_papers
                    .iter()
                    .map(formatters::compact_paper)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&compact)?)
            }
        }
    }
}

/// Paper recommendations tool.
pub struct RecommendationsTool;

#[async_trait::async_trait]
impl McpTool for RecommendationsTool {
    fn name(&self) -> &'static str {
        "recommendations"
    }

    fn description(&self) -> &'static str {
        "Get paper recommendations based on seed papers. Uses SPECTER2 embeddings \
         for semantic similarity."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "positive_paper_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to use as positive examples"
                },
                "negative_paper_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to avoid"
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
            "required": ["positive_paper_ids"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: RecommendationsInput = serde_json::from_value(input)?;

        let papers = ctx
            .client
            .get_recommendations(
                &params.positive_paper_ids,
                params.negative_paper_ids.as_deref(),
                params.limit,
                fields::DEFAULT,
            )
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

/// Citation snowball tool.
pub struct CitationSnowballTool;

#[async_trait::async_trait]
impl McpTool for CitationSnowballTool {
    fn name(&self) -> &'static str {
        "citation_snowball"
    }

    fn description(&self) -> &'static str {
        "Traverse the citation network from seed papers. Forward snowballing finds \
         papers that cite your seeds; backward finds papers your seeds cite."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "seed_paper_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Starting paper IDs"
                },
                "direction": {
                    "type": "string",
                    "enum": ["citations", "references", "both"],
                    "default": "both"
                },
                "depth": {
                    "type": "integer",
                    "default": 1,
                    "minimum": 1,
                    "maximum": 3
                },
                "max_per_paper": {
                    "type": "integer",
                    "default": 100
                },
                "min_citations": {
                    "type": "integer",
                    "default": 0,
                    "minimum": 0,
                    "description": "Minimum citations for included papers"
                },
                "deduplicate": {
                    "type": "boolean",
                    "default": true
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["seed_paper_ids"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: CitationSnowballInput = serde_json::from_value(input)?;

        use std::collections::{HashSet, VecDeque};
        use crate::models::SearchDirection;

        let mut seen: HashSet<String> = HashSet::new();
        let mut papers = Vec::new();
        let mut queue: VecDeque<(String, i32)> = VecDeque::new();

        // Initialize queue with seed papers
        for id in &params.seed_paper_ids {
            queue.push_back((id.clone(), 0));
            seen.insert(id.clone());
        }

        while let Some((paper_id, current_depth)) = queue.pop_front() {
            if current_depth >= params.depth {
                continue;
            }

            // Get citations (forward)
            if matches!(
                params.direction,
                SearchDirection::Citations | SearchDirection::Both
            ) {
                let result = ctx
                    .client
                    .get_citations(&paper_id, 0, params.max_per_paper, fields::DEFAULT)
                    .await;

                if let Ok(result) = result {
                    for ctx_paper in result.data {
                        if let Some(paper) = ctx_paper.paper {
                            // Filter by min_citations if set
                            let passes_filter = params.min_citations
                                .map(|min| paper.citation_count.unwrap_or(0) >= min)
                                .unwrap_or(true);

                            if passes_filter && !seen.contains(&paper.paper_id) {
                                if params.deduplicate {
                                    seen.insert(paper.paper_id.clone());
                                }
                                queue.push_back((paper.paper_id.clone(), current_depth + 1));
                                papers.push(paper);
                            }
                        }
                    }
                }
            }

            // Get references (backward)
            if matches!(
                params.direction,
                SearchDirection::References | SearchDirection::Both
            ) {
                let result = ctx
                    .client
                    .get_references(&paper_id, 0, params.max_per_paper, fields::DEFAULT)
                    .await;

                if let Ok(result) = result {
                    for ctx_paper in result.data {
                        if let Some(paper) = ctx_paper.paper {
                            // Filter by min_citations if set
                            let passes_filter = params.min_citations
                                .map(|min| paper.citation_count.unwrap_or(0) >= min)
                                .unwrap_or(true);

                            if passes_filter && !seen.contains(&paper.paper_id) {
                                if params.deduplicate {
                                    seen.insert(paper.paper_id.clone());
                                }
                                queue.push_back((paper.paper_id.clone(), current_depth + 1));
                                papers.push(paper);
                            }
                        }
                    }
                }
            }
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Citation Snowball Results\n\n\
                     **Seeds**: {} papers\n\
                     **Direction**: {:?}\n\
                     **Depth**: {}\n\
                     **Found**: {} papers\n\n---\n\n",
                    params.seed_paper_ids.len(),
                    params.direction,
                    params.depth,
                    papers.len()
                );
                output.push_str(&formatters::format_papers_markdown(&papers));
                Ok(output)
            }
            ResponseFormat::Json => {
                let compact = papers
                    .iter()
                    .map(formatters::compact_paper)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&json!({
                    "seeds": params.seed_paper_ids,
                    "direction": params.direction,
                    "depth": params.depth,
                    "count": papers.len(),
                    "papers": compact
                }))?)
            }
        }
    }
}
