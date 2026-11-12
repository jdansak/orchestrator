# Orchestrator — Code Standards

## Language and Tooling

| Item | Standard |
|------|----------|
| **Language** | Rust (2021 edition) |
| **Rust version** | 1.99 stable — pinned by `rust-toolchain.toml` |
| **Cargo** | Standard Cargo workspace, resolver 2 |
| **Formatter** | `rustfmt` (config in `rustfmt.toml`) |
| **Linter** | `clippy` (strict mode, `-D warnings`) |
| **Build** | `cargo build --release` |
| **Test** | `cargo test --all` |
| **Docs** | `cargo doc --no-deps` |
| **Dependencies** | Must appear in [`09-dependencies.md`](09-dependencies.md) |

> **Do not use `serde_yaml`.** It is deprecated upstream, and it pulls in the archived
> `unsafe-libyaml`. Use `serde-saphyr` — pure Rust, no `unsafe`, and it denies duplicate keys.
> **Do not use `trust-dns-server`.** The crate was renamed to `hickory-server`; the old name is
> stuck on an unmaintained 0.23 line.
> **Do not use raw `netlink-packet-route`** for address management. Use `rtnetlink`.

### rustfmt.toml

```toml
edition = "2021"
max_width = 100
use_field_init_shorthand = true
use_try_shorthand = true
newline_style = "Unix"
# Nightly-only options, applied via `cargo +nightly fmt`:
# imports_granularity = "Module"
# group_imports = "StdExternalCrate"
```

### clippy.toml

```toml
msrv = "1.99"
disallowed-methods = [
    { path = "std::env::set_var", reason = "not thread-safe" },
    { path = "std::process::exit", reason = "use graceful shutdown instead" },
]
```

## Dependency Rules

- No new dependency without an entry in `09-dependencies.md` — enforced by
  `scripts/check-manifest.sh` in CI
- No `unwrap()`, `expect()`, or `panic!()` outside tests and `main()`
- `cargo deny` runs in CI: licence allow-list plus RUSTSEC advisories
- Prefer a crate already in the dependency list over a new one

## Naming Conventions

### Module Names

```
snake_case
```

Examples:
```
config_engine.rs
data_plane/http_proxy.rs
vip_manager.rs
sync_channel.rs
```

### Type Names

```
PascalCase
```

Examples:
```
struct RouteNode { ... }
enum TrafficStrategy { BlueGreen, Canary }
trait ConfigLoader { ... }
```

### Function and Method Names

```
snake_case
```

Examples:
```
fn load_config(path: &str) -> Result<TenantConfig>
fn validate_weights(config: &TenantConfig) -> Result<()>
fn acquire_vip(&mut self) -> Result<()>
```

### Variable Names

```
snake_case
```

Examples:
```
let weight = 70u32;
let target_cluster = "main-k8s";
let health_check_interval = Duration::from_secs(10);
```

### Constant Names

```
SCREAMING_SNAKE_CASE
```

Examples:
```
const DEFAULT_VIP_PRIORITY: u8 = 100;
const DEFAULT_HEALTH_CHECK_INTERVAL: Duration = Duration::from_secs(10);
const MAX_RETRIES: u32 = 3;
```

### Trait Names

```
PascalCase (often ends in -able or -er)
```

Examples:
```
trait ConfigLoader { ... }
trait TrafficRouter { ... }
trait HealthChecker { ... }
```

### Error Types

```
PascalCase, always end in Error
```

Examples:
```
struct ConfigError { ... }
struct VipError { ... }
struct SyncError { ... }
```

### Module Constants

```
SCREAMING_SNAKE_CASE
```

Examples:
```
pub const DEFAULT_BIND_PORT: u16 = 443;
pub const VIP_PRIORITY_DEFAULT: u8 = 100;
```

### Private Items

No special prefix. Rust visibility is handled by `pub`/`pub(crate)`.

## Error Handling

```rust
// Use thiserror for library errors
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to parse config: {0}")]
    ParseError(#[from] Box<serde_saphyr::Error>),
    #[error("invalid weight: node '{node}' weight {weight} must be 0-100")]
    InvalidWeight { node: String, weight: u32 },
    #[error("weights do not sum to 100: node '{node}' sum is {sum}")]
    WeightSumMismatch { node: String, sum: u32 },
}

// Use anyhow for application-level errors where context matters
use anyhow::{Context, Result};

async fn load_config(path: &str) -> Result<TenantConfig> {
    let content = tokio::fs::read_to_string(path)
        .await
        .context(format!("failed to read config file: {}", path))?;
    let config: TenantConfig = serde_saphyr::from_str(&content)
        .context("failed to parse config YAML")?;
    validate_config(&config)?;
    Ok(config)
}
```

## Async Patterns

```rust
// Use tokio for async runtime
use tokio::time::{timeout, Duration};

// Never block the async runtime
// Bad: let result = std::fs::read_to_string(path);
// Good:
let result = tokio::fs::read_to_string(path).await;

// Use tokio::spawn for concurrent tasks
let handle = tokio::spawn(async move {
    // background task
});

// Use select! for racing tasks
tokio::select! {
    result = worker.sync_loop() => { /* handle */ }
    _ = tokio::time::sleep(Duration::from_secs(30)) => { /* heartbeat */ }
}
```

## Logging

```rust
// Use tracing for structured logging
use tracing::{info, warn, error, debug, instrument};

#[instrument(skip(self, config), fields(tenant = %self.tenant_name))]
async fn reload_config(&mut self, config: TenantConfig) -> Result<()> {
    info!("config change detected");
    debug!(?config, "new config");

    if self.validate(&config).is_err() {
        warn!("config validation failed, rejecting");
        return Err(ConfigError::ValidationFailed);
    }

    info!("config reloaded successfully");
    Ok(())
}
```

### Log Levels

| Level | When to use |
|-------|-------------|
| `error` | System is in an error state, action required |
| `warn` | Something unexpected happened but system is okay |
| `info` | Normal operational events (config loaded, VIP acquired, etc.) |
| `debug` | Detailed diagnostic information |
| `trace` | Extremely detailed information (packet-level) |

## Testing

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_weights_sum_to_100() {
        let config = TenantConfig {
            regions: vec![
                RegionConfig { name: "us-east".into(), weight: 60, /* ... */ },
                RegionConfig { name: "eu-west".into(), weight: 40, /* ... */ },
            ],
        };
        assert!(validate_weights(&config).is_ok());
    }

    #[test]
    fn test_validate_weights_exceed_100() {
        let config = TenantConfig {
            regions: vec![
                RegionConfig { weight: 60, /* ... */ },
                RegionConfig { weight: 50, /* ... */ },
            ],
        };
        assert!(validate_weights(&config).is_err());
    }
}
```

### Test Requirements

See [`10-test-strategy.md`](10-test-strategy.md) for the full strategy. In short:

- Every public function has unit tests
- Integration tests for cross-module interactions
- Configuration validation tests cover every rule V1–V17
- No test depends on the network or the real filesystem — use `orchestrator-testkit` fixtures
- **No test sleeps for real time** to satisfy a timing requirement. Use `FakeClock` or
  `tokio::time::pause()`. The only real sleeps allowed are in the system tests that measure
  genuine wall-clock VIP failover
- Every `TC-xxx` acceptance criterion has an automated test. CI fails if any criterion has none

## CI/CD Requirements

```yaml
# Pipeline stages:
# 1. Build     — cargo build --release
# 2. Test      — cargo test --all
# 3. Lint      — cargo clippy -- -D warnings
# 4. Format    — cargo fmt --check
# 5. Docs      — cargo doc --no-deps
```

## File Structure

```
orchestrator/
├── Cargo.toml                        # Workspace manifest
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml
│
├── crates/
│   ├── orchestrator-core/            # Shared types and config model
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs                # Re-exports
│   │       ├── error.rs              # Error types
│   │       ├── config/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs          # Config model
│   │       │   ├── parse.rs          # YAML → config
│   │       │   ├── validate.rs       # Validation rules
│   │       │   └── duration.rs       # "30s" parsing
│   │       ├── hierarchy.rs          # The 4-level tree
│   │       ├── weights.rs            # Weight composition + selection
│   │       ├── routing.rs            # Route matching
│   │       ├── affinity.rs           # Session key derivation
│   │       ├── rollout.rs            # Rollout model + state machine
│   │       ├── dns.rs                # DNS record types
│   │       ├── vip.rs                # VIP types
│   │       ├── metrics.rs            # Metric name constants
│   │       └── version.rs            # SCHEMA_VERSION
│   │
│   ├── orchestrator/                 # Per-tenant binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs               # Entry point
│   │       ├── cli.rs                # Argument parsing
│   │       ├── runtime.rs            # Tokio runtime + signal handling
│   │       ├── control_plane/
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs         # Config engine + hot reload
│   │       │   ├── health.rs         # Health checker
│   │       │   ├── vip_manager.rs    # VIP management
│   │       │   ├── rolling_update.rs # Rolling update logic
│   │       │   ├── session_store.rs  # Sticky sessions
│   │       │   └── api.rs            # Tenant REST API
│   │       ├── data_plane/
│   │       │   ├── mod.rs
│   │       │   ├── http.rs           # HTTP/HTTPS proxy
│   │       │   ├── tcp.rs            # TCP proxy
│   │       │   ├── udp.rs            # UDP proxy
│   │       │   ├── tls.rs            # TLS acceptor
│   │       │   └── headers.rs        # X-Forwarded-*, X-Request-Id
│   │       ├── sync/
│   │       │   ├── mod.rs
│   │       │   ├── channel.rs        # gRPC sync channel
│   │       │   ├── messages.rs       # Sync message types
│   │       │   └── state_machine.rs  # Active/passive state machine
│   │       ├── dns_server/
│   │       │   ├── mod.rs
│   │       │   ├── server.rs         # DNS server
│   │       │   └── zone.rs           # Zone management
│   │       ├── vip/
│   │       │   ├── mod.rs
│   │       │   └── netlink.rs        # Linux VIP binding
│   │       └── metrics.rs            # Prometheus metrics
│   │
│   ├── orchestrator-manager/         # Manager binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs               # Entry point
│   │       ├── registry.rs           # Tenant registry
│   │       ├── vip_pool.rs           # VIP pool allocator
│   │       ├── supervisor.rs         # Process lifecycle
│   │       ├── config_watcher.rs     # Filesystem watcher
│   │       ├── auth.rs               # Credentials + tokens
│   │       └── api.rs                # Manager REST API
│   │
│   ├── orchestrator-proto/           # gRPC/protobuf definitions
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   └── proto/sync.proto
│   │
│   └── orchestrator-testkit/         # Shared test fixtures (dev-only)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── mock_backend.rs
│           ├── load_generator.rs
│           ├── test_config.rs
│           ├── fake_clock.rs
│           └── cluster.rs            # netns TestCluster (Linux)
│
├── orchestrator-gui/                 # Web GUI
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── e2e/                          # Playwright
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── auth/
│       │   ├── AuthContext.tsx
│       │   └── LoginPage.tsx
│       ├── components/
│       │   ├── TenantSwitcher.tsx
│       │   ├── TreeView.tsx
│       │   ├── SankeyDiagram.tsx
│       │   ├── RollingUpdate.tsx
│       │   ├── DnsZoneEditor.tsx
│       │   └── HealthDashboard.tsx
│       ├── pages/
│       │   ├── Dashboard.tsx
│       │   ├── TenantDetail.tsx
│       │   ├── Rollouts.tsx
│       │   ├── Dns.tsx
│       │   └── Settings.tsx
│       ├── api/
│       │   ├── client.ts             # API client
│       │   ├── types.ts
│       │   └── endpoints.ts
│       └── hooks/
│
├── systemd/                          # systemd service templates
│   ├── orchestrator-manager.service
│   └── orchestrator-tenant.service.template
│
├── packaging/                        # RPM/DEB packaging
│   ├── deb/
│   ├── rpm/orchestrator.spec
│   └── Makefile
│
├── docs/
│   ├── architecture.md
│   ├── getting-started.md
│   ├── configuration.md
│   ├── api-reference.md
│   ├── deployment.md
│   ├── operations.md
│   └── troubleshooting.md
│
├── examples/
│   ├── tenant.yaml
│   └── registry.yaml
│
├── system-tests/                     # Privileged Linux tests
│   └── tests/
│
└── scripts/
    ├── bootstrap.sh
    └── check-manifest.sh
```

## Module Dependencies

```
orchestrator-core (no dependencies on other orchestrator modules)
    ↑                    ↑
    │                    │
orchestrator        orchestrator-manager      orchestrator-proto
                                              (no internal deps)
                                                    ↑
                                          orchestrator + manager

orchestrator-testkit ──▶ orchestrator-core   (dev-dependency only)
```

**No circular dependencies between modules.**

- `orchestrator` and `orchestrator-manager` are **siblings**. Neither depends on the other — the
  manager talks to tenants over the network, not over Rust types.
- `orchestrator-proto` depends on nothing internal.
- CI verifies the graph with `cargo metadata` plus a cycle check.

## API Design

All APIs follow REST conventions. The full contract — request/response bodies, status codes, and
error shapes — is in [`08-api-reference.md`](08-api-reference.md).

### Manager API

```
GET    /api/tenants                  # List tenants
POST   /api/tenants                  # Create tenant
GET    /api/tenants/:name            # Get tenant details
PATCH  /api/tenants/:name            # Update tenant registry entry
DELETE /api/tenants/:name            # Delete tenant
GET    /api/tenants/:name/health     # Health status
POST   /api/tenants/:name/start      # Start the tenant's services
POST   /api/tenants/:name/stop       # Stop the tenant's services
POST   /api/tenants/:name/restart    # Restart the tenant's services
POST   /api/tenants/:name/reload     # Trigger config reload (SIGHUP)
GET    /api/tenants/:name/metrics    # Metrics
GET    /api/vip-pool                 # VIP pool assignments
GET    /api/health                   # Manager liveness  (public)
GET    /api/version                  # Build version     (public)
```

### Tenant API

```
GET    /api/config                   # The active, validated config
GET    /api/routes                   # The active routing table
GET    /api/routes/:name             # One route
GET    /api/hierarchy                # The hierarchy tree
GET    /api/hierarchy/effective-weights
GET    /api/endpoints                # Per-endpoint health
GET    /api/worker                   # This worker's role and VIP state
GET    /api/rollout                  # Current rollout state
POST   /api/rollout                  # Start a rollout
POST   /api/rollout/advance          # Advance one step
POST   /api/rollout/rollback         # Trigger an immediate rollback
POST   /api/rollout/promote          # Skip to 100% green
POST   /api/rollout/pause            # Pause an auto-advancing rollout
POST   /api/rollout/resume           # Resume a paused rollout
GET    /api/dns/zones                # DNS zones and records
PUT    /api/dns/zones/:zone          # Replace a zone's records
POST   /api/dns/zones/:zone/records  # Add a record
DELETE /api/dns/zones/:zone/records/:name
GET    /api/sessions                 # Sticky-session stats
DELETE /api/sessions                 # Flush the session store
GET    /api/events                   # Server-sent event stream
GET    /api/health                   # Tenant liveness     (public)
```

### Errors

RFC 9457 problem details. A single shape for every error:

```json
{
  "type": "https://orchestrator.dev/errors/validation-failed",
  "title": "Validation failed",
  "status": 422,
  "detail": "config validation produced 2 errors",
  "errors": [ { "field": "...", "message": "..." } ],
  "instance": "/api/tenants/app-frontend/reload"
}
```

## Security Standards

- **TLS**: rustls (not OpenSSL) — pure Rust, no system dependency
- **Secrets**: No secrets in config files. Cert paths reference files on disk. Passwords live in a
  separate `0600` credential file, never in `tenant.yaml` or `registry.yaml`
- **Passwords**: Argon2id with a per-user random salt
- **Tokens**: Opaque, cryptographically random, constant-time comparison
- **Network**: Workers only listen on configured interfaces. No wildcard binds for sensitive
  services. The metrics listener binds to loopback by default
- **Privileges**: Run as non-root user where possible. VIP binding requires `CAP_NET_ADMIN`
- **Input validation**: All user input (config, API) validated before processing. Request bodies
  are size-limited
- **Error messages**: No internal details in error messages exposed to clients. Log the detail,
  return a generic message
- **DNS**: Zone transfers (`AXFR`/`IXFR`) are refused
- **Headers**: The proxy **overwrites** client-supplied `X-Forwarded-*` headers rather than
  appending to them

## Performance Standards

The authoritative budgets are in
[`11-non-functional-requirements.md`](11-non-functional-requirements.md) §1. Summary:

| Metric | Target |
|--------|--------|
| Startup time | < 1 second per tenant |
| Config reload time | < 100ms |
| Request latency overhead | < 1ms per hop (proxy) |
| Memory per tenant | < 256MB typical, < 512MB peak |
| Concurrent connections | 10,000+ per tenant |
| VIP failover time | < 1 second |
| HTTP throughput | 5,000 rps per tenant |
