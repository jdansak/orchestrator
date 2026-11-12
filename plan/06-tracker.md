# Orchestrator — Tracker

Running checklist of all tasks and test cases, by phase and by goal.

## Status Legend

| Symbol | Meaning |
|--------|---------|
| `⬜` | Not started |
| `🔨` | In progress |
| `✅` | Complete |
| `⏸️` | Blocked |
| `❌` | Failed |

> **Updated 2026-11-12** after the gap review. Added Phase 0 and Phase 9, added the tasks that
> were referenced by the goals but never scheduled (health checker, session affinity,
> authentication, test harness, packaging), and added Goals 10 and 11 with their test cases.

---

## By Phase

### Phase 0: Foundation

- [ ] 0.1 Repo scaffolding: Cargo workspace, crate skeletons, toolchain/fmt/clippy/deny configs ⬜
- [ ] 0.2 CI pipeline + `scripts/check-manifest.sh` ⬜
- [ ] 0.3 Testkit: `MockBackend` ⬜
- [ ] 0.4 Testkit: `LoadGenerator` + distribution helpers ⬜
- [ ] 0.5 Testkit: `TestConfig` builder + `FakeClock` ⬜
- [ ] 0.6 Repository governance: README, LICENSE, CONTRIBUTING, CODE_OF_CONDUCT, SECURITY, CHANGELOG, templates ⬜
- [ ] 0.7 GUI scaffold: Vite + React + TypeScript + lint + test config ⬜
- [ ] 0.8 `docs/` skeleton, `examples/`, `Makefile`, `scripts/bootstrap.sh` ⬜

### Phase 1: Core Engine

- [ ] 1.1 Define orchestrator-core types (Tenant, Region, Datacenter, Cluster, Endpoint) ⬜
- [ ] 1.2 Config schema model + duration/size parsing ⬜
- [ ] 1.3 YAML config parser (`serde-saphyr`, deny unknown fields) ⬜
- [ ] 1.4 Validator (rules V1–V17, collect all errors) ⬜
- [ ] 1.5 Hierarchy model + multiplicative weight composition ⬜
- [ ] 1.6 Weighted selection algorithm ⬜
- [ ] 1.7 Route matching (prefix/exact, longest-prefix-wins, path boundaries) ⬜
- [ ] 1.8 HTTP proxy (hyper 1.x) with pooling, timeouts, retries, forwarded headers ⬜
- [ ] 1.9 Backend health checker (HTTP/TCP probes, thresholds) ⬜
- [ ] 1.10 Session affinity store (cookie/header/client-ip/none, TTL, LRU) ⬜
- [ ] 1.11 Structured logging (`tracing` + `tracing-subscriber`) ⬜
- [ ] 1.12 Metrics collection (counters, histograms, health gauges) ⬜
- [ ] 1.13 Config hot reload (SIGHUP + atomic `arc-swap` swap) ⬜
- [ ] 1.14 Unit + integration tests for the core ⬜

### Phase 2: HA and VIP

- [ ] 2.1 VIP manager via `rtnetlink` ⬜
- [ ] 2.2 Sync channel (gRPC via `tonic`) ⬜
- [ ] 2.3 Sync message types (RoutingTable, HealthReport, StickySession, UpdateState, Heartbeat) ⬜
- [ ] 2.4 Active/passive state machine ⬜
- [ ] 2.5 VIP takeover on peer failure ⬜
- [ ] 2.6 Graceful shutdown: drain + release VIP ⬜
- [ ] 2.7 Per-tenant port allocation + conflict detection ⬜
- [ ] 2.8 System test rig: `TestCluster` (netns + veth) ⬜
- [ ] 2.9 End-to-end VIP failover tests ⬜

### Phase 3: Rolling Updates

- [ ] 3.1 Define rolling update config model ⬜
- [ ] 3.2 Rollout state machine + step engine ⬜
- [ ] 3.3 Traffic split integration with the routing table ⬜
- [ ] 3.4 Error rate monitoring with `min_samples` guard ⬜
- [ ] 3.5 p99 latency monitoring ⬜
- [ ] 3.6 Auto-rollback logic ⬜
- [ ] 3.7 Rollout state persistence across restart ⬜
- [ ] 3.8 Alert / notification on rollback ⬜
- [ ] 3.9 Manual and instant transition modes ⬜
- [ ] 3.10 End-to-end rollout tests ⬜

### Phase 4: Multi-Tenant and Manager

- [ ] 4.1 Define tenant registry format ⬜
- [ ] 4.2 Registry loading + validation + port conflict detection ⬜
- [ ] 4.3 Systemd unit template generation ⬜
- [ ] 4.4 Process supervisor (systemctl start/stop/restart) ⬜
- [ ] 4.5 Config watcher + per-tenant SIGHUP routing ⬜
- [ ] 4.6 Tenant lifecycle API (create/start/stop/delete) ⬜
- [ ] 4.7 VIP pool allocator ⬜
- [ ] 4.8 Credential store (Argon2id) + token issuing ⬜
- [ ] 4.9 Manager REST API + auth middleware ⬜
- [ ] 4.10 Multi-tenant isolation tests ⬜

### Phase 5: DNS Server

- [ ] 5.1 Integrate `hickory-server` ⬜
- [ ] 5.2 Zone model + automatic record generation ⬜
- [ ] 5.3 Upstream forwarding ⬜
- [ ] 5.4 SRV record support ⬜
- [ ] 5.5 DNS hot reload (TTL changes) ⬜
- [ ] 5.6 DNS zone editor write-back to `tenant.yaml` ⬜
- [ ] 5.7 DNS end-to-end tests ⬜

### Phase 6: TLS and TCP/UDP

- [ ] 6.1 Integrate `rustls` TLS acceptor ⬜
- [ ] 6.2 TLS config from tenant YAML + cert hot reload ⬜
- [ ] 6.3 HTTP/2 over TLS (ALPN) ⬜
- [ ] 6.4 Client certificate verification ⬜
- [ ] 6.5 TLS session resumption (tickets) ⬜
- [ ] 6.6 TCP proxy ⬜
- [ ] 6.7 UDP proxy ⬜
- [ ] 6.8 TLS / TCP / UDP end-to-end tests ⬜

### Phase 7: GUI

- [ ] 7.1 Typed API client + auth interceptor ⬜
- [ ] 7.2 Login screen + session restore + logout ⬜
- [ ] 7.3 App shell, routing, layout, error boundary ⬜
- [ ] 7.4 Tenant switcher ⬜
- [ ] 7.5 Tree view component ⬜
- [ ] 7.6 Sankey diagram component ⬜
- [ ] 7.7 Rolling update UI ⬜
- [ ] 7.8 DNS zone editor ⬜
- [ ] 7.9 Health dashboard ⬜
- [ ] 7.10 Real-time updates via SSE ⬜
- [ ] 7.11 GUI tests (Vitest + Playwright) ⬜

### Phase 8: Security & Hardening

- [ ] 8.1 Threat model + security review ⬜
- [ ] 8.2 systemd hardening profile ⬜
- [ ] 8.3 Input validation hardening + config parser fuzzing ⬜
- [ ] 8.4 Non-root operation + capability drop ⬜
- [ ] 8.5 Dependency audit (`cargo deny`) + SBOM ⬜
- [ ] 8.6 API penetration test ⬜

### Phase 9: Release, Packaging & Operations

- [ ] 9.1 Prometheus exposition endpoint + metric docs ⬜
- [ ] 9.2 Release CI: binaries, checksums, GitHub release, container image ⬜
- [ ] 9.3 RPM and DEB packaging with install scripts ⬜
- [ ] 9.4 Container image ⬜
- [ ] 9.5 Performance testing and tuning ⬜
- [ ] 9.6 Documentation ⬜
- [ ] 9.7 Operator runbook + upgrade procedure ⬜
- [ ] 9.8 Release rehearsal on a clean VM ⬜

---

## By Goal

### Goal 1: Multi-Tenant Process Isolation

- [ ] TC-001 Spin up 5 tenants, crash one, others continue ⬜
- [ ] TC-002 Modify one tenant config, others unaffected ⬜
- [ ] TC-003 CPU limit enforcement ⬜
- [ ] TC-004 Memory limit enforcement ⬜
- [ ] TC-005 Config deletion only stops affected tenant ⬜

### Goal 2: Weighted Traffic Routing

- [ ] TC-010 Region-level weight distribution ⬜
- [ ] TC-011 DC-level weight distribution ⬜
- [ ] TC-012 Cluster-level weight distribution ⬜
- [ ] TC-013 Canary split (80/20) ⬜
- [ ] TC-014 Single target routing ⬜
- [ ] TC-015 Exact path matching ⬜

### Goal 3: HA Worker Pairs with VIP Failover

- [ ] TC-020 Active has VIP, standby does not ⬜
- [ ] TC-021 Standby acquires VIP within 1s on failure ⬜
- [ ] TC-022 No more than 2-3 dropped requests during failover ⬜
- [ ] TC-023 Recovered worker becomes standby ⬜
- [ ] TC-024 Single worker continues without panic ⬜
- [ ] TC-025 VIP released when both workers stopped ⬜
- [ ] TC-026 VIP visible on active worker only ⬜

### Goal 4: Blue-Green Rolling Updates with Auto-Rollback

- [ ] TC-030 Initial state: blue 100%, green 0% ⬜
- [ ] TC-031 Advance step: blue 75%, green 25% ⬜
- [ ] TC-032 Auto-rollback on 6% error rate ⬜
- [ ] TC-033 Rollback returns all traffic to blue ⬜
- [ ] TC-034 Rollback on p99 latency threshold ⬜
- [ ] TC-035 Full rollout to green succeeds ⬜
- [ ] TC-036 Manual step advancement ⬜
- [ ] TC-037 Instant rollback ⬜

### Goal 5: Session Affinity

- [ ] TC-040 Cookie method: same cookie → same backend ⬜
- [ ] TC-041 Cookie method: different cookies → distribution ⬜
- [ ] TC-042 Client-IP method: same IP → same backend ⬜
- [ ] TC-043 Client-IP method: different IPs → distribution ⬜
- [ ] TC-044 Session expiry → new backend ⬜
- [ ] TC-045 No affinity: even distribution ⬜
- [ ] TC-046 Header method: same header → same backend ⬜

### Goal 6: DNS Server

- [ ] TC-050 A record resolution ⬜
- [ ] TC-051 SRV record resolution ⬜
- [ ] TC-052 TTL enforcement ⬜
- [ ] TC-053 Upstream forwarding ⬜
- [ ] TC-054 NXDOMAIN for unknown zones ⬜
- [ ] TC-055 DNS responds on configured port ⬜

### Goal 7: TLS Termination

- [ ] TC-060 TLS handshake succeeds ⬜
- [ ] TC-061 HTTP/2 negotiation ⬜
- [ ] TC-062 Expired cert rejected ⬜
- [ ] TC-063 TLS 1.0/1.1 rejected ⬜
- [ ] TC-064 Client cert verification ⬜

### Goal 8: Web GUI

- [ ] TC-070 Valid login succeeds ⬜
- [ ] TC-071 Invalid login shows error ⬜
- [ ] TC-072 Tenant switcher updates tree view ⬜
- [ ] TC-073 Tree node selection updates Sankey ⬜
- [ ] TC-074 Rolling update UI shows progress ⬜
- [ ] TC-075 Rollback button triggers rollback ⬜
- [ ] TC-076 DNS zone editor saves records ⬜
- [ ] TC-077 Health dashboard shows correct status ⬜

### Goal 9: Configuration Hot Reload

- [ ] TC-080 Config change applied within 100ms ⬜
- [ ] TC-081 Invalid config rejected, error logged ⬜
- [ ] TC-082 No dropped requests during reload ⬜
- [ ] TC-083 DNS TTL change picked up ⬜
- [ ] TC-084 TLS cert change picked up ⬜

### Goal 10: L4 / L7 Proxying and Forwarding Semantics

- [ ] TC-090 Method, path, query, and body are forwarded intact ⬜
- [ ] TC-091 Client-supplied `X-Forwarded-For` is overwritten ⬜
- [ ] TC-092 Unmatched path returns 404 ⬜
- [ ] TC-093 No healthy endpoints returns 503 + reason header ⬜
- [ ] TC-094 Unreachable backend returns 502 ⬜
- [ ] TC-095 Unresponsive backend returns 504 ⬜
- [ ] TC-096 Large bodies are streamed, not buffered ⬜
- [ ] TC-097 Upstream connections are pooled and reused ⬜
- [ ] TC-098 TCP listener forwards bytes transparently ⬜
- [ ] TC-099 UDP listener forwards datagrams ⬜

### Goal 11: API Security and Authentication

- [ ] TC-100 Protected endpoint without a token returns 401 ⬜
- [ ] TC-101 Protected endpoint with a garbage token returns 401 ⬜
- [ ] TC-102 Valid login returns a usable token ⬜
- [ ] TC-103 Wrong password returns the standard error message ⬜
- [ ] TC-104 Unknown username returns a byte-identical error message ⬜
- [ ] TC-105 Credential file stores only an Argon2id hash ⬜
- [ ] TC-106 No secrets in `tenant.yaml` / `registry.yaml` ⬜
- [ ] TC-107 Expired token returns 401 ⬜
- [ ] TC-108 Oversized request body returns 413 ⬜
- [ ] TC-109 DNS `AXFR` is refused ⬜
- [ ] TC-110 Internal errors leak no internal detail ⬜

---

## Milestones

| Milestone | Phases | Meaning |
|-----------|--------|---------|
| **M1 — Routing works** | 0, 1 | A single tenant routes weighted HTTP traffic and hot-reloads |
| **M2 — HA works** | 2 | Two workers, one VIP, sub-second failover |
| **M3 — Operable** | 3, 4, 5 | Rollouts, multi-tenancy, DNS — a real deployment is possible |
| **M4 — Production-ready traffic** | 6 | TLS, HTTP/2, L4 |
| **M5 — Human-usable** | 7, 8 | GUI and security posture |
| **M6 — Shippable** | 9 | Packaged, documented, rehearsed |

---

*Last updated: 2026-11-12*
