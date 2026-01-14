//! Markdown output formatting.

use std::borrow::Cow;

use crate::models::{Author, Paper};

/// Format a list of papers as Markdown.
#[must_use]
pub fn format_papers_markdown(papers: &[Paper]) -> String {
    if papers.is_empty() {
        return "No papers found.".to_string();
    }

    let mut output = format!("# Papers ({} results)\n\n", papers.len());

    for (i, paper) in papers.iter().enumerate() {
        output.push_str(&format_paper_markdown(paper, i + 1));
        output.push_str("\n---\n\n");
    }

    output
}

/// Format a single paper as Markdown.
#[must_use]
pub fn format_paper_markdown(paper: &Paper, index: usize) -> String {
    let mut output = String::new();

    // Title
    output.push_str(&format!("## {}. {}\n\n", index, paper.title_or_default()));

    // Authors
    if !paper.authors.is_empty() {
        output.push_str(&format!("**Authors**: {}\n\n", paper.author_names()));
    }

    // Year and citations
    let mut meta = Vec::new();
    if let Some(year) = paper.year {
        meta.push(format!("**Year**: {year}"));
    }
    meta.push(format!("**Citations**: {}", paper.citations()));

    if let Some(venue) = &paper.venue {
        meta.push(format!("**Venue**: {venue}"));
    }

    output.push_str(&format!("{}\n\n", meta.join(" | ")));

    // Fields of study
    if let Some(fields) = &paper.fields_of_study {
        if !fields.is_empty() {
            output.push_str(&format!("**Fields**: {}\n\n", fields.join(", ")));
        }
    }

    // External IDs
    let mut ids = Vec::new();
    if let Some(doi) = paper.doi() {
        ids.push(format!("[DOI](https://doi.org/{doi})"));
    }
    if let Some(arxiv) = paper.arxiv_id() {
        ids.push(format!("[arXiv](https://arxiv.org/abs/{arxiv})"));
    }
    ids.push(format!(
        "[S2](https://www.semanticscholar.org/paper/{})",
        paper.paper_id
    ));

    if !ids.is_empty() {
        output.push_str(&format!("**Links**: {}\n\n", ids.join(" | ")));
    }

    // PDF
    if let Some(pdf_url) = paper.pdf_url() {
        output.push_str(&format!("**PDF**: [Open Access]({pdf_url})\n\n"));
    }

    // TLDR
    if let Some(tldr) = paper.tldr_text() {
        output.push_str(&format!("> **TLDR**: {tldr}\n\n"));
    }

    // Abstract (truncated)
    if let Some(abs) = &paper.r#abstract {
        let truncated: Cow<'_, str> = if abs.len() > 300 {
            Cow::Owned(format!("{}...", &abs[..300]))
        } else {
            Cow::Borrowed(abs)
        };
        output.push_str(&format!("**Abstract**: {truncated}\n"));
    }

    output
}

/// Format a list of authors as Markdown.
#[must_use]
pub fn format_authors_markdown(authors: &[Author]) -> String {
    if authors.is_empty() {
        return "No authors found.".to_string();
    }

    let mut output = format!("# Authors ({} results)\n\n", authors.len());

    for (i, author) in authors.iter().enumerate() {
        output.push_str(&format_author_markdown_indexed(author, i + 1));
        output.push_str("\n---\n\n");
    }

    output
}

/// Format a single author as Markdown.
#[must_use]
pub fn format_author_markdown(author: &Author) -> String {
    format_author_markdown_indexed(author, 0)
}

fn format_author_markdown_indexed(author: &Author, index: usize) -> String {
    let mut output = String::new();

    // Name
    if index > 0 {
        output.push_str(&format!("## {}. {}\n\n", index, author.name_or_default()));
    } else {
        output.push_str(&format!("## {}\n\n", author.name_or_default()));
    }

    // Affiliations
    if !author.affiliations.is_empty() {
        output.push_str(&format!("**Affiliations**: {}\n\n", author.affiliations.join(", ")));
    }

    // Metrics
    let mut metrics = Vec::new();
    metrics.push(format!("**h-index**: {}", author.h_index_value()));
    metrics.push(format!("**Citations**: {}", author.citations()));
    metrics.push(format!("**Papers**: {}", author.papers()));

    output.push_str(&format!("{}\n\n", metrics.join(" | ")));

    // ORCID
    if let Some(orcid) = author.orcid() {
        output.push_str(&format!(
            "**ORCID**: [{}](https://orcid.org/{})\n\n",
            orcid, orcid
        ));
    }

    // Homepage
    if let Some(homepage) = &author.homepage {
        output.push_str(&format!("**Homepage**: [{homepage}]({homepage})\n\n"));
    }

    // S2 link
    output.push_str(&format!(
        "**S2 Profile**: [View](https://www.semanticscholar.org/author/{})\n",
        author.author_id
    ));

    output
}
