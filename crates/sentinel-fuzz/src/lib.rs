//! Fuzzing library for semantic-scholar-mcp.
//!
//! This crate provides fuzzing targets for testing JSON deserialization
//! of the Semantic Scholar API models.
//!
//! # Usage
//!
//! ```bash
//! cd crates/sentinel-fuzz
//! cargo +nightly fuzz run fuzz_paper_parse -- -max_total_time=60
//! ```

pub use semantic_scholar_mcp::models;
