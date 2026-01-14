//! Enumeration types for API and tool parameters.

use serde::{Deserialize, Serialize};

/// Output format for tool responses.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ResponseFormat {
    /// Human-readable Markdown format.
    #[default]
    Markdown,
    /// Machine-readable JSON format.
    Json,
}

impl ResponseFormat {
    /// Check if this is markdown format.
    #[must_use]
    pub const fn is_markdown(self) -> bool {
        matches!(self, Self::Markdown)
    }

    /// Check if this is JSON format.
    #[must_use]
    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

/// Direction for citation traversal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchDirection {
    /// Papers that cite this paper (forward citations).
    Citations,
    /// Papers that this paper cites (references).
    References,
    /// Both citations and references.
    #[default]
    Both,
}

/// Export format for reference managers.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportFormat {
    /// Research Information Systems format.
    #[default]
    Ris,
    /// BibTeX format.
    Bibtex,
    /// Comma-separated values.
    Csv,
    /// EndNote format.
    Endnote,
}

impl ExportFormat {
    /// Get the file extension for this format.
    #[must_use]
    pub const fn extension(self) -> &'static str {
        match self {
            Self::Ris => "ris",
            Self::Bibtex => "bib",
            Self::Csv => "csv",
            Self::Endnote => "enw",
        }
    }

    /// Get the MIME type for this format.
    #[must_use]
    pub const fn mime_type(self) -> &'static str {
        match self {
            Self::Ris => "application/x-research-info-systems",
            Self::Bibtex => "application/x-bibtex",
            Self::Csv => "text/csv",
            Self::Endnote => "application/x-endnote-refer",
        }
    }
}

/// Time granularity for trend analysis.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TrendGranularity {
    /// Aggregate by year.
    #[default]
    Year,
    /// Aggregate by quarter.
    Quarter,
}

/// Strategy for pearl growing search expansion.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PearlGrowingStrategy {
    /// Expand using extracted keywords.
    Keywords,
    /// Expand using frequent authors.
    Authors,
    /// Expand using citation network.
    Citations,
    /// Use all strategies.
    #[default]
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_response_format_default() {
        assert_eq!(ResponseFormat::default(), ResponseFormat::Markdown);
        assert!(ResponseFormat::Markdown.is_markdown());
        assert!(!ResponseFormat::Markdown.is_json());
    }

    #[test]
    fn test_export_format_extensions() {
        assert_eq!(ExportFormat::Ris.extension(), "ris");
        assert_eq!(ExportFormat::Bibtex.extension(), "bib");
        assert_eq!(ExportFormat::Csv.extension(), "csv");
        assert_eq!(ExportFormat::Endnote.extension(), "enw");
    }

    #[test]
    fn test_serde_roundtrip() {
        let format = ResponseFormat::Json;
        let json = serde_json::to_string(&format).unwrap();
        assert_eq!(json, r#""json""#);

        let parsed: ResponseFormat = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, format);
    }
}
