# Semantic Scholar MCP Server (Rust)

A high-performance MCP (Model Context Protocol) server for the Semantic Scholar API, rewritten in Rust.

## Features

- **23 MCP Tools**: Discovery, enrichment, systematic review, export, bibliometrics
- **Async-first**: Built on Tokio with streaming pagination
- **Rate-limited**: Respects Semantic Scholar API limits (5 req/s, 1 req/s batch)
- **Cached**: 5-minute TTL cache reduces API calls
- **Compact**: ~5MB binary, ~20ms startup

## Installation

```bash
# Build from source
cargo build --release

# Binary located at
./target/release/semantic-scholar-mcp
```

## Usage

### Claude Desktop (stdio mode)

Add to `~/.config/claude/claude_desktop_config.json`:

```json
{
  "mcpServers": {
    "semantic-scholar": {
      "command": "/path/to/semantic-scholar-mcp",
      "args": []
    }
  }
}
```

### HTTP Mode

```bash
./semantic-scholar-mcp --transport http --port 8000
```

### With API Key (higher rate limits)

```bash
export SEMANTIC_SCHOLAR_API_KEY="your-key"
./semantic-scholar-mcp
```

## Available Tools

### Discovery
- `exhaustive_search` - Full-text search with pagination
- `recommendations` - SPECTER2 embedding similarity
- `citation_snowball` - Citation network traversal

### Enrichment
- `batch_metadata` - Bulk paper retrieval (up to 500)
- `author_search` - Author name lookup
- `author_papers` - Papers by author

### Export
- `reference_export` - RIS, BibTeX, CSV, EndNote formats

### Coming Soon
- `prisma_search` - Systematic review with PRISMA logging
- `semantic_search` - Embedding-based similarity
- `literature_review_pipeline` - Automated 3-step review
- `author_network` - Collaboration graphs
- `research_trends` - Publication trends
- `venue_analytics` - Conference metrics
- Bibliometrics tools (FWCI, h-index, citation half-life)

## Development

```bash
# Check
cargo check

# Test
cargo test

# Lint
cargo clippy

# Format
cargo fmt
```

## License

MIT
