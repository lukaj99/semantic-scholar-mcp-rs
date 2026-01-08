//! Systematic review tools: prisma_search, screening_export, prisma_flow_diagram.

use std::collections::HashMap;

use chrono::Utc;
use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{
    PrismaFlowDiagramInput, PrismaSearchInput, ResponseFormat, ScreeningExportInput,
};

/// PRISMA-compliant multi-query search tool.
pub struct PrismaSearchTool;

#[async_trait::async_trait]
impl McpTool for PrismaSearchTool {
    fn name(&self) -> &'static str {
        "prisma_search"
    }

    fn description(&self) -> &'static str {
        "Run multiple search queries with deduplication and logging. \
         Designed for systematic reviews following PRISMA guidelines."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "queries": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "List of search queries to run"
                },
                "yearStart": {
                    "type": "integer",
                    "description": "Minimum publication year"
                },
                "yearEnd": {
                    "type": "integer",
                    "description": "Maximum publication year"
                },
                "minCitations": {
                    "type": "integer",
                    "description": "Minimum citation count"
                },
                "maxResultsPerQuery": {
                    "type": "integer",
                    "default": 500
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["queries"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: PrismaSearchInput = serde_json::from_value(input)?;

        let mut all_papers: HashMap<String, crate::models::Paper> = HashMap::new();
        let mut search_log = Vec::new();
        let mut results_per_query: HashMap<String, usize> = HashMap::new();

        for query in &params.queries {
            let mut offset = 0;
            let limit = 100;
            let mut query_papers = Vec::new();

            // Paginate through results
            loop {
                if query_papers.len() >= params.max_results_per_query as usize {
                    break;
                }

                let result = ctx
                    .client
                    .search_papers(query, offset, limit, fields::DEFAULT)
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
                    query_papers.push(paper);
                }

                if result.next.is_none() {
                    break;
                }
                offset = result.next.unwrap_or(offset + limit);
            }

            let mut query_new = 0;
            let mut query_duplicate = 0;

            for paper in query_papers {
                if !all_papers.contains_key(&paper.paper_id) {
                    all_papers.insert(paper.paper_id.clone(), paper);
                    query_new += 1;
                } else {
                    query_duplicate += 1;
                }
            }

            results_per_query.insert(query.clone(), query_new + query_duplicate);
            search_log.push(json!({
                "query": query,
                "retrieved": query_new + query_duplicate,
                "new_unique": query_new,
                "duplicates": query_duplicate,
                "timestamp": Utc::now().to_rfc3339()
            }));
        }

        let paper_list: Vec<_> = all_papers.into_values().collect();
        let total_before_dedup: usize = results_per_query.values().sum();
        let duplicates_removed = total_before_dedup - paper_list.len();

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# PRISMA Search Results\n\n\
                     **Queries run:** {}\n\
                     **Total before deduplication:** {}\n\
                     **Duplicates removed:** {}\n\
                     **Unique papers:** {}\n\n\
                     ## Search Log\n\n",
                    params.queries.len(),
                    total_before_dedup,
                    duplicates_removed,
                    paper_list.len()
                );

                for log in &search_log {
                    let q = log["query"].as_str().unwrap_or("");
                    let r = log["retrieved"].as_i64().unwrap_or(0);
                    let n = log["new_unique"].as_i64().unwrap_or(0);
                    output.push_str(&format!("- **{}**: {} retrieved, {} new\n", q, r, n));
                }

                output.push_str("\n---\n\n## Papers\n\n");
                let display_papers: Vec<_> = paper_list.iter().take(100).collect();
                output.push_str(&formatters::format_papers_markdown(&display_papers.into_iter().cloned().collect::<Vec<_>>()));

                if paper_list.len() > 100 {
                    output.push_str(&format!("\n*... and {} more papers*", paper_list.len() - 100));
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "queries": params.queries,
                "results_per_query": results_per_query,
                "total_before_dedup": total_before_dedup,
                "duplicates_removed": duplicates_removed,
                "total_unique": paper_list.len(),
                "search_log": search_log,
                "papers": paper_list.iter().map(formatters::compact_paper).collect::<Vec<_>>()
            }))?),
        }
    }
}

/// Screening export tool.
pub struct ScreeningExportTool;

#[async_trait::async_trait]
impl McpTool for ScreeningExportTool {
    fn name(&self) -> &'static str {
        "screening_export"
    }

    fn description(&self) -> &'static str {
        "Export papers in a format optimized for title/abstract screening."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to export"
                },
                "includeAbstract": {
                    "type": "boolean",
                    "default": true
                },
                "includeTldr": {
                    "type": "boolean",
                    "default": false
                }
            },
            "required": ["paperIds"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: ScreeningExportInput = serde_json::from_value(input)?;

        let mut field_list = vec![
            "paperId", "title", "year", "citationCount", "authors", "venue", "externalIds",
        ];
        if params.include_abstract {
            field_list.push("abstract");
        }
        if params.include_tldr {
            field_list.push("tldr");
        }

        let papers = ctx
            .client
            .get_papers_batch(&params.paper_ids, &field_list)
            .await
            .map_err(ToolError::from)?;

        let mut export_data = Vec::new();

        for paper in papers {
            let authors: Vec<_> = paper
                .authors
                .iter()
                .take(5)
                .map(|a| a.name_or_default().to_string())
                .collect();
            let mut author_str = authors.join("; ");
            if paper.authors.len() > 5 {
                author_str.push_str(&format!(" et al. (+{})", paper.authors.len() - 5));
            }

            let mut row = json!({
                "paperId": paper.paper_id,
                "title": paper.title_or_default(),
                "authors": author_str,
                "year": paper.year,
                "venue": paper.venue,
                "citations": paper.citations(),
                "doi": paper.doi(),
                "arxiv": paper.arxiv_id(),
            });

            if params.include_abstract {
                row["abstract"] = json!(paper.r#abstract.as_deref().unwrap_or(""));
            }

            if params.include_tldr {
                row["tldr"] = json!(paper.tldr_text().unwrap_or(""));
            }

            export_data.push(row);
        }

        Ok(serde_json::to_string(&json!({
            "total": export_data.len(),
            "export_date": Utc::now().to_rfc3339(),
            "papers": export_data
        }))?)
    }
}

/// PRISMA flow diagram generator.
pub struct PrismaFlowDiagramTool;

#[async_trait::async_trait]
impl McpTool for PrismaFlowDiagramTool {
    fn name(&self) -> &'static str {
        "prisma_flow_diagram"
    }

    fn description(&self) -> &'static str {
        "Generate a PRISMA 2020 flow diagram data structure for systematic reviews."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "identification": {
                    "type": "object",
                    "properties": {
                        "databases": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "name": {"type": "string"},
                                    "results": {"type": "integer"}
                                }
                            }
                        }
                    },
                    "required": ["databases"]
                },
                "screening": {
                    "type": "object",
                    "properties": {
                        "records_after_dedup": {"type": "integer"},
                        "records_screened": {"type": "integer"},
                        "records_excluded": {"type": "integer"}
                    },
                    "required": ["records_after_dedup", "records_screened", "records_excluded"]
                },
                "eligibility": {
                    "type": "object",
                    "properties": {
                        "reports_sought": {"type": "integer"},
                        "reports_assessed": {"type": "integer"},
                        "reports_excluded": {"type": "integer"}
                    }
                },
                "included": {
                    "type": "object",
                    "properties": {
                        "studies_included": {"type": "integer"},
                        "reports_included": {"type": "integer"}
                    }
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["identification", "screening"]
        })
    }

    async fn execute(&self, _ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: PrismaFlowDiagramInput = serde_json::from_value(input)?;

        // Calculate identification totals
        let db_records: i32 = params.identification.databases.iter().map(|d| d.results).sum();
        let other_records: i32 = params
            .identification
            .other_sources
            .as_ref()
            .map(|s| s.iter().map(|o| o.records).sum())
            .unwrap_or(0);
        let total_records = db_records + other_records;

        let duplicates_removed = total_records - params.screening.records_after_dedup;

        let mut data = json!({
            "version": "PRISMA 2020",
            "identification": {
                "databases_searched": params.identification.databases.len(),
                "records_from_databases": db_records,
                "records_from_other": other_records,
                "total_records": total_records
            },
            "screening": {
                "duplicates_removed": duplicates_removed,
                "after_duplicates": params.screening.records_after_dedup,
                "records_screened": params.screening.records_screened,
                "records_excluded": params.screening.records_excluded,
                "exclusion_reasons": params.screening.exclusion_reasons
            }
        });

        if let Some(ref elig) = params.eligibility {
            data["eligibility"] = json!({
                "reports_sought": elig.reports_sought,
                "reports_not_retrieved": elig.reports_not_retrieved,
                "reports_assessed": elig.reports_assessed,
                "reports_excluded": elig.reports_excluded,
                "exclusion_reasons": elig.exclusion_reasons
            });
        }

        if let Some(ref incl) = params.included {
            data["included"] = json!({
                "studies_included": incl.studies_included,
                "reports_included": incl.reports_included
            });
        }

        // Summary
        let mut summary = json!({
            "total_identified": total_records,
            "duplicates_removed": duplicates_removed,
            "screened": params.screening.records_screened,
            "excluded_at_screening": params.screening.records_excluded
        });

        if let Some(ref elig) = params.eligibility {
            summary["excluded_at_eligibility"] = json!(elig.reports_excluded);
        }

        if let Some(ref incl) = params.included {
            summary["final_included"] = json!(incl.studies_included);
            if total_records > 0 {
                let rate = (incl.studies_included as f64 / total_records as f64) * 100.0;
                summary["inclusion_rate_percent"] = json!(format!("{:.2}", rate));
            }
        }

        data["summary"] = summary;

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = String::from("# PRISMA 2020 Flow Diagram\n\n");
                output.push_str(&generate_ascii_diagram(&data));
                output.push_str("\n\n## Summary Statistics\n\n");

                if let Some(summary) = data.get("summary").and_then(|s| s.as_object()) {
                    for (key, value) in summary {
                        let label = key.replace('_', " ");
                        output.push_str(&format!("- **{}:** {}\n", label, value));
                    }
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&data)?),
        }
    }
}

fn generate_ascii_diagram(data: &serde_json::Value) -> String {
    let mut lines = Vec::new();
    lines.push("=".repeat(70));
    lines.push("                    PRISMA 2020 FLOW DIAGRAM".to_string());
    lines.push("=".repeat(70));
    lines.push(String::new());

    // Identification
    if let Some(ident) = data.get("identification") {
        lines.push("IDENTIFICATION".to_string());
        lines.push("-".repeat(40));
        if let Some(db) = ident.get("records_from_databases") {
            lines.push(format!("  Records from databases: {}", db));
        }
        if let Some(other) = ident.get("records_from_other") {
            if other.as_i64().unwrap_or(0) > 0 {
                lines.push(format!("  Records from other sources: {}", other));
            }
        }
        if let Some(total) = ident.get("total_records") {
            lines.push(format!("  TOTAL: {}", total));
        }
        lines.push(String::new());
    }

    // Screening
    if let Some(screen) = data.get("screening") {
        lines.push("SCREENING".to_string());
        lines.push("-".repeat(40));
        if let Some(after) = screen.get("after_duplicates") {
            lines.push(format!("  After duplicates removed: {}", after));
        }
        if let Some(screened) = screen.get("records_screened") {
            lines.push(format!("  Records screened: {}", screened));
        }
        if let Some(excluded) = screen.get("records_excluded") {
            lines.push(format!("  Records excluded: {}", excluded));
        }
        lines.push(String::new());
    }

    // Eligibility
    if let Some(elig) = data.get("eligibility") {
        lines.push("ELIGIBILITY".to_string());
        lines.push("-".repeat(40));
        if let Some(sought) = elig.get("reports_sought") {
            lines.push(format!("  Reports sought: {}", sought));
        }
        if let Some(assessed) = elig.get("reports_assessed") {
            lines.push(format!("  Reports assessed: {}", assessed));
        }
        if let Some(excluded) = elig.get("reports_excluded") {
            lines.push(format!("  Reports excluded: {}", excluded));
        }
        lines.push(String::new());
    }

    // Included
    if let Some(incl) = data.get("included") {
        lines.push("INCLUDED".to_string());
        lines.push("-".repeat(40));
        if let Some(studies) = incl.get("studies_included") {
            lines.push(format!("  Studies included: {}", studies));
        }
        if let Some(reports) = incl.get("reports_included") {
            lines.push(format!("  Reports included: {}", reports));
        }
        lines.push(String::new());
    }

    lines.push("=".repeat(70));
    lines.join("\n")
}
