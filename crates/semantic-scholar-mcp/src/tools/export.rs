//! Export tools: reference_export.

use serde_json::json;

use super::{McpTool, ToolContext};
use crate::config::fields;
use crate::error::{ToolError, ToolResult};
use crate::models::{ExportFormat, Paper, ReferenceExportInput};

/// Reference export tool.
pub struct ReferenceExportTool;

#[async_trait::async_trait]
impl McpTool for ReferenceExportTool {
    fn name(&self) -> &'static str {
        "reference_export"
    }

    fn description(&self) -> &'static str {
        "Export papers in reference manager formats (RIS, BibTeX, CSV, EndNote)."
    }

    fn input_schema(&self) -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "paperIds": {
                    "type": "array",
                    "items": {"type": "string"},
                    "maxItems": 500,
                    "description": "Paper IDs to export"
                },
                "format": {
                    "type": "string",
                    "enum": ["ris", "bibtex", "csv", "endnote"],
                    "default": "ris"
                },
                "includeAbstract": {
                    "type": "boolean",
                    "default": true
                }
            },
            "required": ["paperIds"]
        })
    }

    async fn execute(&self, ctx: &ToolContext, input: serde_json::Value) -> ToolResult<String> {
        let params: ReferenceExportInput = serde_json::from_value(input)?;

        let papers = ctx
            .client
            .get_papers_batch(&params.paper_ids, fields::DEFAULT)
            .await
            .map_err(ToolError::from)?;

        let output = match params.format {
            ExportFormat::Ris => format_ris(&papers, params.include_abstract),
            ExportFormat::Bibtex => format_bibtex(&papers, params.include_abstract),
            ExportFormat::Csv => format_csv(&papers, params.include_abstract),
            ExportFormat::Endnote => format_endnote(&papers, params.include_abstract),
        };

        Ok(output)
    }
}

/// Format papers as RIS.
fn format_ris(papers: &[Paper], include_abstract: bool) -> String {
    let mut output = String::new();

    for paper in papers {
        output.push_str("TY  - JOUR\n");
        output.push_str(&format!("TI  - {}\n", paper.title_or_default()));

        for author in &paper.authors {
            if let Some(name) = &author.name {
                output.push_str(&format!("AU  - {name}\n"));
            }
        }

        if let Some(year) = paper.year {
            output.push_str(&format!("PY  - {year}\n"));
        }

        if let Some(venue) = &paper.venue {
            output.push_str(&format!("JO  - {venue}\n"));
        }

        if include_abstract {
            if let Some(abs) = &paper.r#abstract {
                // RIS format requires continuation lines for multi-line abstracts
                let abs_clean = abs.replace('\r', "").replace('\n', " ");
                output.push_str(&format!("AB  - {abs_clean}\n"));
            }
        }

        if let Some(doi) = paper.doi() {
            output.push_str(&format!("DO  - {doi}\n"));
        }

        output.push_str(&format!("ID  - {}\n", paper.paper_id));
        output.push_str("ER  - \n\n");
    }

    output
}

/// Format papers as BibTeX.
fn format_bibtex(papers: &[Paper], include_abstract: bool) -> String {
    let mut output = String::new();

    for paper in papers {
        let first_author = paper.first_author().unwrap_or("Unknown");
        let year = paper.year.unwrap_or(0);
        let key = format!(
            "{}{}",
            first_author.split_whitespace().last().unwrap_or("Unknown"),
            year
        );

        output.push_str(&format!("@article{{{key},\n"));
        output.push_str(&format!("  title = {{{}}},\n", escape_bibtex(paper.title_or_default())));
        output.push_str(&format!("  author = {{{}}},\n", escape_bibtex(&paper.author_names())));

        if year > 0 {
            output.push_str(&format!("  year = {{{year}}},\n"));
        }

        if let Some(venue) = &paper.venue {
            output.push_str(&format!("  journal = {{{}}},\n", escape_bibtex(venue)));
        }

        if include_abstract {
            if let Some(abs) = &paper.r#abstract {
                let abs_escaped = escape_bibtex(abs);
                output.push_str(&format!("  abstract = {{{abs_escaped}}},\n"));
            }
        }

        if let Some(doi) = paper.doi() {
            output.push_str(&format!("  doi = {{{doi}}},\n"));
        }

        output.push_str("}\n\n");
    }

    output
}

/// Format papers as CSV.
fn format_csv(papers: &[Paper], include_abstract: bool) -> String {
    let mut output = String::new();

    // Header
    if include_abstract {
        output.push_str("paper_id,title,authors,year,venue,citations,doi,abstract\n");
    } else {
        output.push_str("paper_id,title,authors,year,venue,citations,doi\n");
    }

    for paper in papers {
        let title = csv_escape(paper.title_or_default());
        let authors = csv_escape(&paper.author_names());
        let year = paper.year.map_or(String::new(), |y| y.to_string());
        let venue = csv_escape(paper.venue.as_deref().unwrap_or(""));
        let citations = paper.citations().to_string();
        let doi = paper.doi().unwrap_or("");

        if include_abstract {
            let abs = csv_escape(paper.r#abstract.as_deref().unwrap_or(""));
            output.push_str(&format!(
                "{},{title},{authors},{year},{venue},{citations},{doi},{abs}\n",
                paper.paper_id
            ));
        } else {
            output.push_str(&format!(
                "{},{title},{authors},{year},{venue},{citations},{doi}\n",
                paper.paper_id
            ));
        }
    }

    output
}

/// Format papers as EndNote.
fn format_endnote(papers: &[Paper], include_abstract: bool) -> String {
    let mut output = String::new();

    for paper in papers {
        output.push_str("%0 Journal Article\n");
        output.push_str(&format!("%T {}\n", paper.title_or_default()));

        for author in &paper.authors {
            if let Some(name) = &author.name {
                output.push_str(&format!("%A {name}\n"));
            }
        }

        if let Some(year) = paper.year {
            output.push_str(&format!("%D {year}\n"));
        }

        if let Some(venue) = &paper.venue {
            output.push_str(&format!("%J {venue}\n"));
        }

        if include_abstract {
            if let Some(abs) = &paper.r#abstract {
                // EndNote format: replace newlines with spaces
                let abs_clean = abs.replace('\r', "").replace('\n', " ");
                output.push_str(&format!("%X {abs_clean}\n"));
            }
        }

        if let Some(doi) = paper.doi() {
            output.push_str(&format!("%R {doi}\n"));
        }

        output.push('\n');
    }

    output
}

/// Escape a string for BibTeX output.
fn escape_bibtex(s: &str) -> String {
    s.replace('\\', "\\textbackslash{}")
        .replace('{', "\\{")
        .replace('}', "\\}")
        .replace('&', "\\&")
        .replace('%', "\\%")
        .replace('$', "\\$")
        .replace('#', "\\#")
        .replace('_', "\\_")
        .replace('^', "\\textasciicircum{}")
        .replace('~', "\\textasciitilde{}")
}

/// Escape a string for CSV output.
fn csv_escape(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        // Prefix with single quote to prevent formula injection in spreadsheets
        let escaped = s.replace('"', "\"\"");
        if escaped.starts_with('=') || escaped.starts_with('+') || escaped.starts_with('-') || escaped.starts_with('@') {
            format!("\"'{}\"", escaped)
        } else {
            format!("\"{}\"", escaped)
        }
    } else if s.starts_with('=') || s.starts_with('+') || s.starts_with('-') || s.starts_with('@') {
        // Prevent CSV injection
        format!("'{}", s)
    } else {
        s.to_string()
    }
}
