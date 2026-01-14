//! MCP server implementation.
//!
//! Provides both stdio (for Claude Desktop) and HTTP transports.
//!
//! ## Never-Failing Architecture
//!
//! The HTTP transport implements a robust "mailbox" pattern:
//! - Session-based message buffering with ring buffer
//! - Last-Event-ID support for reconnection recovery
//! - Broadcast channels for live event delivery
//! - Background cleanup of stale sessions

pub mod session;
pub mod stdio;
pub mod transport;

use std::net::SocketAddr;
use std::sync::Arc;

use crate::client::SemanticScholarClient;
use crate::tools::{self, McpTool, ToolContext};

/// MCP server for Semantic Scholar.
pub struct McpServer {
    /// Tool execution context.
    ctx: ToolContext,

    /// Registered tools.
    tools: Vec<Box<dyn McpTool>>,
}

impl McpServer {
    /// Create a new MCP server.
    #[must_use]
    pub fn new(client: SemanticScholarClient) -> Self {
        let ctx = ToolContext::new(Arc::new(client));
        let tools = tools::register_all_tools();

        Self { ctx, tools }
    }

    /// Run the server in stdio mode (for Claude Desktop).
    ///
    /// # Errors
    ///
    /// Returns error on I/O failure.
    pub async fn run_stdio(self) -> anyhow::Result<()> {
        tracing::info!("Starting MCP server in stdio mode");
        tracing::info!("Registered {} tools", self.tools.len());

        stdio::run_stdio(self.tools, self.ctx).await
    }

    /// Run the server in HTTP mode.
    ///
    /// # Errors
    ///
    /// Returns error on server failure.
    pub async fn run_http(self, port: u16, base_url: Option<String>) -> anyhow::Result<()> {
        tracing::info!("Starting MCP server in HTTP mode on port {}", port);
        tracing::info!("Registered {} tools", self.tools.len());

        let router = transport::create_router(self.tools, self.ctx, base_url);
        let addr = SocketAddr::from(([0, 0, 0, 0], port));

        tracing::info!("HTTP server listening on http://{}", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, router).with_graceful_shutdown(shutdown_signal()).await?;

        tracing::info!("HTTP server shut down");
        Ok(())
    }

    /// Get tool by name.
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<&dyn McpTool> {
        self.tools.iter().find(|t| t.name() == name).map(|t| t.as_ref())
    }

    /// List all available tools.
    #[must_use]
    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        self.tools.iter().map(|t| (t.name(), t.description())).collect()
    }

    /// Get tool context for execution.
    #[must_use]
    pub const fn context(&self) -> &ToolContext {
        &self.ctx
    }
}

impl std::fmt::Debug for McpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServer").field("tools", &self.tools.len()).finish()
    }
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c().await.expect("Failed to install CTRL+C handler");
    tracing::info!("Received shutdown signal");
}
