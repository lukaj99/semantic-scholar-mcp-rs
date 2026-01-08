//! Research trend analysis tools: research_trends, venue_analytics.

use std::collections::HashMap;

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::models::{ResponseFormat, TrendAnalysisInput, VenueAnalyticsInput};

/// Research trend analysis tool.
pub struct ResearchTrendsTool;

#[async_trait::async_trait]
impl McpTool for ResearchTrendsTool {
    fn name(&self) -> &'static str {
        "research_trends"
    }

    fn description(&self) -> &'static str {
        "Analyze publication trends for a research topic over time. \
         Groups papers by year with statistics and top papers per period."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Research topic to analyze"
                },
                "year_start": {
                    "type": "integer",
                    "description": "Start year for analysis"
                },
                "year_end": {
                    "type": "integer",
                    "description": "End year for analysis"
                },
                "granularity": {
                    "type": "string",
                    "enum": ["year", "quarter"],
                    "default": "year"
                },
                "max_papers_per_period": {
                    "type": "integer",
                    "default": 100
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["query", "year_start", "year_end"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: TrendAnalysisInput = serde_json::from_value(input)?;

        // Search for papers in the time range
        let max_results = params.max_papers_per_period * (params.year_end - params.year_start + 1);
        let mut all_papers = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            if all_papers.len() >= max_results as usize {
                break;
            }

            let result = ctx
                .client
                .search_papers(&params.query, offset, limit, fields::DEFAULT)
                .await
                .map_err(ToolError::from)?;

            for paper in result.data {
                if let Some(year) = paper.year {
                    if year >= params.year_start && year <= params.year_end {
                        all_papers.push(paper);
                    }
                }
            }

            if result.next.is_none() {
                break;
            }
            offset = result.next.unwrap_or(offset + limit);
        }

        // Group papers by year
        let mut papers_by_year: HashMap<i32, Vec<crate::models::Paper>> = HashMap::new();
        for paper in &all_papers {
            if let Some(year) = paper.year {
                papers_by_year.entry(year).or_default().push(paper.clone());
            }
        }

        // Build trend data
        let mut trends = Vec::new();
        for year in params.year_start..=params.year_end {
            let year_papers = papers_by_year.get(&year).cloned().unwrap_or_default();

            let paper_count = year_papers.len();
            let total_citations: i32 = year_papers.iter().map(|p| p.citations()).sum();
            let avg_citations = if paper_count > 0 {
                total_citations as f64 / paper_count as f64
            } else {
                0.0
            };

            // Get top papers by citations
            let mut sorted_papers = year_papers.clone();
            sorted_papers.sort_by(|a, b| b.citations().cmp(&a.citations()));
            let top_papers: Vec<_> = sorted_papers
                .iter()
                .take(3)
                .map(|p| {
                    json!({
                        "paperId": p.paper_id,
                        "title": p.title_or_default(),
                        "citations": p.citations()
                    })
                })
                .collect();

            trends.push(json!({
                "year": year,
                "paper_count": paper_count,
                "total_citations": total_citations,
                "avg_citations": (avg_citations * 10.0).round() / 10.0,
                "top_papers": top_papers
            }));
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Research Trends\n\n\
                     **Query:** {}\n\
                     **Period:** {} - {}\n\
                     **Total papers:** {}\n\n\
                     ---\n\n\
                     ## Yearly Breakdown\n\n",
                    params.query,
                    params.year_start,
                    params.year_end,
                    all_papers.len()
                );

                for trend in &trends {
                    output.push_str(&format!("### {}\n", trend["year"]));
                    output.push_str(&format!("- Papers: {}\n", trend["paper_count"]));
                    output.push_str(&format!("- Total citations: {}\n", trend["total_citations"]));
                    output.push_str(&format!("- Avg citations: {}\n", trend["avg_citations"]));

                    if let Some(top) = trend["top_papers"].as_array() {
                        if !top.is_empty() {
                            output.push_str("- Top papers:\n");
                            for tp in top {
                                output.push_str(&format!(
                                    "  - {} ({} citations)\n",
                                    tp["title"].as_str().unwrap_or("Unknown"),
                                    tp["citations"]
                                ));
                            }
                        }
                    }
                    output.push('\n');
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "query": params.query,
                "year_start": params.year_start,
                "year_end": params.year_end,
                "total_papers": all_papers.len(),
                "trends": trends
            }))?),
        }
    }
}

/// Venue analytics tool.
pub struct VenueAnalyticsTool;

#[async_trait::async_trait]
impl McpTool for VenueAnalyticsTool {
    fn name(&self) -> &'static str {
        "venue_analytics"
    }

    fn description(&self) -> &'static str {
        "Analyze publication statistics for a venue/conference. \
         Calculates metrics and identifies top papers."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "venue_query": {
                    "type": "string",
                    "description": "Venue name (e.g., 'NeurIPS', 'Nature Medicine')"
                },
                "year_start": {
                    "type": "integer",
                    "description": "Start year for analysis"
                },
                "year_end": {
                    "type": "integer",
                    "description": "End year for analysis"
                },
                "max_papers": {
                    "type": "integer",
                    "default": 500
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["venue_query"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: VenueAnalyticsInput = serde_json::from_value(input)?;

        // Search for papers in this venue
        let mut all_papers = Vec::new();
        let mut offset = 0;
        let limit = 100;

        loop {
            if all_papers.len() >= params.max_papers as usize {
                break;
            }

            let result = ctx
                .client
                .search_papers(&params.venue_query, offset, limit, fields::DEFAULT)
                .await
                .map_err(ToolError::from)?;

            for paper in result.data {
                // Apply year filters
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
            }

            if result.next.is_none() {
                break;
            }
            offset = result.next.unwrap_or(offset + limit);
        }

        // Calculate statistics
        let total_papers = all_papers.len();
        let total_citations: i32 = all_papers.iter().map(|p| p.citations()).sum();
        let avg_citations = if total_papers > 0 {
            total_citations as f64 / total_papers as f64
        } else {
            0.0
        };

        // Papers by year
        let mut years: HashMap<i32, i32> = HashMap::new();
        for paper in &all_papers {
            if let Some(year) = paper.year {
                *years.entry(year).or_insert(0) += 1;
            }
        }

        // Top papers
        let mut sorted_papers = all_papers.clone();
        sorted_papers.sort_by(|a, b| b.citations().cmp(&a.citations()));
        let top_papers: Vec<_> = sorted_papers
            .iter()
            .take(10)
            .map(|p| {
                json!({
                    "paperId": p.paper_id,
                    "title": p.title_or_default(),
                    "year": p.year,
                    "citations": p.citations()
                })
            })
            .collect();

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Venue Analytics\n\n\
                     **Venue:** {}\n",
                    params.venue_query
                );

                if params.year_start.is_some() || params.year_end.is_some() {
                    output.push_str(&format!(
                        "**Period:** {} - {}\n",
                        params.year_start.map(|y| y.to_string()).unwrap_or_else(|| "any".to_string()),
                        params.year_end.map(|y| y.to_string()).unwrap_or_else(|| "any".to_string())
                    ));
                }

                output.push_str(&format!(
                    "\n## Statistics\n\
                     - Total papers: {}\n\
                     - Total citations: {}\n\
                     - Avg citations: {:.1}\n\n\
                     ## Papers by Year\n",
                    total_papers,
                    total_citations,
                    avg_citations
                ));

                let mut year_list: Vec<_> = years.into_iter().collect();
                year_list.sort_by(|a, b| b.0.cmp(&a.0));
                for (year, count) in year_list {
                    output.push_str(&format!("- {}: {} papers\n", year, count));
                }

                output.push_str("\n## Top Cited Papers\n\n");
                for (i, paper) in top_papers.iter().enumerate() {
                    output.push_str(&format!(
                        "{}. **{}** ({})\n   - {} citations\n\n",
                        i + 1,
                        paper["title"].as_str().unwrap_or("Unknown"),
                        paper["year"],
                        paper["citations"]
                    ));
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "venue_query": params.venue_query,
                "year_start": params.year_start,
                "year_end": params.year_end,
                "statistics": {
                    "total_papers": total_papers,
                    "total_citations": total_citations,
                    "avg_citations": (avg_citations * 10.0).round() / 10.0
                },
                "papers_by_year": years,
                "top_papers": top_papers
            }))?),
        }
    }
}
