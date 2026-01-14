//! Semantic Scholar MCP Server
//!
//! A Model Context Protocol (MCP) server for the Semantic Scholar Graph API.
//! Enables LLM agents to search academic papers, analyze citations, run systematic
//! reviews, and compute bibliometrics.
//!
//! # Features
//!
//! - **23 MCP Tools**: Discovery, enrichment, systematic review, export, bibliometrics
//! - **Async-first**: Built on Tokio with streaming pagination
//! - **Rate-limited**: Respects Semantic Scholar API limits
//! - **Cached**: 5-minute TTL cache reduces API calls
//!
//! # Example
//!
//! ```no_run
//! use semantic_scholar_mcp::{client::SemanticScholarClient, config::Config};
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let config = Config::from_env()?;
//!     let client = SemanticScholarClient::new(config)?;
//!
//!     // Use client for API calls
//!     Ok(())
//! }
//! ```

pub mod client;
pub mod config;
pub mod error;
pub mod formatters;
pub mod models;
pub mod server;
pub mod tools;

pub use client::SemanticScholarClient;
pub use config::Config;
pub use error::{ClientError, ToolError};
