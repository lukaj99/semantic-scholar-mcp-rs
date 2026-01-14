//! Discovery tools: exhaustive_search, recommendations, citation_snowball, bulk_boolean_search, snippet_search.

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::formatters;
use crate::models::{
    BulkBooleanSearchInput, CitationSnowballInput, ExhaustiveSearchInput, RecommendationsInput,
    ResponseFormat, SnippetSearchInput,
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
                "yearStart": {
                    "type": "integer",
                    "description": "Minimum publication year"
                },
                "yearEnd": {
                    "type": "integer",
                    "description": "Maximum publication year"
                },
                "fieldsOfStudy": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by fields (e.g., ['Computer Science'])"
                },
                "minCitations": {
                    "type": "integer",
                    "description": "Minimum citation count"
                },
                "openAccessOnly": {
                    "type": "boolean",
                    "default": false
                },
                "maxResults": {
                    "type": "integer",
                    "default": 100,
                    "description": "Maximum papers to return (-1 for unlimited)"
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

        #[allow(clippy::cast_possible_wrap)]
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
                "positivePaperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to use as positive examples"
                },
                "negativePaperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Paper IDs to avoid"
                },
                "limit": {
                    "type": "integer",
                    "default": 100
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["positivePaperIds"]
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
                "seedPaperIds": {
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
                "maxPerPaper": {
                    "type": "integer",
                    "default": 100
                },
                "minCitations": {
                    "type": "integer",
                    "default": 0,
                    "minimum": 0,
                    "description": "Minimum citations for included papers"
                },
                "deduplicate": {
                    "type": "boolean",
                    "default": true
                },
                "responseFormat": {
                    "type": "string",
                    "enum": ["markdown", "json"],
                    "default": "markdown"
                }
            },
            "required": ["seedPaperIds"]
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

/// Bulk boolean search tool (up to 10M papers).
pub struct BulkBooleanSearchTool;

#[async_trait::async_trait]
impl McpTool for BulkBooleanSearchTool {
    fn name(&self) -> &'static str {
        "bulk_boolean_search"
    }

    fn description(&self) -> &'static str {
        "Search for papers using boolean query syntax. Supports +term (AND), -term (NOT), \
         |term (OR), \"phrase\", term*, term~N (fuzzy). Can retrieve up to 10M papers."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Boolean query: +term -term |term \"phrase\" term* term~2"
                },
                "fieldsOfStudy": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by fields of study"
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
                "venue": {
                    "type": "string",
                    "description": "Filter by venue name"
                },
                "publicationTypes": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "JournalArticle, Conference, Review, etc."
                },
                "openAccessOnly": {
                    "type": "boolean",
                    "default": false
                },
                "sort": {
                    "type": "string",
                    "description": "Sort: citationCount:desc, publicationDate:asc, paperId:asc"
                },
                "maxResults": {
                    "type": "integer",
                    "default": 1000,
                    "description": "Maximum papers to return"
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
        let params: BulkBooleanSearchInput = serde_json::from_value(input)?;

        // Build filter parameters
        let mut filters: Vec<(String, String)> = Vec::new();

        if let Some(ref fields) = params.fields_of_study {
            filters.push(("fieldsOfStudy".to_string(), fields.join(",")));
        }

        if let Some(min_year) = params.year_start {
            if let Some(max_year) = params.year_end {
                filters.push(("year".to_string(), format!("{}-{}", min_year, max_year)));
            } else {
                filters.push(("year".to_string(), format!("{}-", min_year)));
            }
        } else if let Some(max_year) = params.year_end {
            filters.push(("year".to_string(), format!("-{}", max_year)));
        }

        if let Some(min_cites) = params.min_citations {
            filters.push(("minCitationCount".to_string(), min_cites.to_string()));
        }

        if let Some(ref venue) = params.venue {
            filters.push(("venue".to_string(), venue.clone()));
        }

        if let Some(ref pub_types) = params.publication_types {
            filters.push(("publicationTypes".to_string(), pub_types.join(",")));
        }

        if params.open_access_only {
            filters.push(("openAccessPdf".to_string(), String::new()));
        }

        // Paginate through results
        let mut all_papers = Vec::new();
        let mut token: Option<String> = None;

        loop {
            if all_papers.len() >= params.max_results as usize {
                break;
            }

            let result = ctx
                .client
                .search_papers_bulk(
                    &params.query,
                    token.as_deref(),
                    fields::DEFAULT,
                    params.sort.as_deref(),
                    &filters,
                )
                .await
                .map_err(ToolError::from)?;

            let has_more = result.has_more();
            token = result.token;
            all_papers.extend(result.data);

            if !has_more {
                break;
            }
        }

        // Truncate to max_results
        all_papers.truncate(params.max_results as usize);

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Bulk Boolean Search Results\n\n\
                     **Query:** `{}`\n\
                     **Found:** {} papers\n\n\
                     ⚠️ *Boolean syntax: +required -excluded |optional \"phrase\" wildcard* fuzzy~2*\n\n---\n\n",
                    params.query,
                    all_papers.len()
                );
                output.push_str(&formatters::format_papers_markdown(&all_papers));
                Ok(output)
            }
            ResponseFormat::Json => {
                let compact = all_papers
                    .iter()
                    .map(formatters::compact_paper)
                    .collect::<Vec<_>>();
                Ok(serde_json::to_string_pretty(&json!({
                    "query": params.query,
                    "total": all_papers.len(),
                    "papers": compact
                }))?)
            }
        }
    }
}

/// Snippet search tool (full-text search with highlights).
pub struct SnippetSearchTool;

#[async_trait::async_trait]
impl McpTool for SnippetSearchTool {
    fn name(&self) -> &'static str {
        "snippet_search"
    }

    fn description(&self) -> &'static str {
        "Search for text snippets within paper titles, abstracts, and body text. \
         Returns highlighted excerpts matching your query - useful for finding specific claims or methods."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "query": {
                    "type": "string",
                    "description": "Plain text search query"
                },
                "paperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter to specific papers (up to ~100)"
                },
                "authors": {
                    "type": "array",
                    "items": {"type": "string"},
                    "description": "Filter by author names (fuzzy match, max 10)"
                },
                "fieldsOfStudy": {
                    "type": "array",
                    "items": {"type": "string"}
                },
                "yearStart": {
                    "type": "integer"
                },
                "yearEnd": {
                    "type": "integer"
                },
                "minCitations": {
                    "type": "integer"
                },
                "venue": {
                    "type": "string"
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
            "required": ["query"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: SnippetSearchInput = serde_json::from_value(input)?;

        // Build filter parameters
        let mut filters: Vec<(String, String)> = Vec::new();

        if let Some(ref paper_ids) = params.paper_ids {
            filters.push(("paperIds".to_string(), paper_ids.join(",")));
        }

        if let Some(ref authors) = params.authors {
            filters.push(("authors".to_string(), authors.join(",")));
        }

        if let Some(ref fields) = params.fields_of_study {
            filters.push(("fieldsOfStudy".to_string(), fields.join(",")));
        }

        if let Some(min_year) = params.year_start {
            if let Some(max_year) = params.year_end {
                filters.push(("year".to_string(), format!("{}-{}", min_year, max_year)));
            } else {
                filters.push(("year".to_string(), format!("{}-", min_year)));
            }
        } else if let Some(max_year) = params.year_end {
            filters.push(("year".to_string(), format!("-{}", max_year)));
        }

        if let Some(min_cites) = params.min_citations {
            filters.push(("minCitationCount".to_string(), min_cites.to_string()));
        }

        if let Some(ref venue) = params.venue {
            filters.push(("venue".to_string(), venue.clone()));
        }

        let result = ctx
            .client
            .search_snippets(&params.query, params.limit, &filters)
            .await
            .map_err(ToolError::from)?;

        match params.response_format {
            ResponseFormat::Markdown => {
                let mut output = format!(
                    "# Snippet Search Results\n\n\
                     **Query:** \"{}\"\n\
                     **Snippets found:** {}\n\n---\n\n",
                    params.query,
                    result.data.len()
                );

                for (i, snippet) in result.data.iter().enumerate() {
                    let title = snippet.paper.as_ref()
                        .and_then(|p| p.title.as_deref())
                        .unwrap_or("Unknown Title");
                    output.push_str(&format!("### {}. {}\n", i + 1, title));

                    if let Some(ref paper) = snippet.paper {
                        if let Some(year) = paper.year {
                            output.push_str(&format!("**Year:** {}\n", year));
                        }
                        if !paper.authors.is_empty() {
                            output.push_str(&format!("**Authors:** {}\n", paper.authors.join(", ")));
                        }
                    }
                    output.push('\n');

                    if let Some(ref snip) = snippet.snippet {
                        if let Some(ref kind) = snip.snippet_kind {
                            output.push_str(&format!("**Source:** {}", kind));
                            if let Some(ref section) = snip.section {
                                output.push_str(&format!(" ({})", section));
                            }
                            output.push('\n');
                        }

                        if let Some(ref text) = snip.text {
                            output.push_str(&format!("> {}\n", text));
                        }
                    }

                    output.push_str("\n---\n\n");
                }

                if result.data.is_empty() {
                    output.push_str("*No snippets found matching the query.*");
                }

                Ok(output)
            }
            ResponseFormat::Json => {
                let snippets: Vec<_> = result
                    .data
                    .iter()
                    .map(|s| {
                        json!({
                            "paper": s.paper.as_ref().map(|p| json!({
                                "paperId": p.paper_id,
                                "title": p.title,
                                "year": p.year,
                                "authors": p.authors
                            })),
                            "score": s.score,
                            "snippet": s.snippet.as_ref().map(|snip| json!({
                                "text": snip.text,
                                "kind": snip.snippet_kind,
                                "section": snip.section
                            }))
                        })
                    })
                    .collect();

                Ok(serde_json::to_string_pretty(&json!({
                    "query": params.query,
                    "total": result.data.len(),
                    "snippets": snippets
                }))?)
            }
        }
    }
}
