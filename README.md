# Semantic Scholar MCP Server (Rust)

> **The Gold Standard for Academic AI Integration**

The `semantic-scholar-mcp-rs` project is a high-performance, enterprise-grade Model Context Protocol (MCP) server that bridges Large Language Models (LLMs) with the authoritative academic data of Semantic Scholar.

Built in Rust, it serves as a robust replacement for previous Python implementations, offering superior speed (~20ms startup), reliability (mailbox architecture), and a comprehensive suite of 29 research tools.

## Key Features

*   **29 Specialized Tools:** Covers the full research lifecycle: Discovery, Enrichment, Bibliometrics, and Network Analysis.
*   **Enterprise Reliability:** Implements a "mailbox" pattern with SSE `Last-Event-ID` tracking for zero-loss connection recovery.
*   **High Performance:**
    *   Strict 5-minute TTL caching via `moka`.
    *   Smart rate-limiting (5 req/s public, 100 req/s authenticated).
    *   ~5MB binary size.
*   **OAuth 2.0 Auto-Approve:** Built-in OAuth server (RFC 6749/7591/7636/8414/9728) with PKCE S256 and auto-approval — no interactive login required.
*   **Dual Transport:** Stdio for Claude Desktop local, Streamable HTTP for remote Claude Connector.

## Installation

### Option 1: Local stdio (Claude Desktop / Claude Code)

Ideal for individual researchers.

1.  **Build:**
    ```bash
    cargo build --release
    ```

2.  **Configure Claude Desktop:** Add to `claude_desktop_config.json`:
    ```json
    {
      "mcpServers": {
        "semantic-scholar": {
          "command": "/absolute/path/to/target/release/semantic-scholar-mcp",
          "env": {
            "SEMANTIC_SCHOLAR_API_KEY": "your-api-key"
          }
        }
      }
    }
    ```

3.  **Configure Claude Code:**
    ```bash
    claude mcp add semantic-scholar /path/to/semantic-scholar-mcp
    ```

### Option 2: Remote HTTPS (Claude.ai Connector)

Ideal for shared deployment. Supports both OAuth 2.0 (Claude.ai) and Bearer token (Claude Code) authentication.

1.  **Deploy with Docker Compose:**
    ```bash
    # Create .env with your secrets
    echo "SEMANTIC_SCHOLAR_API_KEY=your_scholar_key" >> .env
    echo "MCP_SERVER_AUTH_TOKEN=$(openssl rand -hex 32)" >> .env

    docker compose up -d
    ```

2.  **Connect via Claude.ai (OAuth 2.0):**
    1. Navigate to Settings > Integrations > Add Custom Connector
    2. Enter your server URL: `https://your-domain.com`
    3. Claude.ai auto-discovers OAuth endpoints and completes authentication automatically
    4. No password or manual steps required — the server auto-approves

3.  **Connect via Claude Code (Bearer token):**
    ```bash
    claude mcp add --transport http semantic-scholar "https://your-domain.com/mcp?token=YOUR_TOKEN"
    ```

### Docker Compose Reference

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
      - BASE_URL=https://your-domain.com
    command: ["--transport", "http", "--port", "8000", "--base-url", "https://your-domain.com"]
    healthcheck:
      test: ["CMD", "curl", "-f", "http://localhost:8000/health"]
      interval: 30s
      timeout: 10s
      retries: 3
```

A reverse proxy (Caddy, nginx, Traefik) with a valid TLS certificate is required for Claude Connector HTTPS.

## Tool Capabilities

| Category | Key Tools | Description |
| :--- | :--- | :--- |
| **Discovery** | `exhaustive_search`, `bulk_boolean_search`, `snippet_search` | Deep search with pagination, boolean queries, text snippet matching |
| | `citation_snowball`, `recommendations` | Traverse citation graphs, SPECTER2 embedding similarity |
| **Enrichment** | `batch_metadata`, `author_search`, `author_papers` | Up to 500 papers per call, author profiles with h-index |
| | `paper_title_match`, `paper_autocomplete`, `author_batch` | Fuzzy matching, autocomplete, bulk author lookup |
| **Systematic** | `prisma_search`, `screening_export`, `prisma_flow_diagram` | PRISMA-guided reviews with dedup and flow diagrams |
| **Analysis** | `field_weighted_impact`, `highly_cited_papers`, `hot_papers` | FWCI normalization, percentile ranking, trend detection |
| | `research_trends`, `venue_analytics` | Publication trends over time, venue statistics |
| **Network** | `author_network`, `cocitation_analysis`, `bibliographic_coupling` | Collaboration graphs, co-citation, shared references |
| **Advanced** | `pearl_growing`, `orcid_author_lookup` | Iterative literature expansion, ORCID resolution |
| **Export** | `reference_export` | RIS, BibTeX, CSV, EndNote formats |

*All tools handle messy academic data, managing `null` values and optional fields to prevent agent crashes.*

## HTTP Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/health` | GET | Health check |
| `/ready` | GET | Readiness with session count |
| `/.well-known/mcp.json` | GET | MCP discovery (Claude Connector) |
| `/mcp` | POST | JSON-RPC messages (Streamable HTTP) |
| `/mcp` | GET | SSE stream (server-to-client) |
| `/sse` | GET | Legacy SSE transport |
| `/.well-known/oauth-protected-resource` | GET | RFC 9728 resource metadata |
| `/.well-known/oauth-authorization-server` | GET | RFC 8414 AS metadata |
| `/register` | POST | Dynamic client registration (RFC 7591) |
| `/authorize` | GET | Authorization (auto-approves, returns code) |
| `/token` | POST | Token exchange and refresh |

## Security

*   **OAuth 2.0:** Auto-approve flow with PKCE S256 — no passwords exposed. Clients authenticate via dynamic registration and code exchange.
*   **Dual-token model:** Semantic Scholar API key stays server-side. Clients authenticate via OAuth tokens or static Bearer token.
*   **Retry middleware:** Always enabled to handle transient API failures.
*   **Caching:** 5-minute TTL to respect API quotas.

## Testing

```bash
# Unit + integration tests (602 tests, no API calls)
cargo test

# Live API tests (requires network, rate-limited)
cargo test --features integration -- --test-threads=1
```

## License

MIT
