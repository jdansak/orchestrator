# Orchestrator — Non-Functional Requirements & Definition of Done

The goals in `02-goals-and-acceptance-criteria.md` cover functional behaviour. This document covers
the qualities the system must have, and the bar that must be cleared for a release.

---

## 1. Performance Budgets

| # | Metric | Target | How it is measured |
|---|--------|--------|--------------------|
| NFR-P1 | Tenant startup → first request served | **< 1s** | Timestamped at process start and at listener bind |
| NFR-P2 | Config reload (SIGHUP → new routing table live) | **< 100ms** | `orchestrator_config_reload_duration_seconds` histogram, p99 |
| NFR-P3 | Proxy overhead per request | **< 1ms** | Proxy latency minus upstream latency, p50 at 1k rps |
| NFR-P4 | Memory per tenant | **< 256MB** steady, **< 512MB** peak | RSS under sustained load |
| NFR-P5 | Concurrent connections per tenant | **10,000+** | Load test with 10k in-flight requests |
| NFR-P6 | VIP failover time | **< 1s** | System test, worker SIGKILL → standby holds VIP |
| NFR-P7 | DNS query latency (authoritative) | **< 5ms** | p99, measured locally |
| NFR-P8 | DNS query latency (forwarded) | **< 100ms** | p99, dominated by upstream |
| NFR-P9 | Manager API response | **< 100ms** | p99 for a tenant list |
| NFR-P10 | Rollout step transition | **< 1s** | From "advance" to traffic reflecting the new split |

### Sustained throughput

| Scenario | Target |
|----------|--------|
| HTTP proxy, 2 vCPU tenant, 2 backends | **5,000 rps** at p99 < 10ms |
| HTTPS proxy (TLS 1.3, HTTP/2) | **2,000 rps** at p99 < 15ms |
| DNS (authoritative, cached) | **50,000 qps** single-threaded |

---

## 2. Availability & Resilience

| # | Requirement |
|---|-------------|
| NFR-R1 | A tenant process crash must not affect any other tenant (G1) |
| NFR-R2 | Killing the active worker must not drop more than 3 in-flight requests (TC-022) |
| NFR-R3 | An invalid config change must leave the running config untouched (G9.3) |
| NFR-R4 | A backend that becomes unhealthy is removed from the routing table within `unhealthy_threshold × interval` |
| NFR-R5 | The manager must restart a crashed tenant process within 10s (systemd `Restart=on-failure`, `RestartSec=3`) |
| NFR-R6 | On `SIGTERM`, in-flight requests drain for up to 30s before the process exits |
| NFR-R7 | On `SIGTERM`, the VIP is released after draining so the peer can take over cleanly |
| NFR-R8 | A DNS upstream failure must not block authoritative answers |
| NFR-R9 | The GUI must remain usable when a tenant is down — it shows the tenant as unhealthy, it does not error out |

---

## 3. Scalability Limits

These are the documented ceilings. Exceeding them is not a silent failure — the system logs a
warning and refuses the operation.

| # | Limit | Value | Behaviour when exceeded |
|---|-------|-------|--------------------------|
| NFR-S1 | Tenants per host | 50 | Manager refuses to register more, returns `409` |
| NFR-S2 | Routes per tenant | 1,000 | Config rejected: `TooManyRoutes` |
| NFR-S3 | Hierarchy leaves (endpoints) per tenant | 10,000 | Config rejected: `TooManyEndpoints` |
| NFR-S4 | Concurrent connections per tenant | 65,535 | Connections are refused with `503` |
| NFR-S5 | Sticky sessions per tenant | 100,000 | LRU eviction (oldest first) |
| NFR-S6 | Config file size | 4 MiB | Config rejected: `ConfigTooLarge` |
| NFR-S7 | DNS records per zone | 10,000 | Config rejected |
| NFR-S8 | Rollout history entries retained | 100 | Older entries pruned |

---

## 4. Security Requirements

| # | Requirement |
|---|-------------|
| NFR-SEC1 | TLS termination uses **rustls**. No OpenSSL linkage anywhere in the binary |
| NFR-SEC2 | TLS 1.0 and TLS 1.1 are never negotiable. The minimum is configurable but the floor is 1.2 and a config below that is rejected |
| NFR-SEC3 | No secret material is stored in any `tenant.yaml` or `registry.yaml`. Cert/key paths reference files on disk |
| NFR-SEC4 | Passwords are stored as Argon2id hashes in a `0600` file, never in the config tree |
| NFR-SEC5 | The REST API rejects unauthenticated requests with `401`. Only `/api/login`, `/api/health`, and `/api/version` are public |
| NFR-SEC6 | Login errors never disclose whether the username exists |
| NFR-SEC7 | The metrics listener binds to `127.0.0.1` by default and is never exposed publicly |
| NFR-SEC8 | The DNS server does not answer `AXFR`/`IXFR` zone transfers |
| NFR-SEC9 | The DNS server rejects `ANY` queries that would leak the full zone |
| NFR-SEC10 | The tenant process runs as a non-root user where the platform allows it. Root is required only for VIP manipulation and privileged ports |
| NFR-SEC11 | systemd hardening is applied to every generated unit: `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, `RestrictAddressFamilies` |
| NFR-SEC12 | A config file with mode wider than `0644` is refused with a warning |
| NFR-SEC13 | All dependencies pass `cargo deny`'s licence allow-list and have no outstanding RUSTSEC advisories |
| NFR-SEC14 | Backend TLS verification is on by default and cannot be silently disabled without an explicit config flag |
| NFR-SEC15 | Request headers added by the proxy (`X-Forwarded-For`, `X-Forwarded-Proto`, `X-Real-IP`) overwrite any client-supplied value of the same name, never append to an attacker-controlled value |

---

## 5. Observability Requirements

| # | Requirement |
|---|-------------|
| NFR-O1 | Every log line is structured JSON in production, with `tenant`, `worker`, and `request_id` fields |
| NFR-O2 | Logs are written to stdout for systemd journal capture, and to `/var/log/orchestrator/<tenant>.log` when file logging is enabled |
| NFR-O3 | The log level is configurable per tenant and changeable at runtime without a restart |
| NFR-O4 | Prometheus metrics are exposed on a separate listener from the API |
| NFR-O5 | Every proxied request emits exactly one `info` line with method, path, status, duration, route, and endpoint |
| NFR-O6 | Configuration reloads emit an `info` line with the outcome and duration |
| NFR-O7 | VIP acquisition and release emit `info` lines naming the interface and address |
| NFR-O8 | Rollout transitions and rollbacks emit `info`/`warn` lines and a webhook notification |

---

## 6. Portability & Platform Support

| # | Requirement |
|---|-------------|
| NFR-PT1 | **The target platform is Linux** (kernel `≥ 5.10`, systemd `≥ 245`). Everything else is a development convenience |
| NFR-PT2 | The workspace compiles on macOS and Windows for editor support and for running unit tests, but the VIP and data-plane integration tests are Linux-only |
| NFR-PT3 | Linux-only functionality is behind `#[cfg(target_os = "linux")]` with a stub that returns `Err(VipError::UnsupportedPlatform)` elsewhere, so the crate compiles everywhere |
| NFR-PT4 | The GUI is platform-independent and builds on any machine with Node 22 |
| NFR-PT5 | A container image is published for the tenant binary so a deployment does not require installing to the host |

---

## 7. Compatibility & Migration

| # | Requirement |
|---|-------------|
| NFR-C1 | The config schema version is explicit and checked (see `07-config-schema.md` §1) |
| NFR-C2 | A config written for a newer schema version is refused, never partially applied |
| NFR-C3 | The `orchestrator migrate` subcommand upgrades configs losslessly |
| NFR-C4 | The manager tolerates a tenant whose `tenant.yaml` is absent — it reports the tenant as `config-missing` rather than crash-looping |
| NFR-C5 | The manager tolerates a partially-written `registry.yaml` — it keeps the last valid registry in memory and logs the parse error |
| NFR-C6 | The product follows semantic versioning. A breaking REST API change requires a major version and a documented migration path |

---

## 8. Definition of Done

A task is **done** when all of the following are true. This is the checklist applied in code review.

### Code

- [ ] Implemented against the acceptance criteria for the task
- [ ] Unit tests cover the new logic; edge cases are covered, not just the happy path
- [ ] Integration tests exist for anything crossing a module boundary
- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy -- -D warnings` passes with **no** new `#[allow]` attributes
- [ ] No `unwrap()`, `expect()`, or `panic!()` outside tests and `main()`. Panics are documented where they are genuinely invariant
- [ ] Every public function has a doc comment
- [ ] Errors use the project's `thiserror` types and include enough context to diagnose from the log alone
- [ ] No `blocking` call inside an async context. No `std::fs` in an async fn
- [ ] Blocking work is moved to `tokio::task::spawn_blocking`
- [ ] No new dependency without an entry in `09-dependencies.md`
- [ ] Logs are structured; no `println!` outside of `main()` and tests

### Tests

- [ ] The relevant `TC-xxx` criteria are covered by an automated test
- [ ] The test fails when the feature is broken (verified by temporarily reverting the change)
- [ ] The test does not sleep for real time to satisfy a timing requirement
- [ ] Coverage does not drop below the threshold for the crate

### Documentation

- [ ] The `docs/` content is updated if behaviour changed
- [ ] `CHANGELOG.md` has an entry under `Unreleased`
- [ ] Any new config field is documented in `07-config-schema.md`
- [ ] Any new API endpoint is documented in `08-api-reference.md`
- [ ] The tracker in `06-tracker.md` is updated

### Operational

- [ ] The change is safe to roll back — no destructive migration without a documented rollback path
- [ ] Metrics or logs exist to observe the feature working in production
- [ ] Failure modes are handled and logged, not panicked

---

## 9. Release Criteria

A release is cut when:

- [ ] Every phase in `05-implementation-plan.md` is complete
- [ ] Every `TC-xxx` criterion has a passing automated test
- [ ] `cargo deny` reports zero advisories and zero licence violations
- [ ] The full test suite passes on Linux and the GUI test suite passes
- [ ] `cargo build --release` produces binaries on both `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`
- [ ] RPM and DEB packages build and install cleanly on a clean VM
- [ ] `docs/getting-started.md` has been followed end-to-end on a fresh machine by someone who did not write it
- [ ] A security review of the TLS, auth, and config-parsing paths has been done

### Versioning

| Change | Version bump |
|--------|--------------|
| Breaking REST API change | major |
| Breaking config schema change | major |
| New feature, backward compatible | minor |
| Bug fix, internal change | patch |
