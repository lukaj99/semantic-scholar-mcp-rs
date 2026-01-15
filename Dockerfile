# Build stage
FROM rust:1.85-slim-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./
COPY crates/semantic-scholar-mcp/Cargo.toml crates/semantic-scholar-mcp/

# Create dummy src to build dependencies
RUN mkdir -p crates/semantic-scholar-mcp/src && \
    echo "fn main() {}" > crates/semantic-scholar-mcp/src/main.rs && \
    echo "pub fn dummy() {}" > crates/semantic-scholar-mcp/src/lib.rs

# Build dependencies only
RUN cargo build --release --package semantic-scholar-mcp

# Remove dummy sources
RUN rm -rf crates/semantic-scholar-mcp/src

# Copy actual source
COPY crates/semantic-scholar-mcp/src crates/semantic-scholar-mcp/src

# Build the actual binary (touch both files to force rebuild)
RUN touch crates/semantic-scholar-mcp/src/main.rs crates/semantic-scholar-mcp/src/lib.rs && \
    cargo build --release --package semantic-scholar-mcp

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates curl && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/semantic-scholar-mcp /usr/local/bin/

# Create non-root user
RUN useradd -m -s /bin/bash appuser
USER appuser

ENV RUST_LOG=info

EXPOSE 8000

ENTRYPOINT ["semantic-scholar-mcp"]
CMD ["--transport", "http", "--port", "8000"]
