//! JSON output formatting with token efficiency.

use serde_json::{Value, json};

use crate::models::{Author, Paper};

/// Create a compact paper representation for JSON output.
///
/// Reduces token usage by ~90% compared to full API response.
#[must_use]
pub fn compact_paper(paper: &Paper) -> Value {
    let mut obj = json!({
        "id": paper.paper_id,
        "title": paper.title_or_default(),
        "year": paper.year,
        "citations": paper.citations(),
    });

    // Add authors as names only
    if !paper.authors.is_empty() {
        obj["authors"] =
            json!(paper.authors.iter().filter_map(|a| a.name.as_ref()).collect::<Vec<_>>());
    }

    // Add optional fields only if present
    if let Some(venue) = &paper.venue {
        obj["venue"] = json!(venue);
    }

    if let Some(doi) = paper.doi() {
        obj["doi"] = json!(doi);
    }

    if let Some(arxiv) = paper.arxiv_id() {
        obj["arxiv"] = json!(arxiv);
    }

    if let Some(pdf) = paper.pdf_url() {
        obj["pdf"] = json!(pdf);
    }

    if let Some(tldr) = paper.tldr_text() {
        obj["tldr"] = json!(tldr);
    }

    if let Some(fields) = &paper.fields_of_study {
        if !fields.is_empty() {
            obj["fields"] = json!(fields);
        }
    }

    obj
}

/// Create a compact author representation for JSON output.
#[must_use]
pub fn compact_author(author: &Author) -> Value {
    let mut obj = json!({
        "id": author.author_id,
        "name": author.name_or_default(),
        "hIndex": author.h_index_value(),
        "citations": author.citations(),
        "papers": author.papers(),
    });

    if !author.affiliations.is_empty() {
        obj["affiliations"] = json!(author.affiliations);
    }

    if let Some(orcid) = author.orcid() {
        obj["orcid"] = json!(orcid);
    }

    if let Some(homepage) = &author.homepage {
        obj["homepage"] = json!(homepage);
    }

    obj
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::AuthorRef;

    #[test]
    fn test_compact_paper() {
        let paper = Paper {
            paper_id: "abc123".to_string(),
            title: Some("Test Paper".to_string()),
            year: Some(2024),
            citation_count: Some(42),
            authors: vec![AuthorRef {
                author_id: Some("auth1".to_string()),
                name: Some("John Doe".to_string()),
            }],
            ..Default::default()
        };

        let compact = compact_paper(&paper);

        assert_eq!(compact["id"], "abc123");
        assert_eq!(compact["title"], "Test Paper");
        assert_eq!(compact["year"], 2024);
        assert_eq!(compact["citations"], 42);
        assert_eq!(compact["authors"], json!(["John Doe"]));
    }
}
