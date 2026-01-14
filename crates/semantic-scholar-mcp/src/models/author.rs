//! Author data model matching Semantic Scholar API schema.

use serde::{Deserialize, Serialize};

/// Author search result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorSearchResult {
    /// Total matching authors.
    pub total: i64,

    /// Offset for pagination.
    #[serde(default)]
    pub offset: i32,

    /// Next offset if more results.
    #[serde(default)]
    pub next: Option<i32>,

    /// List of authors.
    pub data: Vec<Author>,
}

impl AuthorSearchResult {
    /// Check if there are more results.
    #[must_use]
    pub const fn has_more(&self) -> bool {
        self.next.is_some()
    }
}

/// A research author from Semantic Scholar.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Author {
    /// Unique Semantic Scholar author ID.
    pub author_id: String,

    /// Author name.
    #[serde(default)]
    pub name: Option<String>,

    /// Author's institutional affiliations.
    #[serde(default)]
    pub affiliations: Vec<String>,

    /// Author's homepage URL.
    #[serde(default)]
    pub homepage: Option<String>,

    /// Total number of papers by this author.
    #[serde(default)]
    pub paper_count: Option<i32>,

    /// Total citation count across all papers.
    #[serde(default)]
    pub citation_count: Option<i32>,

    /// h-index metric.
    #[serde(default)]
    pub h_index: Option<i32>,

    /// External IDs (ORCID, DBLP, etc.).
    #[serde(default)]
    pub external_ids: Option<AuthorExternalIds>,
}

impl Author {
    /// Get the author name, falling back to "Unknown" if not available.
    #[must_use]
    pub fn name_or_default(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown")
    }

    /// Get the primary affiliation if available.
    #[must_use]
    pub fn primary_affiliation(&self) -> Option<&str> {
        self.affiliations.first().map(String::as_str)
    }

    /// Get the ORCID if available.
    #[must_use]
    pub fn orcid(&self) -> Option<&str> {
        self.external_ids.as_ref()?.orcid.as_deref()
    }

    /// Get the h-index or 0 if not available.
    #[must_use]
    pub fn h_index_value(&self) -> i32 {
        self.h_index.unwrap_or(0)
    }

    /// Get citation count or 0 if not available.
    #[must_use]
    pub fn citations(&self) -> i32 {
        self.citation_count.unwrap_or(0)
    }

    /// Get paper count or 0 if not available.
    #[must_use]
    pub fn papers(&self) -> i32 {
        self.paper_count.unwrap_or(0)
    }
}

/// Minimal author reference (used in paper author lists).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorRef {
    /// Author ID.
    #[serde(default)]
    pub author_id: Option<String>,

    /// Author name.
    #[serde(default)]
    pub name: Option<String>,
}

impl AuthorRef {
    /// Get the author ID or empty string.
    #[must_use]
    pub fn id(&self) -> &str {
        self.author_id.as_deref().unwrap_or("")
    }

    /// Get the author name or "Unknown".
    #[must_use]
    pub fn name_or_default(&self) -> &str {
        self.name.as_deref().unwrap_or("Unknown")
    }
}

/// External identifiers for an author.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub struct AuthorExternalIds {
    /// ORCID identifier.
    #[serde(rename = "ORCID")]
    pub orcid: Option<String>,

    /// DBLP key.
    #[serde(rename = "DBLP")]
    pub dblp: Option<Vec<String>>,
}

/// Author papers result.
#[allow(dead_code)] // Reserved for future author papers endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthorPapersResult {
    /// Offset for pagination.
    #[serde(default)]
    pub offset: i32,

    /// Next offset if more results.
    #[serde(default)]
    pub next: Option<i32>,

    /// List of papers.
    pub data: Vec<super::Paper>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_author_deserialize() {
        let json = r#"{
            "authorId": "123",
            "name": "Jane Smith",
            "affiliations": ["MIT", "Stanford"],
            "paperCount": 50,
            "citationCount": 1000,
            "hIndex": 15
        }"#;

        let author: Author = serde_json::from_str(json).unwrap();
        assert_eq!(author.author_id, "123");
        assert_eq!(author.name_or_default(), "Jane Smith");
        assert_eq!(author.primary_affiliation(), Some("MIT"));
        assert_eq!(author.h_index_value(), 15);
        assert_eq!(author.citations(), 1000);
    }

    #[test]
    fn test_author_ref() {
        let json = r#"{"authorId": "123", "name": "John"}"#;
        let author: AuthorRef = serde_json::from_str(json).unwrap();
        assert_eq!(author.id(), "123");
        assert_eq!(author.name_or_default(), "John");
    }

    #[test]
    fn test_author_minimal() {
        let json = r#"{"authorId": "456"}"#;
        let author: Author = serde_json::from_str(json).unwrap();
        assert_eq!(author.name_or_default(), "Unknown");
        assert_eq!(author.h_index_value(), 0);
    }
}
