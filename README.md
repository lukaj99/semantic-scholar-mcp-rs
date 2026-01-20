# Semantic Scholar MCP Server (Rust)

> **The Gold Standard for Academic AI Integration**

The `semantic-scholar-mcp-rs` project is a high-performance, enterprise-grade Model Context Protocol (MCP) server that bridges Large Language Models (LLMs) with the authoritative academic data of Semantic Scholar.

Built in Rust, it serves as a robust replacement for previous Python implementations, offering superior speed (~20ms startup), reliability (mailbox architecture), and a comprehensive suite of 29 research tools.

## 🚀 Key Features

*   **29 Specialized Tools:** Covers the full research lifecycle: Discovery, Enrichment, Bibliometrics, and Network Analysis.
*   **Enterprise Reliability:** Implements a "mailbox" pattern with SSE `Last-Event-ID` tracking for zero-loss connection recovery.
*   **High Performance:**
    *   Strict 5-minute TTL caching via `moka`.
    *   Smart rate-limiting (5 req/s public, 100 req/s authenticated).
    *   ~5MB binary size.
*   **Dual Authentication:** Securely separates upstream API access (Semantic Scholar Key) from downstream client access (Bearer Token).

## 📦 Installation

### Option 1: Local (Claude Desktop)

Ideal for individual researchers.

1.  **Build:**
    ```bash
    cargo build --release
    ```

2.  **Configure:** Add to your `claude_desktop_config.json`:
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

### Option 2: Remote (Claude.ai / Teams)

Ideal for shared deployment using the "Magic Link" flow.

1.  **Deploy with Docker Compose:**
    ```yaml
    services:
      semantic-scholar-mcp:
        image: semantic-scholar-mcp
        environment:
          - SEMANTIC_SCHOLAR_API_KEY=your_scholar_key
          - MCP_SERVER_AUTH_TOKEN=your_secure_token  # generate with: openssl rand -hex 32
        ports:
          - "8000:8000"
    ```

2.  **Connect in Claude.ai:**
    Add a Custom Connector using your Magic Link:
    `https://your-domain.com?token=your_secure_token`

    *The server automatically handles token injection for secure, seamless connectivity.*

## 🛠️ Tool Capabilities

| Category | Key Tools | Description |
| :--- | :--- | :--- |
| **Discovery** | `exhaustive_search` | Deep, paginated search for comprehensive literature reviews. |
| | `citation_snowball` | Traverse citation graphs forward and backward. |
| **Enrichment** | `batch_metadata` | Retrieve details for up to 500 papers in one call. |
| | `author_search` | Find researchers and metrics (h-index, citations). |
| **Analysis** | `field_weighted_impact` | Calculate normalized impact metrics. |
| | `research_trends` | Visualize topic velocity over time. |

*All tools are engineered to handle messy academic data, strictly managing `null` values and optional fields to prevent agent crashes.*

## 🔒 Security & Best Practices

*   **Authentication:** We use a dual-token model. Your Semantic Scholar API key never leaves the server. Clients authenticate via a separate Bearer token.
*   **Operational Rules:**
    *   **Retry Middleware:** Always enabled to handle transient API failures.
    *   **Caching:** Always enabled in production to respect API quotas.

##  License

MIT