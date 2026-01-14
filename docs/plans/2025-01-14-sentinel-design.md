# Sentinel: Never-Fail Verification System

> Design document for semantic-scholar-mcp-rs robustness infrastructure

**Status:** Approved
**Created:** 2025-01-14
**Author:** Claude (brainstorming session)

## Overview

Sentinel is a multi-layer verification system ensuring the MCP server is correct at compile time, test time, and runtime. It transforms the server from "works correctly" to "provably works correctly and degrades gracefully when the world changes."

### Design Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Verification scope | Hybrid (live + fixtures) | Catches API drift while maintaining stability |
| Execution model | Multi-layer | Static, CI, and runtime checks catch different issues |
| Runtime behavior | Graceful degradation | Server stays helpful, logs anomalies |
| API key verification | Startup probe | Immediate feedback, fail fast |
| Static analysis | Full formal methods | Proptest, fuzzing, deny, audit |
| Documentation sync | Schema + contracts + golden | Three layers catch different drift types |
| Error handling | Severity + category matrix | Precise recovery per error type |
| Observability | Logs + metrics + webhooks | Full visibility into system health |
| Codebase structure | Cargo workspace | Production binary stays lean |

## Workspace Structure

```
semantic-scholar-mcp-rs/
├── Cargo.toml                    # Workspace root
├── crates/
│   ├── semantic-scholar-mcp/     # Core server (current code)
│   ├── sentinel-verify/          # API probing, schema diffing, contract tests
│   ├── sentinel-fuzz/            # Fuzzing targets for JSON parsing
│   └── sentinel-observe/         # Metrics exporter, webhook client
├── tests/
│   ├── golden/                   # Golden file fixtures
│   └── contracts/                # Contract test definitions
└── docs/
    └── plans/                    # Design documents
```

## Verification Layers

| Layer | When | What | Catches |
|-------|------|------|---------|
| **Static** | `cargo build` | Clippy pedantic, deny, audit | Code smells, vulnerabilities, bad deps |
| **Property** | `cargo test` | Proptest on models | Edge cases in parsing |
| **Contract** | CI | Live API probing | API drift, schema changes |
| **Golden** | CI | Fixture comparison | Subtle format changes |
| **Runtime** | Production | Response validation | Unexpected API behavior |
| **Fuzz** | Scheduled CI | cargo-fuzz on inputs | Crash-inducing inputs |

---

## Layer 1: Static Analysis

### Clippy Configuration

```toml
# Cargo.toml (workspace)
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }
nursery = { level = "warn", priority = -1 }

# Explicit allows (must justify each)
missing_errors_doc = "allow"
module_name_repetitions = "allow"

# Deny - never allow these
unwrap_used = "deny"
panic = "deny"
todo = "deny"
```

### Dependency Auditing

**cargo-deny** (`deny.toml`):
- License checking (allow MIT, Apache-2.0)
- Duplicate detection
- Security advisory checking
- Source restrictions (crates.io only)

**cargo-audit** runs in CI, blocks on security advisories.

### Unsafe Code Policy

```toml
[lints.rust]
unsafe_code = "forbid"
```

### Build-Time Schema Validation

Procedural macro validates input structs match `input_schema()` JSON:

```rust
#[verified_input(schema = "exhaustive_search")]
pub struct ExhaustiveSearchInput {
    pub query: String,
    pub year_start: Option<i32>,
}
```

---

## Layer 2: Property-Based Testing & Fuzzing

### Property-Based Testing (Proptest)

Key properties:

```rust
// Roundtrip serialization
proptest! {
    #[test]
    fn paper_roundtrip(paper in arb_paper()) {
        let json = serde_json::to_value(&paper)?;
        let decoded: Paper = serde_json::from_value(json)?;
        prop_assert_eq!(paper.paper_id, decoded.paper_id);
    }
}

// Malformed input never panics
proptest! {
    #[test]
    fn malformed_json_never_panics(data in any::<Vec<u8>>()) {
        let _ = serde_json::from_slice::<Paper>(&data);
    }
}
```

### Fuzzing (cargo-fuzz)

Targets in `crates/sentinel-fuzz/`:

| Target | Input | Goal |
|--------|-------|------|
| `fuzz_paper_parse` | Arbitrary bytes | Crash-free Paper deserialization |
| `fuzz_tool_input` | Arbitrary JSON | Crash-free tool input parsing |
| `fuzz_mcp_message` | Arbitrary JSON-RPC | Crash-free message handling |

---

## Layer 3: API Verification (sentinel-verify)

### Live API Probing

```rust
pub struct ApiProbe {
    client: reqwest::Client,
    api_key: Option<String>,
}

impl ApiProbe {
    pub async fn probe_endpoint(&self, endpoint: &str, params: Value) -> ProbeResult;
    pub fn diff_against_model<T: JsonSchema>(&self, probe: &ProbeResult) -> Vec<SchemaDiff>;
}
```

### Schema Diffing

```rust
pub enum SchemaDiff {
    FieldAdded { path: String, field_type: String },
    FieldRemoved { path: String },
    TypeChanged { path: String, expected: String, actual: String },
    NullabilityChanged { path: String, now_nullable: bool },
}
```

### Contract Tests

Defined in TOML:

```toml
# contracts/paper_details.toml
[contract]
name = "paper_details"
endpoint = "/paper/{paper_id}"

[request]
paper_id = "649def34f8be52c8b66281af98ae884c09aef38b"
fields = "title,year,citationCount"

[response]
status = 200
required_fields = ["paperId", "title", "year", "citationCount"]
```

### Golden File Testing

Structure:

```
tests/golden/
├── paper/
│   ├── single_paper.json
│   └── single_paper.meta.toml
├── author/
└── search/
```

Metadata tracks freshness:

```toml
[fixture]
captured_at = "2025-01-14T12:00:00Z"
endpoint = "/paper/649def34f8be52c8b66281af98ae884c09aef38b"

[expectations]
required_fields = ["paperId", "title", "year"]
```

---

## Layer 4: Runtime Verification

### Error Taxonomy

```rust
pub enum ErrorCategory {
    Network,
    RateLimit,
    Auth,
    NotFound,
    Parse,
    Upstream,
    Internal,
}

pub enum ErrorSeverity {
    Transient,
    Permanent,
    Unknown,
}
```

### Recovery Matrix

| Category | Transient | Permanent |
|----------|-----------|-----------|
| Network | Retry with backoff | Surface error |
| RateLimit | Wait + retry | Surface (key invalid) |
| Parse | Log + partial result | Surface (schema change) |
| NotFound | Return empty | Return empty |
| Upstream | Retry with backoff | Surface error |
| Auth | Surface error | Surface error |

### Response Validation

```rust
impl ResponseValidator {
    pub fn validate<T: DeserializeOwned + Validate>(
        &self,
        response: &Response,
        body: &[u8],
    ) -> ValidatedResponse<T> {
        match serde_json::from_slice::<T>(body) {
            Ok(value) => ValidatedResponse::Ok(value),
            Err(e) => {
                tracing::warn!(error = %e, "Parse anomaly - attempting recovery");
                self.attempt_lenient_parse(body)
            }
        }
    }
}
```

---

## Layer 5: API Key Verification

### Startup Probe

```rust
impl SemanticScholarClient {
    pub async fn new_verified(config: ClientConfig) -> Result<Self, StartupError> {
        let client = Self::new_inner(config)?;
        let probe_result = client.probe_api_status().await?;

        match probe_result {
            ProbeStatus::Authenticated { tier, requests_per_second } => {
                tracing::info!(
                    tier = %tier,
                    rate_limit = requests_per_second,
                    "API key validated - authenticated access"
                );
            }
            ProbeStatus::Unauthenticated { .. } if config.api_key.is_some() => {
                return Err(StartupError::InvalidApiKey);
            }
            // ...
        }
        Ok(client)
    }
}
```

Output:

```
INFO  semantic_scholar_mcp::client > API key validated tier="Partner" rate_limit=100
```

---

## Layer 6: Observability (sentinel-observe)

### Structured Logging

```rust
tracing::info!(
    endpoint = %endpoint,
    latency_ms = elapsed.as_millis(),
    cache_hit = cache_hit,
    status = %response.status(),
    "API request completed"
);
```

### Prometheus Metrics

```rust
pub struct Metrics {
    pub requests_total: Counter,
    pub request_duration: Histogram,
    pub cache_hits: Counter,
    pub cache_misses: Counter,
    pub errors_total: CounterVec,
    pub degraded_responses: Counter,
    pub active_sessions: IntGauge,
    pub api_tier: IntGauge,
}
```

Exposed at `/metrics`:

```
ss_requests_total 1523
ss_errors_total{category="rate_limit",severity="transient"} 12
ss_degraded_responses 3
```

### Webhook Alerts

```rust
pub struct WebhookAlerter {
    endpoint: Url,
    min_severity: AlertSeverity,
}

pub enum AlertSeverity {
    Info,      // Schema drift detected
    Warning,   // Elevated error rate
    Critical,  // API unreachable, auth failure
}
```

Configured via:

```bash
SENTINEL_WEBHOOK_URL=https://hooks.slack.com/services/xxx
SENTINEL_ALERT_MIN_SEVERITY=warning
```

---

## CI Integration

### GitHub Actions

```yaml
jobs:
  static:
    # clippy, deny, audit

  test:
    # cargo test + PROPTEST_CASES=1000

  contracts:
    # Live API probing (main + nightly only)

  fuzz:
    # cargo-fuzz (nightly only)

  golden:
    # Freshness check
```

### Status Checks

| Check | Blocks PR | Blocks Release |
|-------|-----------|----------------|
| Static analysis | Yes | Yes |
| Unit + Property tests | Yes | Yes |
| Contract tests | No (main only) | Yes |
| Fuzzing | No (nightly) | Yes (no crashes) |
| Golden freshness | Warn | Yes (<90 days) |

### Local Commands

```bash
cargo sentinel check          # static + unit tests
cargo sentinel verify         # all layers except fuzz
cargo sentinel refresh-golden # update fixtures
```

---

## Implementation Phases

### Phase 1: Foundation
- Cargo workspace structure
- Clippy pedantic + deny + audit
- `#[forbid(unsafe_code)]`
- CI pipeline for static checks

### Phase 2: Property Testing + Fuzzing
- Proptest for core models
- sentinel-fuzz crate
- Nightly CI fuzzing

### Phase 3: Error Taxonomy + API Key Verification
- `ClassifiedError` implementation
- Startup probe with tier logging
- Graceful degradation in client

### Phase 4: sentinel-verify
- Live API probing
- Schema diffing
- Contract tests for all 29 tools
- Golden file fixtures

### Phase 5: Observability
- Prometheus metrics endpoint
- Structured JSON logging
- Optional webhook alerting

---

## Success Criteria

- Zero clippy warnings (pedantic + nursery)
- Zero security advisories in dependencies
- 10,000+ proptest iterations pass
- 5-minute fuzz runs find no crashes
- All 29 tools have contract tests
- Golden files <90 days old
- Server logs API tier on startup
- Graceful recovery from transient errors
- `/metrics` endpoint operational
