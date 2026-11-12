# Orchestrator — Dependency & Tooling Manifest

Every third-party dependency in the project is listed here with a pinned version and a rationale.
Nothing gets added to `Cargo.toml` that is not in this table. Versions are the current stable
release at the time of writing and are refreshed by `cargo update`.

**Toolchain**: Rust stable (`rust-toolchain.toml` pins `1.99`), edition **2021**.

---

## 1. Runtime Dependencies

### 1.1 Core

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `tokio` | `1.54` | Async runtime | Full feature set: `rt-multi-thread`, `macros`, `net`, `fs`, `signal`, `sync`, `time` |
| `serde` | `1.0` | Serialization | `derive` |
| `serde-saphyr` | `2.1` | YAML parsing | **Pure Rust, no `unsafe`, and no unmaintained transitive dependency.** Replaces the deprecated `serde_yaml` (which also pulls in the archived `unsafe-libyaml`) and `serde_yaml_ng` (which pulls in the same). Schema-driven: the Rust type is the schema, so `1.2` parses as the string `"1.2"` when the field is a string |
| `serde_json` | `1.0` | JSON | `preserve_order` for stable API output |
| `thiserror` | `2.0` | Library error types | Per the code standards |
| `anyhow` | `1.0` | Application-level errors | Per the code standards |
| `tracing` | `0.1` | Structured logging | Per the code standards |
| `tracing-subscriber` | `0.3` | Log subscriber | Features: `env-filter`, `json`, `fmt` |
| `clap` | `4.6` | CLI argument parsing | Features: `derive`, `env` |
| `humantime-serde` | `1.1` | Duration strings in YAML/JSON | Gives us `30s` / `5m` in configs |

### 1.2 HTTP

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `hyper` | `1.12` | HTTP/1.1 + HTTP/2 core | v1.x API. `server`, `client`, `http1` |
| `hyper-util` | `0.1` | Hyper adapter helpers | `TokioIo`, `server`, `client-legacy` |
| `http-body-util` | `0.1` | Body combinators | `BodyExt`, `StreamBody`, `Full` |
| `http` | `1.5` | HTTP types | Matches `hyper` 1.x |
| `bytes` | `1.12` | Byte buffers | |
| `tower` | `0.5` | Middleware stack | `util`, `timeout`, `limit`, `load-shed` |
| `tower-http` | `0.7` | HTTP middleware | `trace`, `cors`, `compression`, `request-id`, `catch-panic` |
| `axum` | `0.8` | REST API server | Used for the Manager and Tenant REST APIs |
| `axum-extra` | `0.12` | Axum extras | `TypedHeader`, `cookie` |
| `matchit` | `0.9` | Route matcher | Used by the data plane for path-prefix matching (much faster than regex) |
| `tower-test` | `0.4` | Test utilities | Tower service mocking |

### 1.3 TLS

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `rustls` | `0.23` | TLS 1.2/1.3 implementation | **Per the code standards: rustls, not OpenSSL.** Pure Rust, no system dependency |
| `tokio-rustls` | `0.26` | Async TLS streams | |
| `rustls-pemfile` | `2.2` | PEM parsing | Certs, keys, CA bundles |
| `rustls-pki-types` | `1` | Shared PKI types | |
| `webpki-roots` | `1.0` | Mozilla root CA bundle | For outbound TLS to backends |
| `rcgen` | `0.14` | Test certificate generation | **Dev-dependency** — used by the test suite to mint throwaway certs |

### 1.4 DNS

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `hickory-server` | `0.26` | DNS server framework | **Replaces `trust-dns-server`** — the project was renamed to Hickory. Using the old name locks us to an unmaintained 0.23 line |
| `hickory-proto` | `0.26` | DNS protocol types | |
| `hickory-resolver` | `0.26` | Outbound resolver | Used for upstream forwarding |

### 1.5 Networking / VIP

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `rtnetlink` | `0.24` | Netlink route/address management | **Chosen over raw `netlink-packet-route`** — the raw API requires hand-building every message; `rtnetlink` gives typed address add/del and is what we actually need |
| `netlink-sys` | `0.9` | Raw netlink socket | Transitive via `rtnetlink` |
| `netlink-packet-route` | `0.34` | Netlink packet types | Transitive via `rtnetlink` |
| `nix` | `0.31` | POSIX APIs | Signals, privileges, capabilities |
| `libc` | `0.2` | Raw FFI | |
| `socket2` | `0.6` | Socket options | `SO_REUSEPORT`, `IP_TRANSPARENT` |

### 1.6 gRPC Sync Channel

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `tonic` | `0.14` | gRPC client + server | `transport` feature (HTTP/2 + TLS) |
| `tonic-prost` | `0.14` | The Protobuf codec | tonic 0.14 split the codec out of `tonic` itself. The generated code references it |
| `tonic-prost-build` | `0.14` | Build-time codegen | Replaces the split `tonic-build` / `prost-build` crates in tonic 0.14 |
| `prost` | `0.14` | Protobuf runtime | |
| `prost-types` | `0.14` | Well-known protobuf types | |
| `protoc-bin-vendored` | `3.3` | A vendored `protoc` | **Build-dependency only.** Makes the build hermetic — CI does not need `protoc` installed |
| `tokio-stream` | `0.1` | Stream adapters | `wrappers` for `Stream` messages |

### 1.7 State / Storage

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `arc-swap` | `1.9` | Lock-free atomic config swap | Implements G9.4 (atomic routing-table swap) with zero downtime and no reader-side locking |
| `moka` | `0.12` | Concurrent cache with TTL + LRU | Sticky-session store (G5.5 expiry, G5.7) and health-state cache |
| `ahash` | `0.8` | Fast hasher | Session key hashing |
| `dashmap` | `6.2` | Concurrent map | Endpoint health tables, rollout state |
| `parking_lot` | `0.13` | Faster mutexes | For non-async critical sections |

### 1.8 Security / Auth

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `argon2` | `0.6` | Password hashing | Argon2id, per the API reference |
| `rand` | `0.10` | CSPRNG | Token generation, salts |
| `base64` | `0.23` | Encoding | Token / hash encoding |
| `subtle` | `2.6` | Constant-time comparison | Token comparison |

### 1.9 Observability

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `metrics` | `0.24` | Metrics facade | Facade over the exporter |
| `metrics-exporter-prometheus` | `0.18` | Prometheus exporter | |
| `hdrhistogram` | `7.6` | Latency histogram | Backs the p99 latency metric for auto-rollback (G4.6) |

### 1.10 Misc

| Crate | Version | Purpose | Notes |
|-------|---------|---------|-------|
| `notify` | `8.2` | Filesystem watching | Cross-platform; `notify` + inotify on Linux |
| `tokio-util` | `0.7` | Cancellation tokens | Graceful shutdown |
| `futures` | `0.3` | Async combinators | |
| `tokio-graceful-shutdown` | `0.20` | Signal handling | SIGTERM → drain → exit |
| `uuid` | `1.29` | IDs | Rollout IDs, session IDs |
| `chrono` | `0.4` | Timestamps | |

---

## 2. Dev Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tempfile` | `3.27` | Temp dirs/files in tests |
| `tokio-test` | `0.4` | Tokio time/pause testing |
| `proptest` | `1.12` | Property-based testing of the weight/route logic |
| `criterion` | `0.8` | Benchmark harness |
| `rcgen` | `0.14` | Throwaway TLS certificates |
| `wiremock` | `0.6` | HTTP mocking for backend tests |
| `serial_test` | `3.2` | Serialise tests that bind fixed ports |
| `tracing-subscriber` | `0.3` | Capturing logs in tests |

---

## 3. GUI Dependencies (npm)

| Package | Purpose | Notes |
|---------|---------|-------|
| `react` / `react-dom` | UI runtime | |
| `typescript` | Type safety | `strict: true` |
| `vite` | Build tool + dev server | |
| `react-router-dom` | Client-side routing | Login / Dashboard / Tenant detail / Settings |
| `@tanstack/react-query` | Server-state fetching + polling | Health dashboard polling |
| `zustand` | Client state | Session + tenant selection |
| `d3-sankey` | Sankey diagram | The G8.4 requirement |
| `d3-hierarchy` | Tree view layout | The G8.3 requirement |
| `@mui/material` | Component library | Consistent, accessible primitives |
| `@mui/x-tree-view` | Tree view component | |
| `@tanstack/react-table` | DNS zone editor table | |
| `react-hook-form` + `zod` | Form handling + validation | |
| `recharts` | Health sparkline charts | |
| `vitest` + `@testing-library/react` | Unit/component tests | |
| `playwright` | End-to-end GUI tests | |
| `eslint` + `typescript-eslint` + `prettier` | Linting / formatting | |
| `npm` | Package manager | Ships with Node. `npm ci` in CI for a reproducible install |

---

## 4. Tooling Versions

| Tool | Version | Config file |
|------|---------|-------------|
| Rust | `1.99` stable | `rust-toolchain.toml` |
| `rustfmt` | stable, default profile + nightly-only `group_imports` | `rustfmt.toml` |
| `clippy` | stable, `-D warnings` | `clippy.toml` |
| Node.js | `22` LTS | `.nvmrc` |
| npm | `10` (ships with Node 22) | `package-lock.json` |
| `cargo-deny` | `0.16` | `deny.toml` — licence/advisory policy |
| `cargo-audit` | `0.21` | Run in CI |
| `taplo` | `0.9` | YAML/TOML formatting |

---

## 5. Licence Policy

All dependencies must be **MIT**, **Apache-2.0**, **BSD-3-Clause**, or **ISC**. `cargo deny` runs in CI and fails the build on:

- A licence outside the allow-list
- A crate with a **GPL** or **AGPL** licence
- A dependency that pulls in `unsafe` code, where a pure-Rust alternative exists
- A crate with **no** licence field
- A crate with a licence file that contradicts the SPDX identifier
- Any RUSTSEC advisory that is not `ignore`d with a written justification
- More than 5 direct dependencies with more than 100 transitive dependencies in total

`cargo deny` config lives in `deny.toml`.

---

## 6. Dependency Update Policy

- **Renovate** (or `cargo-update` + Dependabot) proposes updates weekly.
- Patch and minor updates auto-merge once CI passes.
- Major updates require a human review — every crate listed in this document that has had a
  breaking release must be checked against this table.
- Any crate added to a `Cargo.toml` must be added to this document in the same PR, or CI fails on
  the `dependency-manifest` check.
