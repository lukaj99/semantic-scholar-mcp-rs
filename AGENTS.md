# AI Agent Context: semantic-scholar-mcp-rs

> MCP server for Semantic Scholar API - Rust implementation

## Quick Reference

| Command | Description |
|---------|-------------|
| `cargo build --release` | Build optimized binary (~6.6MB) |
| `cargo test` | Run 39 unit/doc tests |
| `cargo test --features integration` | Run live API tests |
| `./target/release/semantic-scholar-mcp --help` | CLI help |

## Project Overview

Rust rewrite of semantic-scholar-mcp Python server. Provides 29 MCP tools for academic paper discovery, citation analysis, systematic reviews, and bibliometrics.

**Stack:** Tokio, Reqwest, Serde, MCP (JSON-RPC over stdio/HTTP)

## Directory Structure

```
semantic-scholar-mcp-rs/
├── src/
│   ├── main.rs              # CLI entry point (clap)
│   ├── lib.rs               # Library exports
│   ├── config.rs            # Config + field constants
│   ├── error.rs             # thiserror types
│   ├── client/mod.rs        # SemanticScholarClient (HTTP + cache)
│   ├── models/              # Paper, Author, inputs (serde)
│   ├── tools/               # 29 MCP tool implementations
│   ├── formatters/          # Markdown + JSON output
│   └── server/
│       ├── mod.rs           # McpServer entry point
│       ├── stdio.rs         # stdio transport (Claude Desktop)
│       ├── transport.rs     # HTTP/SSE transport (never-failing)
│       └── session.rs       # Session manager with ring buffer
└── tests/
    ├── model_tests.rs       # Unit tests for models
    └── integration_tests.rs # Live API tests (--features integration)
```

## 29 MCP Tools

| Category | Tools |
|----------|-------|
| Discovery (5) | `exhaustive_search`, `recommendations`, `citation_snowball`, `bulk_boolean_search`, `snippet_search` |
| Enrichment (7) | `batch_metadata`, `author_search`, `author_papers`, `paper_autocomplete`, `paper_title_match`, `paper_authors`, `author_batch` |
| Export (1) | `reference_export` (RIS/BibTeX/CSV/EndNote) |
| Systematic (3) | `prisma_search`, `screening_export`, `prisma_flow_diagram` |
| Semantic (2) | `semantic_search`, `literature_review_pipeline` |
| Network (1) | `author_network` |
| Trends (2) | `research_trends`, `venue_analytics` |
| Bibliometrics (6) | `field_weighted_impact`, `highly_cited_papers`, `citation_half_life`, `cocitation_analysis`, `bibliographic_coupling`, `hot_papers` |
| Advanced (2) | `pearl_growing`, `orcid_author_lookup` |

## Critical Rules

```
NEVER: Remove retry/rate-limiting middleware from client
NEVER: Skip cache in production - API has strict rate limits
ALWAYS: Use fields::DEFAULT or fields::EXTENDED for API calls
ALWAYS: Handle Option<T> fields - API responses vary
```

## Key Patterns

### Tool Implementation

```rust
#[async_trait]
impl McpTool for MyTool {
    fn name(&self) -> &'static str { "my_tool" }
    fn description(&self) -> &'static str { "..." }
    fn input_schema(&self) -> serde_json::Value { json!({...}) }
    async fn execute(&self, ctx: &ToolContext, input: Value) -> ToolResult<String> {
        let params: MyInput = serde_json::from_value(input)?;
        // Use ctx.client for API calls
        Ok(formatters::format_markdown(...))
    }
}
```

### Input Models

Input structs use `#[serde(rename_all = "camelCase")]` for MCP protocol compatibility:
- `yearStart` not `year_start` in JSON
- `seedPaperIds` not `seed_paper_ids`

## Never-Failing HTTP Transport

The HTTP transport implements robust "mailbox" pattern for reliable connections:

### Session Management
- **SessionManager:** In-memory session state with ring buffer (100 events)
- **Last-Event-ID:** SSE reconnection recovery - client resumes from where it left off
- **Broadcast Channels:** Live event delivery to multiple subscribers
- **Background Cleanup:** Stale sessions (>1 hour) automatically removed

### How Reconnection Works
1. Client connects to `/sse` or `/mcp` (GET)
2. On disconnect, client reconnects with `Last-Event-ID` header
3. Server replays all missed events from ring buffer
4. Client continues receiving live events seamlessly

### Endpoints
| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/ready` | GET | Readiness with session count |
| `/mcp` | POST | JSON-RPC messages (Streamable HTTP) |
| `/mcp` | GET | SSE stream (server-to-client) |
| `/sse` | GET | Legacy SSE (sends endpoint event) |
| `/message` | POST | Legacy message endpoint |
| `/sessions` | GET | Active session count |

## API Client

- **Base URL:** `https://api.semanticscholar.org/graph/v1`
- **Rate Limit:** 5 req/s (without API key)
- **Cache TTL:** 5 minutes (moka)
- **Retry:** Exponential backoff (reqwest-retry)

## Running

```bash
# stdio mode (for Claude Desktop)
./target/release/semantic-scholar-mcp

# HTTP mode (port 8000)
./target/release/semantic-scholar-mcp --transport http --port 8000

# HTTP mode with custom base URL (for SSE endpoint announcements)
./target/release/semantic-scholar-mcp --transport http --port 8000 --base-url https://scholar.jovanovic.org.uk

# With API key (higher rate limits)
SEMANTIC_SCHOLAR_API_KEY=xxx ./target/release/semantic-scholar-mcp
```

## Claude Desktop Config

```json
{
  "mcpServers": {
    "semantic-scholar": {
      "command": "/path/to/semantic-scholar-mcp"
    }
  }
}
```

## Testing

```bash
# Unit tests (fast, no API)
cargo test

# Live API tests (slow, rate-limited)
cargo test --features integration -- --test-threads=1

# Specific test
cargo test test_paper_deserialize
```

## Data

> Router: See `src/config.rs` for field constants (DEFAULT, EXTENDED, FULL)
> Router: See `tests/fixtures/` for sample API responses

## Resolved Issues

- Input models require camelCase JSON (MCP protocol)
- API returns null for invalid paper IDs in batch requests
- Year field may vary for papers with multiple versions
