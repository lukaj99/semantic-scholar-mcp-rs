# Sentinel Phase 1: Foundation Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Establish Cargo workspace structure and comprehensive static analysis pipeline.

**Architecture:** Convert single crate to workspace with `crates/` directory. Add cargo-deny for dependency auditing. Create GitHub Actions CI with static analysis gates.

**Tech Stack:** Cargo workspaces, cargo-deny, cargo-audit, GitHub Actions

---

## Pre-Flight Check

Current state (already complete):
- `[lints.rust] unsafe_code = "forbid"` - already in Cargo.toml
- `[lints.clippy] pedantic/nursery = "warn"` - already enabled
- `.clippy.toml` - already configured

Phase 1 tasks:
1. Convert to Cargo workspace
2. Add cargo-deny configuration
3. Fix remaining clippy warnings
4. Create GitHub Actions CI pipeline

---

## Task 1: Create Workspace Structure

**Files:**
- Create: `crates/semantic-scholar-mcp/` (move all source here)
- Modify: `Cargo.toml` (convert to workspace root)
- Create: `crates/semantic-scholar-mcp/Cargo.toml` (package manifest)

### Step 1: Create crates directory

Run:
```bash
mkdir -p crates/semantic-scholar-mcp
```

### Step 2: Move source files

Run:
```bash
mv src crates/semantic-scholar-mcp/
mv tests crates/semantic-scholar-mcp/
```

### Step 3: Create workspace Cargo.toml

Replace root `Cargo.toml` with:

```toml
[workspace]
resolver = "2"
members = ["crates/*"]

[workspace.package]
version = "0.1.0"
edition = "2024"
rust-version = "1.92"
license = "MIT"
repository = "https://github.com/luka/semantic-scholar-mcp-rs"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.43", features = ["full"] }

# HTTP client with HTTP/2 support
reqwest = { version = "0.12", default-features = false, features = [
    "json",
    "http2",
    "rustls-tls",
    "gzip",
] }
reqwest-middleware = "0.4"
reqwest-retry = "0.7"

# Serialization
serde = { version = "1", features = ["derive"] }
serde_json = "1"

# Error handling
thiserror = "2"
anyhow = "1"

# CLI
clap = { version = "4", features = ["derive", "env"] }

# Logging/Tracing
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }

# Middleware
tower = { version = "0.5", features = ["limit", "retry", "timeout"] }
governor = "0.8"

# Caching
moka = { version = "0.12", features = ["future"] }

# HTTP Server
axum = { version = "0.8", features = ["json"] }
axum-extra = { version = "0.10", features = ["typed-header"] }
tower-http = { version = "0.6", features = ["cors", "trace"] }

# Async utilities
futures = "0.3"
async-stream = "0.3"
pin-project-lite = "0.2"
tokio-stream = { version = "0.1", features = ["sync"] }
async-trait = "0.1"

# Utilities
md-5 = "0.10"
url = "2"
chrono = { version = "0.4", features = ["serde"] }
regex = "1"
uuid = { version = "1", features = ["v4"] }

# Dev dependencies
tokio-test = "0.4"
wiremock = "0.6"
insta = { version = "1", features = ["json"] }
criterion = { version = "0.8", features = ["async_tokio"] }

[workspace.lints.rust]
unsafe_code = "forbid"

[workspace.lints.clippy]
all = "warn"
pedantic = "warn"
nursery = "warn"
cargo = "warn"
# Explicit allows (justified)
missing_errors_doc = "allow"        # MCP tools have uniform error handling
module_name_repetitions = "allow"   # SemanticScholarClient is intentionally clear

[profile.release]
lto = true
codegen-units = 1
strip = true
panic = "abort"
```

### Step 4: Create crate Cargo.toml

Create `crates/semantic-scholar-mcp/Cargo.toml`:

```toml
[package]
name = "semantic-scholar-mcp"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
license.workspace = true
repository.workspace = true
description = "MCP server for Semantic Scholar API - systematic reviews and bibliometrics"
keywords = ["mcp", "semantic-scholar", "academic", "research", "citations"]
categories = ["command-line-utilities", "science"]

[dependencies]
tokio.workspace = true
reqwest.workspace = true
reqwest-middleware.workspace = true
reqwest-retry.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
anyhow.workspace = true
clap.workspace = true
tracing.workspace = true
tracing-subscriber.workspace = true
tower.workspace = true
governor.workspace = true
moka.workspace = true
axum.workspace = true
axum-extra.workspace = true
tower-http.workspace = true
futures.workspace = true
async-stream.workspace = true
pin-project-lite.workspace = true
tokio-stream.workspace = true
async-trait.workspace = true
md-5.workspace = true
url.workspace = true
chrono.workspace = true
regex.workspace = true
uuid.workspace = true

[dev-dependencies]
tokio-test.workspace = true
wiremock.workspace = true
insta.workspace = true
criterion.workspace = true

[features]
default = []
integration = []

[[bin]]
name = "semantic-scholar-mcp"
path = "src/main.rs"

[lints]
workspace = true
```

### Step 5: Move clippy.toml

Run:
```bash
mv .clippy.toml crates/semantic-scholar-mcp/
```

### Step 6: Verify build

Run:
```bash
cargo build --workspace
```

Expected: Build succeeds with no errors.

### Step 7: Verify tests

Run:
```bash
cargo test --workspace
```

Expected: All 561 tests pass.

### Step 8: Commit workspace conversion

Run:
```bash
git add -A
git commit -m "refactor: convert to Cargo workspace structure

Move semantic-scholar-mcp to crates/ directory. Centralize dependencies
and lints in workspace Cargo.toml. Prepares for sentinel-verify,
sentinel-fuzz, and sentinel-observe crates.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 2: Add cargo-deny Configuration

**Files:**
- Create: `deny.toml`

### Step 1: Create deny.toml

Create `deny.toml` at workspace root:

```toml
# cargo-deny configuration
# https://embarkstudios.github.io/cargo-deny/

[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/rustsec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Zlib",
    "MPL-2.0",
    "Unicode-3.0",
    "Unicode-DFS-2016",
]
copyleft = "warn"
confidence-threshold = 0.8

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"
workspace-default-features = "allow"
external-default-features = "allow"

# Deny specific crates
deny = []

# Skip duplicate version checks for these
skip = []

# Skip checking tree for these
skip-tree = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

### Step 2: Install cargo-deny (if needed)

Run:
```bash
cargo install cargo-deny --locked
```

### Step 3: Verify deny passes

Run:
```bash
cargo deny check
```

Expected: All checks pass (possibly with warnings about duplicate versions).

### Step 4: Commit deny configuration

Run:
```bash
git add deny.toml
git commit -m "chore: add cargo-deny configuration

License checking (MIT, Apache-2.0, BSD allowed), security advisory
checking, duplicate crate detection, source restrictions.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 3: Fix Remaining Clippy Warnings

**Files:**
- Modify: `crates/semantic-scholar-mcp/tests/*.rs`

### Step 1: Run clippy and capture warnings

Run:
```bash
cargo clippy --workspace --all-targets 2>&1 | grep "warning:"
```

### Step 2: Apply automatic fixes

Run:
```bash
cargo clippy --workspace --all-targets --fix --allow-dirty
```

### Step 3: Fix remaining manual warnings

For `unused_async` in test helpers, remove the `async` keyword if no await is used:

In `crates/semantic-scholar-mcp/tests/enrichment_tests.rs` line 17, change:

```rust
async fn setup_test_context(mock_server: &MockServer) -> ToolContext {
```

to:

```rust
fn setup_test_context(mock_server: &MockServer) -> ToolContext {
```

Apply similar fixes to other test files with `unused_async` warnings.

### Step 4: Verify zero warnings

Run:
```bash
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: Build succeeds with zero warnings.

### Step 5: Verify tests still pass

Run:
```bash
cargo test --workspace
```

Expected: All tests pass.

### Step 6: Commit clippy fixes

Run:
```bash
git add -A
git commit -m "fix: resolve all clippy warnings

Remove unused async from test helpers, fix len_zero suggestions.
Now passes cargo clippy -- -D warnings.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 4: Create GitHub Actions CI Pipeline

**Files:**
- Create: `.github/workflows/ci.yml`

### Step 1: Create workflows directory

Run:
```bash
mkdir -p .github/workflows
```

### Step 2: Create CI workflow

Create `.github/workflows/ci.yml`:

```yaml
name: CI

on:
  push:
    branches: [master, main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1

jobs:
  check:
    name: Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo check --workspace --all-targets

  clippy:
    name: Clippy
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy
      - uses: Swatinem/rust-cache@v2
      - run: cargo clippy --workspace --all-targets -- -D warnings

  fmt:
    name: Format
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt
      - run: cargo fmt --all -- --check

  deny:
    name: Deny
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: EmbarkStudios/cargo-deny-action@v2

  audit:
    name: Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: rustsec/audit-check@v2
        with:
          token: ${{ secrets.GITHUB_TOKEN }}

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test --workspace

  build:
    name: Build Release
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo build --workspace --release
```

### Step 3: Verify workflow syntax

Run:
```bash
cat .github/workflows/ci.yml | head -20
```

Expected: Valid YAML displayed.

### Step 4: Commit CI workflow

Run:
```bash
git add .github/workflows/ci.yml
git commit -m "ci: add GitHub Actions pipeline

Static analysis gates: check, clippy (deny warnings), format,
cargo-deny, cargo-audit. Test and release build jobs.

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Task 5: Verify Complete Setup

### Step 1: Run full static analysis locally

Run:
```bash
cargo fmt --all -- --check && \
cargo clippy --workspace --all-targets -- -D warnings && \
cargo deny check && \
cargo test --workspace
```

Expected: All commands succeed.

### Step 2: Build release binary

Run:
```bash
cargo build --workspace --release
ls -lh target/release/semantic-scholar-mcp
```

Expected: Binary exists, ~6-7MB size.

### Step 3: Final commit (if any remaining changes)

Run:
```bash
git status
```

If clean, proceed to merge. If changes exist:

```bash
git add -A
git commit -m "chore: finalize Phase 1 setup

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>"
```

---

## Phase 1 Completion Checklist

- [ ] Cargo workspace structure in `crates/`
- [ ] `deny.toml` configured and passing
- [ ] Zero clippy warnings with `-D warnings`
- [ ] `unsafe_code = "forbid"` enforced
- [ ] GitHub Actions CI pipeline
- [ ] All 561 tests passing
- [ ] Release binary builds successfully

---

## Next Phase

Phase 2: Property Testing + Fuzzing Infrastructure
- Add proptest to core models
- Create sentinel-fuzz crate
- Nightly CI job for fuzzing
