//! OAuth 2.0 authorization server for MCP authentication.
//!
//! Implements a self-contained OAuth server embedded in the binary,
//! supporting the MCP OAuth flow used by Claude.ai Custom Connector.
//!
//! ## Supported Standards
//! - RFC 9728: OAuth Protected Resource Metadata
//! - RFC 8414: OAuth Authorization Server Metadata
//! - RFC 7591: Dynamic Client Registration
//! - RFC 7636: PKCE (S256)
//! - RFC 6749: Authorization Code Grant

pub mod handlers;
pub mod pkce;
pub mod store;
mod types;

pub use store::OAuthStore;
