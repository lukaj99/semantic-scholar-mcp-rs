//! Semantic Scholar MCP Server - Entry Point
//!
//! Provides both stdio (for Claude Desktop) and HTTP transports.

use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use semantic_scholar_mcp::{config::Config, server::McpServer, SemanticScholarClient};

#[derive(Parser, Debug)]
#[command(name = "semantic-scholar-mcp")]
#[command(about = "MCP server for Semantic Scholar API")]
#[command(version)]
struct Cli {
    /// Semantic Scholar API key (optional, enables higher rate limits)
    #[arg(long, env = "SEMANTIC_SCHOLAR_API_KEY")]
    api_key: Option<String>,

    /// Transport mode: stdio or http
    #[arg(long, default_value = "stdio")]
    transport: Transport,

    /// HTTP server port (only used with --transport http)
    #[arg(long, default_value = "8000", env = "PORT")]
    port: u16,

    /// Base URL for SSE endpoint announcements (e.g., https://scholar.jovanovic.org.uk)
    #[arg(long, env = "BASE_URL")]
    base_url: Option<String>,

    /// Log level (trace, debug, info, warn, error)
    #[arg(long, default_value = "info", env = "RUST_LOG")]
    log_level: String,

    /// Output logs as JSON
    #[arg(long)]
    json_logs: bool,
}

#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
enum Transport {
    /// Standard input/output (for Claude Desktop)
    #[default]
    Stdio,
    /// HTTP with Server-Sent Events
    Http,
}

fn init_tracing(log_level: &str, json: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(log_level));

    let subscriber = tracing_subscriber::registry().with(filter);

    if json {
        subscriber
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        subscriber
            .with(tracing_subscriber::fmt::layer().compact())
            .init();
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    init_tracing(&cli.log_level, cli.json_logs);

    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        transport = ?cli.transport,
        "Starting Semantic Scholar MCP server"
    );

    let config = Config::new(cli.api_key);
    let client = SemanticScholarClient::new(config)?;
    let server = McpServer::new(client);

    match cli.transport {
        Transport::Stdio => {
            tracing::info!("Running in stdio mode");
            server.run_stdio().await?;
        }
        Transport::Http => {
            tracing::info!(port = cli.port, base_url = ?cli.base_url, "Running in HTTP mode");
            server.run_http(cli.port, cli.base_url).await?;
        }
    }

    Ok(())
}
