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
| `/.well-known/mcp.json` | GET | MCP discovery (Claude Connector) |
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

## Deployment Configurations

### 1. Local stdio (Claude Desktop / Claude Code)

For local development and Claude Desktop integration:

```json
{
  "mcpServers": {
    "semantic-scholar": {
      "command": "/path/to/semantic-scholar-mcp",
      "env": {
        "SEMANTIC_SCHOLAR_API_KEY": "your-api-key"
      }
    }
  }
}
```

Claude Code:
```bash
claude mcp add semantic-scholar /path/to/semantic-scholar-mcp
```

### 2. Remote HTTPS (Claude Connector)

For Claude.ai and Claude Code remote connections via Claude Connector:

**Docker Deployment:**
```bash
docker compose up -d
```

**Authentication:**
This server uses **Bearer Token Authentication**.
- **Token:** `MCP_SERVER_AUTH_TOKEN` (set in `.env`)
- **Magic Link:** `https://your-domain.com?token=YOUR_TOKEN`

**Add to Claude Code:**
```bash
# Connect using the authenticated endpoint directly (Recommended)
claude mcp add --transport http semantic-scholar "https://scholar.jovanovic.org.uk/mcp?token=YOUR_TOKEN"
```

**Claude.ai Integration:**
1. Navigate to Settings > Integrations
2. Add Custom Connector
3. Enter URL: `https://scholar.jovanovic.org.uk?token=YOUR_TOKEN`
4. The server will auto-discover capabilities via `/.well-known/mcp.json` and inject the token into endpoints.

**Discovery Endpoint:** `GET /.well-known/mcp.json`
```json
{
  "name": "semantic-scholar-mcp",
  "version": "0.1.0",
  "capabilities": { "tools": true, "resources": false, "prompts": false },
  "auth": { "type": "none" },
  "endpoints": {
    "mcp": "https://scholar.jovanovic.org.uk/mcp?token=YOUR_TOKEN",
    "sse": "https://scholar.jovanovic.org.uk/sse?token=YOUR_TOKEN",
    "health": "https://scholar.jovanovic.org.uk/health?token=YOUR_TOKEN"
  }
}
```

### 3. Docker Compose Production

```yaml
services:
  semantic-scholar-mcp:
    build: .
    restart: unless-stopped
    ports:
      - "8000:8000"
    environment:
      - SEMANTIC_SCHOLAR_API_KEY=${SEMANTIC_SCHOLAR_API_KEY}
      - MCP_SERVER_AUTH_TOKEN=${MCP_SERVER_AUTH_TOKEN}
      - RUST_LOG=info
      - BASE_URL=https://scholar.jovanovic.org.uk
    command: ["--transport", "http", "--port", "8000", "--base-url", "https://scholar.jovanovic.org.uk"]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

### HTTPS Requirements for Claude Connector

- **Valid TLS certificate** (Let's Encrypt, Cloudflare, etc.)
- **Public HTTPS URL** (Claude Connector requires HTTPS)
- **Reverse proxy** (nginx, Caddy, Traefik) for TLS termination

Example nginx config:
```nginx
server {
    listen 443 ssl http2;
    server_name scholar.jovanovic.org.uk;

    ssl_certificate /etc/letsencrypt/live/scholar.jovanovic.org.uk/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/scholar.jovanovic.org.uk/privkey.pem;

    location / {
        proxy_pass http://127.0.0.1:8000;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_buffering off;
        proxy_cache off;
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
- Server-side filtering implemented for optimal API usage
- Bearer token authentication with "Magic Link" support for Claude.ai
