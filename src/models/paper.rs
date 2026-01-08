//! Paper data model matching Semantic Scholar API schema.

use serde::{Deserialize, Serialize};

use super::AuthorRef;

/// A research paper from Semantic Scholar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Paper {
    /// Unique Semantic Scholar paper ID.
    pub paper_id: String,

    /// Paper title.
    #[serde(default)]
    pub title: Option<String>,

    /// Paper abstract.
    #[serde(default)]
    pub r#abstract: Option<String>,

    /// Publication year.
    #[serde(default)]
    pub year: Option<i32>,

    /// Number of citations this paper has received.
    #[serde(default)]
    pub citation_count: Option<i32>,

    /// Number of references in this paper.
    #[serde(default)]
    pub reference_count: Option<i32>,

    /// Fields of study (e.g., "Computer Science", "Medicine").
    #[serde(default)]
    pub fields_of_study: Option<Vec<String>>,

    /// List of authors.
    #[serde(default)]
    pub authors: Vec<AuthorRef>,

    /// Publication venue (journal or conference).
    #[serde(default)]
    pub venue: Option<String>,

    /// Publication date in ISO format (YYYY-MM-DD).
    #[serde(default)]
    pub publication_date: Option<String>,

    /// Open access PDF information.
    #[serde(default)]
    pub open_access_pdf: Option<OpenAccessPdf>,

    /// External identifiers (DOI, ArXiv, PubMed, etc.).
    #[serde(default)]
    pub external_ids: Option<ExternalIds>,

    /// AI-generated TLDR summary.
    #[serde(default)]
    pub tldr: Option<Tldr>,

    /// SPECTER2 embedding (768-dimensional).
    #[serde(default)]
    pub embedding: Option<Embedding>,

    /// Influential citation count.
    #[serde(default)]
    pub influential_citation_count: Option<i32>,

    /// Whether this paper is open access.
    #[serde(default)]
    pub is_open_access: Option<bool>,

    /// S2 corpus ID.
    #[serde(default)]
    pub corpus_id: Option<i64>,
}

impl Paper {
    /// Get the paper title, falling back to "Untitled" if not available.
    #[must_use]
    pub fn title_or_default(&self) -> &str {
        self.title.as_deref().unwrap_or("Untitled")
    }

    /// Get the DOI if available.
    #[must_use]
    pub fn doi(&self) -> Option<&str> {
        self.external_ids.as_ref()?.doi.as_deref()
    }

    /// Get the ArXiv ID if available.
    #[must_use]
    pub fn arxiv_id(&self) -> Option<&str> {
        self.external_ids.as_ref()?.arxiv.as_deref()
    }

    /// Get the open access PDF URL if available.
    #[must_use]
    pub fn pdf_url(&self) -> Option<&str> {
        self.open_access_pdf.as_ref()?.url.as_deref()
    }

    /// Get the TLDR text if available.
    #[must_use]
    pub fn tldr_text(&self) -> Option<&str> {
        self.tldr.as_ref()?.text.as_deref()
    }

    /// Check if this paper has a citation count.
    #[must_use]
    pub const fn has_citations(&self) -> bool {
        matches!(self.citation_count, Some(c) if c > 0)
    }

    /// Get citation count or 0 if not available.
    #[must_use]
    pub fn citations(&self) -> i32 {
        self.citation_count.unwrap_or(0)
    }

    /// Get the first author's name if available.
    #[must_use]
    pub fn first_author(&self) -> Option<&str> {
        self.authors.first()?.name.as_deref()
    }

    /// Get author names as a comma-separated string.
    #[must_use]
    pub fn author_names(&self) -> String {
        self.authors
            .iter()
            .filter_map(|a| a.name.as_ref())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// Minimal paper reference (used in citation lists).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaperRef {
    /// Paper ID.
    pub paper_id: String,

    /// Paper title.
    #[serde(default)]
    pub title: Option<String>,
}

/// Open access PDF information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAccessPdf {
    /// Direct URL to the PDF.
    pub url: Option<String>,

    /// Status of open access.
    pub status: Option<String>,
}

/// External identifiers for a paper.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct ExternalIds {
    /// Digital Object Identifier.
    #[serde(rename = "DOI")]
    pub doi: Option<String>,

    /// ArXiv preprint ID.
    #[serde(rename = "ArXiv")]
    pub arxiv: Option<String>,

    /// PubMed ID.
    #[serde(rename = "PubMed")]
    pub pubmed: Option<String>,

    /// PubMed Central ID.
    #[serde(rename = "PubMedCentral")]
    pub pmc: Option<String>,

    /// Microsoft Academic Graph ID.
    #[serde(rename = "MAG")]
    pub mag: Option<String>,

    /// Semantic Scholar Corpus ID.
    #[serde(rename = "CorpusId")]
    pub corpus_id: Option<i64>,

    /// DBLP key.
    #[serde(rename = "DBLP")]
    pub dblp: Option<String>,

    /// ACL Anthology ID.
    #[serde(rename = "ACL")]
    pub acl: Option<String>,
}

/// AI-generated TLDR summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tldr {
    /// Summary text.
    pub text: Option<String>,

    /// Model used to generate the summary.
    pub model: Option<String>,
}

/// SPECTER2 embedding vector.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    /// Model identifier.
    pub model: Option<String>,

    /// 768-dimensional embedding vector.
    pub vector: Option<Vec<f32>>,
}

/// Search result wrapper.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SearchResult {
    /// Total number of matching papers.
    #[serde(default)]
    pub total: i64,

    /// Current offset in the result set.
    #[serde(default)]
    pub offset: i32,

    /// Indicates if more results are available.
    #[serde(default)]
    pub next: Option<i32>,

    /// List of papers in this page.
    #[serde(default)]
    pub data: Vec<Paper>,

    /// Error message (if search failed).
    #[serde(default)]
    pub message: Option<String>,
}

impl SearchResult {
    /// Check if there are more results available.
    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.next.is_some()
    }

    /// Get the next offset for pagination.
    #[must_use]
    pub const fn next_offset(&self) -> Option<i32> {
        self.next
    }
}

/// Citation context with citing/cited paper.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CitationContext {
    /// The citing or cited paper.
    #[serde(alias = "citingPaper", alias = "citedPaper")]
    pub paper: Option<Paper>,

    /// Citation contexts/snippets.
    #[serde(default)]
    pub contexts: Vec<String>,

    /// Intent of the citation.
    #[serde(default)]
    pub intents: Vec<String>,

    /// Whether this is an influential citation.
    #[serde(default)]
    pub is_influential: bool,
}

/// Citation list result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CitationResult {
    /// Offset for pagination.
    #[serde(default)]
    pub offset: i32,

    /// Next offset if more results available.
    #[serde(default)]
    pub next: Option<i32>,

    /// Citation data.
    pub data: Vec<CitationContext>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_deserialize_minimal() {
        let json = r#"{"paperId": "abc123"}"#;
        let paper: Paper = serde_json::from_str(json).unwrap();
        assert_eq!(paper.paper_id, "abc123");
        assert!(paper.title.is_none());
        assert!(paper.authors.is_empty());
    }

    #[test]
    fn test_paper_deserialize_full() {
        let json = r#"{
            "paperId": "abc123",
            "title": "Test Paper",
            "abstract": "This is a test.",
            "year": 2024,
            "citationCount": 42,
            "authors": [{"authorId": "auth1", "name": "John Doe"}],
            "externalIds": {"DOI": "10.1234/test"}
        }"#;

        let paper: Paper = serde_json::from_str(json).unwrap();
        assert_eq!(paper.paper_id, "abc123");
        assert_eq!(paper.title_or_default(), "Test Paper");
        assert_eq!(paper.year, Some(2024));
        assert_eq!(paper.citations(), 42);
        assert_eq!(paper.doi(), Some("10.1234/test"));
        assert_eq!(paper.first_author(), Some("John Doe"));
    }

    #[test]
    fn test_search_result() {
        let json = r#"{
            "total": 100,
            "offset": 0,
            "next": 10,
            "data": []
        }"#;

        let result: SearchResult = serde_json::from_str(json).unwrap();
        assert_eq!(result.total, 100);
        assert!(result.has_more());
        assert_eq!(result.next_offset(), Some(10));
    }
}
