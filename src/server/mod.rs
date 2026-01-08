//! MCP server implementation.
//!
//! Provides both stdio (for Claude Desktop) and HTTP transports.

mod transport;

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
    pub async fn run_stdio(&self) -> anyhow::Result<()> {
        tracing::info!("Starting MCP server in stdio mode");
        tracing::info!("Registered {} tools", self.tools.len());

        for tool in &self.tools {
            tracing::debug!("  - {}: {}", tool.name(), tool.description());
        }

        // TODO: Implement actual MCP protocol handling with rmcp
        // For now, just log tool registration and wait
        tracing::info!("MCP server ready. Waiting for connections...");

        // Placeholder: keep the process running
        // In production, this would handle stdin/stdout JSON-RPC
        tokio::signal::ctrl_c().await?;

        tracing::info!("Shutting down MCP server");
        Ok(())
    }

    /// Run the server in HTTP mode.
    ///
    /// # Errors
    ///
    /// Returns error on server failure.
    pub async fn run_http(&self, port: u16) -> anyhow::Result<()> {
        tracing::info!("Starting MCP server in HTTP mode on port {}", port);
        tracing::info!("Registered {} tools", self.tools.len());

        // TODO: Implement HTTP transport with SSE
        // For now, just log and wait
        tracing::info!("HTTP transport not yet implemented");

        tokio::signal::ctrl_c().await?;

        tracing::info!("Shutting down HTTP server");
        Ok(())
    }

    /// Get tool by name.
    #[must_use]
    pub fn get_tool(&self, name: &str) -> Option<&dyn McpTool> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// List all available tools.
    #[must_use]
    pub fn list_tools(&self) -> Vec<(&str, &str)> {
        self.tools
            .iter()
            .map(|t| (t.name(), t.description()))
            .collect()
    }

    /// Get tool context for execution.
    #[must_use]
    pub const fn context(&self) -> &ToolContext {
        &self.ctx
    }
}

impl std::fmt::Debug for McpServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("McpServer")
            .field("tools", &self.tools.len())
            .finish()
    }
}
