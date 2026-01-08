//! Advanced tools: pearl_growing, orcid_author_lookup.

use std::collections::{HashMap, HashSet};

use regex::Regex;
use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{OrcidAuthorLookupInput, PearlGrowingInput, PearlGrowingStrategy, ResponseFormat};

/// Pearl growing search expansion tool.
pub struct PearlGrowingTool;

#[async_trait::async_trait]
impl McpTool for PearlGrowingTool {
    fn name(&self) -> &'static str {
        "pearl_growing"
    }

    fn description(&self) -> &'static str {
        "Expand literature search using pearl growing methodology. \
         Starts with seed papers and iteratively discovers related papers \
         through keywords, authors, and citations."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "seed_paper_ids": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Initial seed paper IDs"
                },
                "iterations": {
                    "type": "integer",
                    "default": 2,
                    "minimum": 1,
                    "maximum": 3
                },
                "strategy": {
                    "type": "string",
                    "enum": ["keywords", "authors", "citations", "all"],
                    "default": "all"
                },
                "max_papers_per_iteration": {
                    "type": "integer",
                    "default": 50
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
        let params: PearlGrowingInput = serde_json::from_value(input)?;

        // Fetch seed papers
        let seed_results = ctx
            .client
            .get_papers_batch(&params.seed_paper_ids, fields::EXTENDED)
            .await
            .map_err(ToolError::from)?;

        if seed_results.is_empty() {
            return match params.response_format {
                ResponseFormat::Markdown => {
                    Ok("# Pearl Growing\n\n**Error:** No valid seed papers found.".to_string())
                }
                ResponseFormat::Json => {
                    Ok(json!({"error": "No valid seed papers found", "seeds": []}).to_string())
                }
            };
        }

        let mut all_papers: HashMap<String, crate::models::Paper> = HashMap::new();
        for paper in &seed_results {
            all_papers.insert(paper.paper_id.clone(), paper.clone());
        }

        let mut growth_log = Vec::new();

        for iteration in 0..params.iterations {
            let mut iteration_papers: HashMap<String, crate::models::Paper> = HashMap::new();
            let current_seeds: Vec<_> = all_papers.values().cloned().collect();

            // Strategy: Keywords
            if matches!(params.strategy, PearlGrowingStrategy::Keywords | PearlGrowingStrategy::All)
            {
                let keywords = extract_keywords(&current_seeds);
                if !keywords.is_empty() {
                    let query = keywords[..keywords.len().min(5)].join(" ");
                    if let Ok(search_result) = ctx
                        .client
                        .search_papers(&query, 0, params.max_papers_per_iteration, fields::DEFAULT)
                        .await
                    {
                        let mut new_count = 0;
                        for paper in search_result.data {
                            if !all_papers.contains_key(&paper.paper_id)
                                && !iteration_papers.contains_key(&paper.paper_id)
                            {
                                iteration_papers.insert(paper.paper_id.clone(), paper);
                                new_count += 1;
                            }
                        }

                        growth_log.push(json!({
                            "iteration": iteration + 1,
                            "strategy": "keywords",
                            "query": query,
                            "new": new_count
                        }));
                    }
                }
            }

            // Strategy: Authors
            if matches!(params.strategy, PearlGrowingStrategy::Authors | PearlGrowingStrategy::All)
            {
                let author_ids = extract_author_ids(&current_seeds);
                for author_id in author_ids.iter().take(3) {
                    if let Ok(search_result) = ctx
                        .client
                        .search_papers(
                            &format!("author:{}", author_id),
                            0,
                            20,
                            fields::DEFAULT,
                        )
                        .await
                    {
                        let mut new_count = 0;
                        for paper in search_result.data {
                            if !all_papers.contains_key(&paper.paper_id)
                                && !iteration_papers.contains_key(&paper.paper_id)
                            {
                                iteration_papers.insert(paper.paper_id.clone(), paper);
                                new_count += 1;
                            }
                        }

                        growth_log.push(json!({
                            "iteration": iteration + 1,
                            "strategy": "authors",
                            "author_id": author_id,
                            "new": new_count
                        }));
                    }
                }
            }

            // Strategy: Citations (via recommendations)
            if matches!(
                params.strategy,
                PearlGrowingStrategy::Citations | PearlGrowingStrategy::All
            ) {
                let seed_ids: Vec<String> = current_seeds
                    .iter()
                    .take(5)
                    .map(|p| p.paper_id.clone())
                    .collect();

                if let Ok(recs) = ctx
                    .client
                    .get_recommendations(
                        &seed_ids,
                        None,
                        params.max_papers_per_iteration,
                        fields::DEFAULT,
                    )
                    .await
                {
                    let mut new_count = 0;
                    for paper in recs {
                        if !all_papers.contains_key(&paper.paper_id)
                            && !iteration_papers.contains_key(&paper.paper_id)
                        {
                            iteration_papers.insert(paper.paper_id.clone(), paper);
                            new_count += 1;
                        }
                    }

                    growth_log.push(json!({
                        "iteration": iteration + 1,
                        "strategy": "citations",
                        "seeds_used": seed_ids.len(),
                        "new": new_count
                    }));
                }
            }

            // Add new papers (up to limit, sorted by citations)
            let mut sorted_new: Vec<_> = iteration_papers.into_values().collect();
            sorted_new.sort_by(|a, b| b.citations().cmp(&a.citations()));
            sorted_new.truncate(params.max_papers_per_iteration as usize);

            for paper in sorted_new {
                all_papers.insert(paper.paper_id.clone(), paper);
            }
        }

        // Final results
        let mut final_papers: Vec<_> = all_papers.into_values().collect();
        final_papers.sort_by(|a, b| b.citations().cmp(&a.citations()));

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Pearl Growing Results\n\n\
                     **Seed papers:** {}\n\
                     **Iterations:** {}\n\
                     **Strategy:** {:?}\n\
                     **Total papers found:** {}\n\n\
                     ## Growth Log\n\n",
                    seed_results.len(),
                    params.iterations,
                    params.strategy,
                    final_papers.len()
                );

                for log in &growth_log {
                    output.push_str(&format!(
                        "- Iteration {}, {}: +{} new\n",
                        log["iteration"],
                        log["strategy"].as_str().unwrap_or(""),
                        log["new"]
                    ));
                }

                output.push_str("\n---\n\n## Papers (sorted by citations)\n\n");

                for (i, paper) in final_papers.iter().take(30).enumerate() {
                    output.push_str(&formatters::format_paper_markdown(paper, i + 1));
                }

                if final_papers.len() > 30 {
                    output.push_str(&format!(
                        "\n*... and {} more papers*",
                        final_papers.len() - 30
                    ));
                }

                Ok(output)
            }
            ResponseFormat::Json => {
                let compact: Vec<_> = final_papers.iter().map(formatters::compact_paper).collect();
                Ok(serde_json::to_string(&json!({
                    "seed_papers": params.seed_paper_ids,
                    "iterations": params.iterations,
                    "strategy": format!("{:?}", params.strategy),
                    "total_papers": final_papers.len(),
                    "growth_log": growth_log,
                    "papers": compact
                }))?)
            }
        }
    }
}

fn extract_keywords(papers: &[crate::models::Paper]) -> Vec<String> {
    let stopwords: HashSet<&str> = [
        "the", "a", "an", "and", "or", "but", "in", "on", "at", "to", "for", "of", "with", "by",
        "from", "as", "is", "was", "are", "were", "been", "be", "have", "has", "had", "do", "does",
        "did", "will", "would", "could", "should", "may", "might", "can", "this", "that", "these",
        "those", "it", "its", "we", "our", "their", "using", "based", "via", "new", "method",
        "approach", "study", "paper", "results", "analysis", "data", "model", "models",
    ]
    .into_iter()
    .collect();

    let mut word_counts: HashMap<String, i32> = HashMap::new();

    // Improved regex: matches words with letters, numbers, and hyphens
    // Handles technical terms like "GPT-4", "COVID-19", "BERT", "transformer"
    let word_re = Regex::new(r"\b[a-zA-Z][a-zA-Z0-9-]*[a-zA-Z0-9]\b|\b[a-zA-Z]{2,}\b")
        .expect("valid word regex pattern");

    for paper in papers {
        let text = format!(
            "{} {}",
            paper.title_or_default(),
            paper.r#abstract.as_deref().unwrap_or("")
        );

        for cap in word_re.find_iter(&text.to_lowercase()) {
            let word = cap.as_str();
            // Filter out pure numbers and very short words
            if word.len() >= 3 && !stopwords.contains(word) && !word.chars().all(|c| c.is_ascii_digit()) {
                *word_counts.entry(word.to_string()).or_insert(0) += 1;
            }
        }
    }

    let mut sorted: Vec<_> = word_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(10).map(|(w, _)| w).collect()
}

fn extract_author_ids(papers: &[crate::models::Paper]) -> Vec<String> {
    let mut author_counts: HashMap<String, i32> = HashMap::new();

    for paper in papers {
        for author in &paper.authors {
            if let Some(ref id) = author.author_id {
                *author_counts.entry(id.clone()).or_insert(0) += 1;
            }
        }
    }

    let mut sorted: Vec<_> = author_counts.into_iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1));
    sorted.into_iter().take(5).map(|(id, _)| id).collect()
}

/// ORCID author lookup tool.
pub struct OrcidAuthorLookupTool;

#[async_trait::async_trait]
impl McpTool for OrcidAuthorLookupTool {
    fn name(&self) -> &'static str {
        "orcid_author_lookup"
    }

    fn description(&self) -> &'static str {
        "Look up an author by their ORCID identifier. \
         ORCID provides persistent digital identifier for researchers."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "orcid": {
                    "type": "string",
                    "description": "ORCID iD (e.g., '0000-0002-1825-0097')"
                },
                "include_papers": {
                    "type": "boolean",
                    "default": false
                },
                "max_papers": {
                    "type": "integer",
                    "default": 100
                },
                "response_format": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["orcid"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: OrcidAuthorLookupInput = serde_json::from_value(input)?;

        // Semantic Scholar accepts ORCID with the ORCID: prefix
        let author_id = format!("ORCID:{}", params.orcid);

        // Get author details
        let author = ctx
            .client
            .get_author(&author_id)
            .await
            .map_err(ToolError::from)?;

        // Optionally get papers
        let mut papers = Vec::new();
        if params.include_papers {
            // Search for papers by this author
            let mut offset = 0;
            let limit = 100;

            loop {
                if papers.len() >= params.max_papers as usize {
                    break;
                }

                let result = ctx
                    .client
                    .search_papers(
                        &format!("author:{}", author.author_id),
                        offset,
                        limit,
                        fields::DEFAULT,
                    )
                    .await;

                match result {
                    Ok(search_result) => {
                        papers.extend(search_result.data);
                        if search_result.next.is_none() {
                            break;
                        }
                        offset = search_result.next.unwrap_or(offset + limit);
                    }
                    Err(_) => break,
                }
            }
        }

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# ORCID Lookup: {}\n\n\
                     **Name:** {}\n\
                     **Author ID:** {}\n\
                     **Citations:** {}\n\
                     **h-index:** {}\n\
                     **Papers:** {}\n",
                    params.orcid,
                    author.name_or_default(),
                    author.author_id,
                    author.citations(),
                    author.h_index_value(),
                    author.papers()
                );

                if !author.affiliations.is_empty() {
                    output.push_str(&format!(
                        "**Affiliations:** {}\n",
                        author.affiliations.join(", ")
                    ));
                }

                if params.include_papers && !papers.is_empty() {
                    output.push_str(&format!(
                        "\n## Recent Papers ({} found)\n\n",
                        papers.len()
                    ));
                    for (i, paper) in papers.iter().take(20).enumerate() {
                        output.push_str(&formatters::format_paper_markdown(paper, i + 1));
                    }

                    if papers.len() > 20 {
                        output.push_str(&format!("\n*... and {} more papers*", papers.len() - 20));
                    }
                }

                Ok(output)
            }
            ResponseFormat::Json => Ok(serde_json::to_string(&json!({
                "orcid": params.orcid,
                "found": true,
                "author": {
                    "id": author.author_id,
                    "name": author.name_or_default(),
                    "affiliations": author.affiliations,
                    "citation_count": author.citations(),
                    "h_index": author.h_index_value(),
                    "paper_count": author.papers()
                },
                "papers": if params.include_papers {
                    Some(papers.iter().map(formatters::compact_paper).collect::<Vec<_>>())
                } else {
                    None
                },
                "paper_count": if params.include_papers { Some(papers.len()) } else { None }
            }))?),
        }
    }
}
