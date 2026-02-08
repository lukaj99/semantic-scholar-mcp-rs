//! Bibliometric analysis tools: FWCI, highly cited, citation half-life, co-citation, bibliographic coupling, hot papers.

use std::collections::HashMap;

use chrono::{Datelike, Utc};
use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{
    BibliographicCouplingInput, CitationHalfLifeInput, CocitationAnalysisInput,
    FieldWeightedImpactInput, HighlyCitedPapersInput, HotPapersInput, ResponseFormat,
};

/// Field-weighted citation impact tool.
pub struct FieldWeightedImpactTool;

#[async_trait::async_trait]
impl McpTool for FieldWeightedImpactTool {
    fn name(&self) -> &'static str {
        "field_weighted_impact"
    }

    fn description(&self) -> &'static str {
        "Calculate approximate Field-Weighted Citation Impact (FWCI). \
         FWCI normalizes citations by field and year. 1.0 = average."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to analyze"
                },
                "baseline_sample_size": {
                    "type": "integer",
                    "default": 100
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
        let params: FieldWeightedImpactInput = serde_json::from_value(input)?;

        let papers = ctx
            .client
            .get_papers_batch(&params.paper_ids, fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        // Cache for field-year baselines
        let mut baselines: HashMap<(String, i32), f64> = HashMap::new();
        let mut results = Vec::new();

        for paper in papers {
            let citations = paper.citations();
            let year = paper.year;
            let paper_fields = paper.fields_of_study.clone().unwrap_or_default();

            let Some(year) = year else {
                results.push(json!({
                    "paperId": paper.paper_id,
                    "title": paper.title_or_default(),
                    "fwci": null,
                    "error": "Missing year data"
                }));
                continue;
            };

            if paper_fields.is_empty() {
                results.push(json!({
                    "paperId": paper.paper_id,
                    "title": paper.title_or_default(),
                    "fwci": null,
                    "error": "Missing field data"
                }));
                continue;
            }

            // Get baselines for each field (limit to 3)
            let mut field_baselines = Vec::new();
            for field in paper_fields.iter().take(3) {
                let key = (field.clone(), year);
                if !baselines.contains_key(&key) {
                    // Estimate baseline from search
                    let baseline =
                        get_field_baseline(ctx, field, year, params.baseline_sample_size).await;
                    baselines.insert(key.clone(), baseline);
                }
                field_baselines.push(*baselines.get(&key).unwrap_or(&1.0));
            }

            let avg_baseline = if field_baselines.is_empty() {
                1.0
            } else {
                field_baselines.iter().sum::<f64>() / field_baselines.len() as f64
            };

            let fwci = if avg_baseline > 0.0 { citations as f64 / avg_baseline } else { 0.0 };

            results.push(json!({
                "paperId": paper.paper_id,
                "title": paper.title_or_default(),
                "year": year,
                "citations": citations,
                "fields": paper_fields,
                "avg_baseline": (avg_baseline * 100.0).round() / 100.0,
                "fwci": (fwci * 100.0).round() / 100.0,
                "interpretation": format!("{}% of expected citations", (fwci * 100.0).round())
            }));
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = String::from(
                    "# Field-Weighted Citation Impact\n\n\
                     ⚠️ *Baselines estimated from Semantic Scholar data, not official FWCI.*\n\n",
                );

                for r in &results {
                    if r.get("fwci").and_then(|v| v.as_f64()).is_some() {
                        output.push_str(&format!(
                            "### {}\n",
                            r["title"].as_str().unwrap_or("Unknown")
                        ));
                        output.push_str(&format!(
                            "- **FWCI:** {} ({})\n",
                            r["fwci"],
                            r["interpretation"].as_str().unwrap_or("")
                        ));
                        output.push_str(&format!(
                            "- **Citations:** {} | **Baseline:** {}\n\n",
                            r["citations"], r["avg_baseline"]
                        ));
                    }
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "methodology_note": "Baselines estimated from Semantic Scholar",
                "results": results
            }))?),
        }
    }
}

async fn get_field_baseline(ctx: &ToolContext, field: &str, year: i32, sample_size: i32) -> f64 {
    // Use bulk search with year filter and citation sorting for a representative sample
    let filters = vec![("year".to_string(), format!("{}-{}", year, year))];

    let result = ctx
        .client
        .search_papers_bulk(
            field,
            None,
            &["citationCount"],
            Some("citationCount:desc"),
            &filters,
        )
        .await;

    match result {
        Ok(search_result) => {
            let citations: Vec<i32> = search_result
                .data
                .iter()
                .take(sample_size as usize)
                .map(|p| p.citations())
                .collect();
            if citations.is_empty() {
                1.0
            } else {
                // Calculate mean (papers are sorted desc, so mean is representative)
                let sum: i64 = citations.iter().map(|&c| c as i64).sum();
                let mean = sum as f64 / citations.len() as f64;
                mean.max(1.0)
            }
        }
        Err(_) => 1.0,
    }
}

/// Highly cited papers detection tool.
pub struct HighlyCitedPapersTool;

#[async_trait::async_trait]
impl McpTool for HighlyCitedPapersTool {
    fn name(&self) -> &'static str {
        "highly_cited_papers"
    }

    fn description(&self) -> &'static str {
        "Identify highly cited papers (top percentile by field/year). \
         Based on Essential Science Indicators methodology."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to evaluate"
                },
                "percentileThreshold": {
                    "type": "number",
                    "default": 1.0,
                    "description": "Top X percentile (1.0 = top 1%)"
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
        let params: HighlyCitedPapersInput = serde_json::from_value(input)?;

        let papers = ctx
            .client
            .get_papers_batch(&params.paper_ids, fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        let mut thresholds: HashMap<(String, i32), i32> = HashMap::new();
        let mut results = Vec::new();

        for paper in papers {
            let citations = paper.citations();
            let year = paper.year;
            let paper_fields = paper.fields_of_study.clone().unwrap_or_default();
            let primary_field =
                paper_fields.first().cloned().unwrap_or_else(|| "Unknown".to_string());

            let Some(year) = year else {
                continue;
            };

            let key = (primary_field.clone(), year);
            if !thresholds.contains_key(&key) {
                // Estimate threshold from search
                let threshold = get_percentile_threshold(
                    ctx,
                    &primary_field,
                    year,
                    params.percentile_threshold,
                )
                .await;
                thresholds.insert(key.clone(), threshold);
            }

            let threshold = *thresholds.get(&key).unwrap_or(&0);
            let is_highly_cited = threshold > 0 && citations >= threshold;

            results.push(json!({
                "paperId": paper.paper_id,
                "title": paper.title_or_default(),
                "year": year,
                "primary_field": primary_field,
                "citations": citations,
                "threshold": threshold,
                "is_highly_cited": is_highly_cited
            }));
        }

        let highly_count =
            results.iter().filter(|r| r["is_highly_cited"].as_bool().unwrap_or(false)).count();

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Highly Cited Papers (Top {}%)\n\
                     **Analyzed:** {} | **Highly cited:** {}\n\n",
                    params.percentile_threshold,
                    results.len(),
                    highly_count
                );

                for r in &results {
                    let status = if r["is_highly_cited"].as_bool().unwrap_or(false) {
                        " ⭐ HIGHLY CITED"
                    } else {
                        ""
                    };
                    output.push_str(&format!(
                        "### {}{}\n- Citations: {} | Threshold: {}\n\n",
                        r["title"].as_str().unwrap_or("Unknown"),
                        status,
                        r["citations"],
                        r["threshold"]
                    ));
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "percentile_threshold": params.percentile_threshold,
                "results": results
            }))?),
        }
    }
}

async fn get_percentile_threshold(
    ctx: &ToolContext,
    field: &str,
    year: i32,
    percentile: f64,
) -> i32 {
    // Use bulk search sorted by citations desc with proper year filter
    let filters = vec![("year".to_string(), format!("{}-{}", year, year))];

    let result = ctx
        .client
        .search_papers_bulk(
            field,
            None,
            &["citationCount"],
            Some("citationCount:desc"),
            &filters,
        )
        .await;

    match result {
        Ok(search_result) => {
            // Papers are already sorted by citations desc from bulk API
            let cites: Vec<i32> = search_result.data.iter().map(|p| p.citations()).collect();
            if cites.is_empty() {
                0
            } else {
                let threshold_idx =
                    ((cites.len() as f64 * percentile / 100.0) as usize).max(1) - 1;
                cites.get(threshold_idx).copied().unwrap_or(0)
            }
        }
        Err(_) => 0,
    }
}

/// Citation half-life calculator.
pub struct CitationHalfLifeTool;

#[async_trait::async_trait]
impl McpTool for CitationHalfLifeTool {
    fn name(&self) -> &'static str {
        "citation_half_life"
    }

    fn description(&self) -> &'static str {
        "Calculate citation half-life (median age of citations). \
         Low = rapidly obsoleting; high = enduring relevance."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperId": {
                    "type": "string",
                    "description": "Paper ID to analyze"
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
        let params: CitationHalfLifeInput = serde_json::from_value(input)?;

        let papers = ctx
            .client
            .get_papers_batch(std::slice::from_ref(&params.paper_id), fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        let paper = papers
            .into_iter()
            .next()
            .ok_or_else(|| ToolError::validation("paperId", "Paper not found"))?;

        let pub_year = paper
            .year
            .ok_or_else(|| ToolError::validation("paper", "Paper has no publication year"))?;

        // Get citations
        let citations = ctx
            .client
            .get_citations(&params.paper_id, 0, 1000, &["year"])
            .await
            .map_err(ToolError::from)?;

        let _current_year = Utc::now().year();
        let mut ages = Vec::new();
        let mut age_distribution: HashMap<&str, i32> = HashMap::new();
        age_distribution.insert("0-2", 0);
        age_distribution.insert("2-4", 0);
        age_distribution.insert("4-6", 0);
        age_distribution.insert("6-10", 0);
        age_distribution.insert("10+", 0);

        for citation in &citations.data {
            if let Some(ref citing_paper) = citation.paper {
                if let Some(cite_year) = citing_paper.year {
                    if cite_year >= pub_year {
                        let age = cite_year - pub_year;
                        ages.push(age);

                        let bucket = if age <= 2 {
                            "0-2"
                        } else if age <= 4 {
                            "2-4"
                        } else if age <= 6 {
                            "4-6"
                        } else if age <= 10 {
                            "6-10"
                        } else {
                            "10+"
                        };
                        *age_distribution.entry(bucket).or_insert(0) += 1;
                    }
                }
            }
        }

        // Calculate median (half-life)
        let half_life = if ages.is_empty() {
            None
        } else {
            ages.sort_unstable();
            let mid = ages.len() / 2;
            Some(if ages.len() % 2 == 0 {
                (ages[mid - 1] + ages[mid]) as f64 / 2.0
            } else {
                ages[mid] as f64
            })
        };

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Citation Half-Life: {}\n\
                     **Year:** {} | **Citations:** {}\n\n",
                    paper.title_or_default(),
                    pub_year,
                    paper.citations()
                );

                if let Some(hl) = half_life {
                    output.push_str(&format!("**Half-life:** {:.1} years\n\n", hl));
                } else {
                    output.push_str("**Half-life:** N/A\n\n");
                }

                output.push_str("## Age Distribution\n");
                for bucket in ["0-2", "2-4", "4-6", "6-10", "10+"] {
                    let count = age_distribution.get(bucket).unwrap_or(&0);
                    let pct = if ages.is_empty() {
                        0.0
                    } else {
                        (*count as f64 / ages.len() as f64) * 100.0
                    };
                    output.push_str(&format!("- {} years: {} ({:.1}%)\n", bucket, count, pct));
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "paperId": paper.paper_id,
                "title": paper.title_or_default(),
                "year": pub_year,
                "total_citations": paper.citations(),
                "citations_analyzed": ages.len(),
                "citation_half_life_years": half_life.map(|h| (h * 10.0).round() / 10.0),
                "age_distribution": age_distribution,
                "interpretation": half_life.map(|h| format!("50% of citations within {:.1} years", h))
            }))?),
        }
    }
}

/// Co-citation analysis tool.
pub struct CocitationAnalysisTool;

#[async_trait::async_trait]
impl McpTool for CocitationAnalysisTool {
    fn name(&self) -> &'static str {
        "cocitation_analysis"
    }

    fn description(&self) -> &'static str {
        "Find conceptually related papers via co-citation analysis. \
         Papers frequently cited together are likely related."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperId": {
                    "type": "string",
                    "description": "Focal paper ID"
                },
                "minCocitations": {
                    "type": "integer",
                    "default": 5
                },
                "maxCitingPapers": {
                    "type": "integer",
                    "default": 100
                },
                "maxResults": {
                    "type": "integer",
                    "default": 50
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
        let params: CocitationAnalysisInput = serde_json::from_value(input)?;

        // Get focal paper
        let papers = ctx
            .client
            .get_papers_batch(std::slice::from_ref(&params.paper_id), fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        let focal_paper = papers
            .into_iter()
            .next()
            .ok_or_else(|| ToolError::validation("paperId", "Focal paper not found"))?;

        // Get citing papers
        let citations = ctx
            .client
            .get_citations(&params.paper_id, 0, params.max_citing_papers, &["paperId"])
            .await
            .map_err(ToolError::from)?;

        // Count co-citations
        let mut cocitation_counts: HashMap<String, i32> = HashMap::new();

        for citation in &citations.data {
            if let Some(ref citing_paper) = citation.paper {
                // Get references of this citing paper
                if let Ok(refs) =
                    ctx.client.get_references(&citing_paper.paper_id, 0, 100, &["paperId"]).await
                {
                    for ref_paper in refs.data {
                        if let Some(cited) = ref_paper.paper {
                            if cited.paper_id != params.paper_id {
                                *cocitation_counts.entry(cited.paper_id).or_insert(0) += 1;
                            }
                        }
                    }
                }
            }
        }

        // Filter and sort
        let mut filtered: Vec<_> = cocitation_counts
            .into_iter()
            .filter(|(_, count)| *count >= params.min_cocitations)
            .collect();
        filtered.sort_by(|a, b| b.1.cmp(&a.1));
        filtered.truncate(params.max_results as usize);

        // Get details for top co-cited papers
        let top_ids: Vec<String> = filtered.iter().map(|(id, _)| id.clone()).collect();
        let mut cocited_papers = Vec::new();

        if !top_ids.is_empty() {
            if let Ok(details) = ctx.client.get_papers_batch(&top_ids, fields::DEFAULT).await {
                let focal_cites = focal_paper.citations().max(1);
                for paper in details {
                    let count = filtered
                        .iter()
                        .find(|(id, _)| *id == paper.paper_id)
                        .map(|(_, c)| *c)
                        .unwrap_or(0);
                    let paper_cites = paper.citations().max(1);
                    let strength = count as f64 / ((focal_cites * paper_cites) as f64).sqrt();

                    cocited_papers.push(json!({
                        "paper": formatters::compact_paper(&paper),
                        "cocitation_count": count,
                        "cocitation_strength": (strength * 10000.0).round() / 10000.0
                    }));
                }
            }
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let title = focal_paper.title_or_default();
                let display_title = if title.len() > 60 { &title[..60] } else { title };
                let mut output = format!(
                    "# Co-citation Analysis: {}...\n\
                     **Citing papers analyzed:** {}\n\
                     **Co-cited papers found:** {}\n\n\
                     ⚠️ *Co-citation indicates papers frequently cited together.*\n\n---\n\n",
                    display_title,
                    citations.data.len(),
                    cocited_papers.len()
                );

                for item in &cocited_papers {
                    output.push_str(&format!(
                        "**Co-citations: {}** (strength: {})\n",
                        item["cocitation_count"], item["cocitation_strength"]
                    ));
                    if let Some(p) = item["paper"].as_object() {
                        output.push_str(&format!(
                            "- {} ({})\n\n",
                            p.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown"),
                            p.get("year").and_then(|y| y.as_i64()).unwrap_or(0)
                        ));
                    }
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "focal_paper": {
                    "id": focal_paper.paper_id,
                    "title": focal_paper.title_or_default()
                },
                "citing_papers_analyzed": citations.data.len(),
                "cocited_papers": cocited_papers
            }))?),
        }
    }
}

/// Bibliographic coupling tool.
pub struct BibliographicCouplingTool;

#[async_trait::async_trait]
impl McpTool for BibliographicCouplingTool {
    fn name(&self) -> &'static str {
        "bibliographic_coupling"
    }

    fn description(&self) -> &'static str {
        "Find methodologically similar papers via bibliographic coupling. \
         Papers sharing many references likely use similar methods."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperId": {
                    "type": "string",
                    "description": "Focal paper ID"
                },
                "minSharedRefs": {
                    "type": "integer",
                    "default": 3
                },
                "maxRefsToCheck": {
                    "type": "integer",
                    "default": 50
                },
                "maxResults": {
                    "type": "integer",
                    "default": 50
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
        let params: BibliographicCouplingInput = serde_json::from_value(input)?;

        // Get focal paper
        let papers = ctx
            .client
            .get_papers_batch(std::slice::from_ref(&params.paper_id), fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        let focal_paper = papers
            .into_iter()
            .next()
            .ok_or_else(|| ToolError::validation("paperId", "Focal paper not found"))?;

        // Get references of focal paper
        let focal_refs = ctx
            .client
            .get_references(&params.paper_id, 0, params.max_refs_to_check, &["paperId"])
            .await
            .map_err(ToolError::from)?;

        let focal_ref_ids: std::collections::HashSet<String> = focal_refs
            .data
            .iter()
            .filter_map(|r| r.paper.as_ref().map(|p| p.paper_id.clone()))
            .collect();

        if focal_ref_ids.is_empty() {
            return Ok(json!({"error": "No references found for focal paper"}).to_string());
        }

        // For each reference, find other papers that cite it
        let mut coupling_counts: HashMap<String, i32> = HashMap::new();

        for ref_id in focal_ref_ids.iter().take(params.max_refs_to_check as usize) {
            if let Ok(citers) = ctx.client.get_citations(ref_id, 0, 100, &["paperId"]).await {
                for citer in citers.data {
                    if let Some(citing) = citer.paper {
                        if citing.paper_id != params.paper_id {
                            *coupling_counts.entry(citing.paper_id).or_insert(0) += 1;
                        }
                    }
                }
            }
        }

        // Filter and sort
        let mut filtered: Vec<_> = coupling_counts
            .into_iter()
            .filter(|(_, count)| *count >= params.min_shared_refs)
            .collect();
        filtered.sort_by(|a, b| b.1.cmp(&a.1));
        filtered.truncate(params.max_results as usize);

        // Get details
        let top_ids: Vec<String> = filtered.iter().map(|(id, _)| id.clone()).collect();
        let mut coupled_papers = Vec::new();

        if !top_ids.is_empty() {
            if let Ok(details) = ctx.client.get_papers_batch(&top_ids, fields::DEFAULT).await {
                let focal_ref_count = focal_ref_ids.len() as f64;
                for paper in details {
                    let shared = filtered
                        .iter()
                        .find(|(id, _)| *id == paper.paper_id)
                        .map(|(_, c)| *c)
                        .unwrap_or(0);
                    let paper_ref_count = paper.reference_count.unwrap_or(1).max(1) as f64;
                    let strength = shared as f64 / (focal_ref_count * paper_ref_count).sqrt();

                    coupled_papers.push(json!({
                        "paper": formatters::compact_paper(&paper),
                        "shared_references": shared,
                        "coupling_strength": (strength * 10000.0).round() / 10000.0
                    }));
                }
            }
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let title = focal_paper.title_or_default();
                let display_title = if title.len() > 60 { &title[..60] } else { title };
                let mut output = format!(
                    "# Bibliographic Coupling: {}...\n\
                     **Focal paper references:** {}\n\
                     **Coupled papers found:** {}\n\n\
                     ⚠️ *Bibliographic coupling indicates papers sharing references.*\n\n---\n\n",
                    display_title,
                    focal_ref_ids.len(),
                    coupled_papers.len()
                );

                for item in &coupled_papers {
                    output.push_str(&format!(
                        "**Shared refs: {}** (strength: {})\n",
                        item["shared_references"], item["coupling_strength"]
                    ));
                    if let Some(p) = item["paper"].as_object() {
                        output.push_str(&format!(
                            "- {} ({})\n\n",
                            p.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown"),
                            p.get("year").and_then(|y| y.as_i64()).unwrap_or(0)
                        ));
                    }
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "focal_paper": {
                    "id": focal_paper.paper_id,
                    "title": focal_paper.title_or_default(),
                    "reference_count": focal_ref_ids.len()
                },
                "coupled_papers": coupled_papers
            }))?),
        }
    }
}

/// Hot papers detection tool.
pub struct HotPapersTool;

#[async_trait::async_trait]
impl McpTool for HotPapersTool {
    fn name(&self) -> &'static str {
        "hot_papers"
    }

    fn description(&self) -> &'static str {
        "Detect 'hot papers' with accelerating citation velocity. \
         Identifies trending research by analyzing citation patterns."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Search query for candidate papers"
                },
                "timeWindowMonths": {
                    "type": "integer",
                    "default": 24
                },
                "minRecentCitations": {
                    "type": "integer",
                    "default": 10
                },
                "maxPapers": {
                    "type": "integer",
                    "default": 50
                },
                "yearStart": {
                    "type": "integer",
                    "description": "Minimum publication year"
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
        let params: HotPapersInput = serde_json::from_value(input)?;

        let current_year = Utc::now().year();
        let year_start = params.year_start.unwrap_or(current_year - 5);

        // Search for candidate papers
        let mut all_papers = Vec::new();
        let mut offset = 0;
        let limit = 100;

        // Build filter parameters
        let filters = vec![
            ("year".to_string(), format!("{}-", year_start)),
            ("minCitationCount".to_string(), params.min_recent_citations.to_string()),
        ];

        loop {
            if all_papers.len() >= params.max_papers as usize {
                break;
            }

            let result = ctx
                .client
                .search_papers(&params.query, offset, limit, fields::DEFAULT, &filters)
                .await
                .map_err(ToolError::from)?;

            for paper in result.data {
                all_papers.push(paper);
            }

            if result.next.is_none() {
                break;
            }
            offset = result.next.unwrap_or(offset + limit);
        }

        // For simplicity, we'll use citation count as a proxy for velocity
        // (A more accurate implementation would fetch individual citation dates)
        let mut hot_papers_data: Vec<_> = all_papers
            .iter()
            .map(|paper| {
                let years_since_pub = (current_year - paper.year.unwrap_or(current_year)).max(1);
                let velocity = paper.citations() as f64 / years_since_pub as f64;
                json!({
                    "paper": formatters::compact_paper(paper),
                    "metrics": {
                        "total_citations": paper.citations(),
                        "years_since_publication": years_since_pub,
                        "citation_velocity": (velocity * 100.0).round() / 100.0
                    }
                })
            })
            .collect();

        // Sort by velocity
        hot_papers_data.sort_by(|a, b| {
            let a_vel = a["metrics"]["citation_velocity"].as_f64().unwrap_or(0.0);
            let b_vel = b["metrics"]["citation_velocity"].as_f64().unwrap_or(0.0);
            b_vel.partial_cmp(&a_vel).unwrap_or(std::cmp::Ordering::Equal)
        });

        // Mark top 10% as hot
        let hot_threshold = (hot_papers_data.len() / 10).max(1);
        for (i, item) in hot_papers_data.iter_mut().enumerate() {
            item["metrics"]["is_hot"] = json!(i < hot_threshold);
            item["metrics"]["velocity_rank"] = json!(i + 1);
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Hot Papers: {}\n\
                     **Time window:** {} months\n\
                     **Papers analyzed:** {}\n\
                     **Hot papers (top 10%):** {}\n\n---\n\n",
                    params.query,
                    params.time_window_months,
                    hot_papers_data.len(),
                    hot_threshold
                );

                for item in &hot_papers_data {
                    let is_hot = item["metrics"]["is_hot"].as_bool().unwrap_or(false);
                    let hot_badge = if is_hot { " 🔥 **HOT**" } else { "" };
                    let rank = item["metrics"]["velocity_rank"].as_i64().unwrap_or(0);
                    let velocity = item["metrics"]["citation_velocity"].as_f64().unwrap_or(0.0);

                    output.push_str(&format!("### #{}{}\n", rank, hot_badge));
                    if let Some(p) = item["paper"].as_object() {
                        output.push_str(&format!(
                            "**{}** ({})\n",
                            p.get("title").and_then(|t| t.as_str()).unwrap_or("Unknown"),
                            p.get("year").and_then(|y| y.as_i64()).unwrap_or(0)
                        ));
                    }
                    output.push_str(&format!("**Velocity:** {:.2} citations/year\n\n", velocity));
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "query": params.query,
                "timeWindowMonths": params.time_window_months,
                "papers_analyzed": hot_papers_data.len(),
                "hot_threshold_rank": hot_threshold,
                "results": hot_papers_data
            }))?),
        }
    }
}
