//! Data models for Semantic Scholar API entities.
//!
//! All models use `#[serde(default)]` for optional fields and
//! `#[serde(rename = "camelCase")]` to match API naming.

mod author;
mod enums;
mod inputs;
mod paper;

pub use author::{Author, AuthorRef, AuthorSearchResult};
pub use enums::{
    ExportFormat, PearlGrowingStrategy, ResponseFormat, SearchDirection, TrendGranularity,
};
pub use inputs::*;
pub use paper::{
    AutocompleteMatch, AutocompleteResult, BulkSearchResult, CitationResult, ExternalIds,
    OpenAccessPdf, Paper, PaperAuthorsResult, PaperRef, SearchResult, Snippet, SnippetPaper,
    SnippetSearchResult, SnippetText, TitleMatchResult, Tldr,
};
