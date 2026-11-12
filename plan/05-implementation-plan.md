# Orchestrator — Implementation Plan

> **Revision note (2026-11-12).** This plan was revised after a full gap review of the original
> 8-phase plan. The review found whole features referenced by the goals but absent from the task
> list (session affinity, the health checker, authentication, the test harness), a task-ordering
> defect (Phase 3's rollback engine depended on metrics that only arrived in Phase 8), and several
> correctness problems (a hardcoded metrics port that collides across tenants, a `--vip-priority`
> flag implying a keepalived-style model the design does not use, and the renamed
> `trust-dns-server` crate). The phases below fix all of these.
>
> Task IDs from the original plan are **preserved** wherever the task still exists. New tasks use
> new IDs within their phase. Phase 8 was renamed and Phase 9 was added.

---

## Phase 0: Foundation

**Goal**: A repository that builds, lints, tests, and can be extended without friction — plus the
test fixtures that every later phase depends on.

Nothing else can be verified without this phase. The `orchestrator-testkit` crate in particular is
a hard prerequisite for proving Goals 2, 4, 5, and 6.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 0.1 | Repo scaffolding: Cargo workspace, crate skeletons, `rust-toolchain.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `.gitignore`, `.editorconfig` | 6h | ⬜ |
| 0.2 | CI pipeline: `lint`, `unit`, `integration`, `build` jobs + `scripts/check-manifest.sh` (Cargo.toml ↔ `09-dependencies.md` cross-check) | 6h | ⬜ |
| 0.3 | `orchestrator-testkit`: `MockBackend` — a real HTTP server that records every request and can simulate latency, failures, and death | 8h | ⬜ |
| 0.4 | `orchestrator-testkit`: `LoadGenerator` + `assert_binomial` / `assert_within` distribution helpers | 6h | ⬜ |
| 0.5 | `orchestrator-testkit`: `TestConfig` builder + `FakeClock` | 6h | ⬜ |
| 0.6 | Repository governance: `README.md`, `LICENSE` (Apache-2.0), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `CHANGELOG.md`, PR + issue templates | 4h | ⬜ |
| 0.7 | GUI scaffold: Vite + React + TypeScript, ESLint, Prettier, Vitest, Playwright config | 6h | ⬜ |
| 0.8 | `docs/` skeleton, `examples/tenant.yaml`, `examples/registry.yaml`, `Makefile`, `scripts/bootstrap.sh` | 6h | ⬜ |

**Total**: ~48h

### Phase 0 Acceptance

`make check` passes on a clean clone. `MockBackend` + `LoadGenerator` can demonstrate a 60/40 split
across two backends in a test.

---

## Phase 1: Core Engine

**Goal**: A single tenant process that routes HTTP traffic with weighted splits, tracks backend
health, honours session affinity, and hot-reloads its configuration.

> The original Phase 1 omitted the health checker, the session store, logging, and metrics. All
> four are added here because later phases depend on them — in particular, Phase 3's auto-rollback
> needs the metrics that were previously scheduled for Phase 8.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 1.1 | Define `orchestrator-core` types (Tenant, Region, Datacenter, Cluster, Endpoint) | 4h | ⬜ |
| 1.2 | Config schema model + duration/size string parsing (`30s`, `5m`, `512M`) | 6h | ⬜ |
| 1.3 | YAML config parser (`serde-saphyr`, `deny_unknown_fields`) | 4h | ⬜ |
| 1.4 | Validator implementing rules V1–V17, **collecting all errors** rather than failing on the first | 8h | ⬜ |
| 1.5 | Hierarchy model + multiplicative weight composition (`07-config-schema.md` §5) | 6h | ⬜ |
| 1.6 | Weighted selection algorithm (cumulative-distribution weighted random) | 4h | ⬜ |
| 1.7 | Route matching: prefix / exact, longest-prefix-wins, path-boundary rule, query-string stripping | 6h | ⬜ |
| 1.8 | HTTP proxy (hyper 1.x) with connection pooling, timeouts, retries, `X-Forwarded-*` headers | 8h | ⬜ |
| 1.9 | Backend health checker — HTTP and TCP probes, `healthy_threshold` / `unhealthy_threshold`, endpoint health state | 8h | ⬜ |
| 1.10 | Session affinity store — `cookie` / `header` / `client-ip` / `none`, consistent hashing, TTL expiry, LRU cap | 8h | ⬜ |
| 1.11 | Structured logging with `tracing` + `tracing-subscriber` (JSON / compact) | 3h | ⬜ |
| 1.12 | Metrics collection — request counters, latency histograms, endpoint health gauges, per-route/per-endpoint labels | 6h | ⬜ |
| 1.13 | Config hot reload — SIGHUP → validate → atomic swap via `arc-swap`, with the old config surviving a validation failure | 6h | ⬜ |
| 1.14 | Unit + integration tests for the core | 8h | ⬜ |

**Total**: ~85h

### Phase 1 Acceptance

`orchestrator --config tenant.yaml` routes HTTP traffic according to the config. Changing a weight
and sending `SIGHUP` applies the change within 100ms with no dropped requests. A config with
weights that do not sum to 100 is rejected and the running config is left untouched.

---

## Phase 2: HA and VIP

**Goal**: Worker pair with VIP failover via the Linux kernel.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 2.1 | VIP manager — address add/del via `rtnetlink` (not raw `netlink-packet-route`) | 6h | ⬜ |
| 2.2 | Sync channel — gRPC server + client via `tonic`, TLS-capable transport | 8h | ⬜ |
| 2.3 | Sync message types: `RoutingTable`, `HealthReport`, `StickySession`, `UpdateState`, `Heartbeat` | 4h | ⬜ |
| 2.4 | Active/passive state machine (active → standby on peer loss → standby on recovery) | 6h | ⬜ |
| 2.5 | VIP takeover on peer failure, within `peer_timeout` (default 1s) | 4h | ⬜ |
| 2.6 | Graceful shutdown — drain in-flight requests, then release the VIP before exit | 4h | ⬜ |
| 2.7 | Per-tenant port allocation + conflict detection (replaces the colliding hardcoded `--metrics-port 9090`) | 4h | ⬜ |
| 2.8 | System test rig — `TestCluster` using network namespaces + veth pairs | 10h | ⬜ |
| 2.9 | End-to-end VIP failover tests (TC-020 … TC-026) | 8h | ⬜ |

**Total**: ~54h

### Phase 2 Acceptance

Two worker processes share a VIP. When the active is killed, the standby acquires the VIP within
1 second. When the active restarts, it becomes standby. Killing the standby does not disturb the
active.

---

## Phase 3: Rolling Updates

**Goal**: Blue-green deployments with auto-rollback.

> Depends on Phase 1.12 (metrics) and Phase 1.9 (health). This is why those were moved forward.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 3.1 | Define rolling update config model | 2h | ⬜ |
| 3.2 | Rollout state machine + step engine (`idle → in_progress → paused → rolling_back → rolled_back / completed / failed`) | 8h | ⬜ |
| 3.3 | Traffic split integration — the rollout overrides the route's effective weights for the blue/green targets | 4h | ⬜ |
| 3.4 | Error rate monitoring over a sliding window, with `min_samples` guard | 4h | ⬜ |
| 3.5 | p99 latency monitoring over the same window | 3h | ⬜ |
| 3.6 | Auto-rollback logic — trigger on either threshold, shift traffic to blue immediately | 4h | ⬜ |
| 3.7 | Rollout state persistence — survive a process restart mid-rollout | 4h | ⬜ |
| 3.8 | Alert / notification on rollback (webhook + log + GUI event) | 3h | ⬜ |
| 3.9 | Manual (`advance`) and instant transition modes | 3h | ⬜ |
| 3.10 | End-to-end rollout tests (TC-030 … TC-037) | 10h | ⬜ |

**Total**: ~45h

### Phase 3 Acceptance

A blue-green rollout starts, traffic shifts gradually, and a 6% error rate on green triggers an
automatic rollback within the configured window.

---

## Phase 4: Multi-Tenant and Manager

**Goal**: A manager binary that manages multiple tenant processes, with an authenticated API.

> The original Phase 4 had no authentication at all, even though Goal 8 requires a login screen.
> Tasks 4.8 and 4.9 close that hole.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 4.1 | Define tenant registry format | 2h | ⬜ |
| 4.2 | Manager registry loading + validation + **port conflict detection** | 4h | ⬜ |
| 4.3 | Systemd unit template generation from the registry | 6h | ⬜ |
| 4.4 | Process supervisor — start / stop / restart via `systemctl` | 6h | ⬜ |
| 4.5 | Config watcher (`notify`/inotify) + **per-tenant** SIGHUP routing (G1.5) | 4h | ⬜ |
| 4.6 | Tenant lifecycle API — create / start / stop / delete | 8h | ⬜ |
| 4.7 | VIP pool allocator | 4h | ⬜ |
| 4.8 | Credential store — Argon2id hashing, `0600` file, token issuing | 6h | ⬜ |
| 4.9 | Manager REST API + auth middleware (bearer tokens, problem-details errors) | 8h | ⬜ |
| 4.10 | Multi-tenant isolation tests (TC-001 … TC-005) | 8h | ⬜ |

**Total**: ~58h

### Phase 4 Acceptance

The manager creates three tenant processes from the registry. A config change to one tenant does
not affect the others. The API rejects unauthenticated requests.

---

## Phase 5: DNS Server

**Goal**: Authoritative DNS for tenant zones.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 5.1 | Integrate `hickory-server` (the renamed `trust-dns-server`) | 4h | ⬜ |
| 5.2 | Zone model + automatic record generation from the tenant config | 4h | ⬜ |
| 5.3 | Upstream forwarding for non-authoritative queries | 4h | ⬜ |
| 5.4 | SRV record support | 4h | ⬜ |
| 5.5 | DNS hot reload — TTL changes take effect without a restart | 3h | ⬜ |
| 5.6 | DNS zone editor write-back — API writes go to `tenant.yaml` atomically (TC-076) | 4h | ⬜ |
| 5.7 | DNS end-to-end tests (TC-050 … TC-055) | 6h | ⬜ |

**Total**: ~29h

### Phase 5 Acceptance

`dig api.tenant.internal @localhost` returns the configured VIP. `dig google.com @localhost`
returns the upstream result. A TTL change is picked up without a restart.

---

## Phase 6: TLS and TCP/UDP

**Goal**: TLS termination and L4 proxy support.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 6.1 | Integrate `rustls` TLS acceptor | 8h | ⬜ |
| 6.2 | TLS config from tenant YAML + certificate hot reload | 4h | ⬜ |
| 6.3 | HTTP/2 over TLS (ALPN negotiation) | 4h | ⬜ |
| 6.4 | Client certificate verification (`none` / `optional` / `required`) | 4h | ⬜ |
| 6.5 | TLS session resumption via session tickets | 3h | ⬜ |
| 6.6 | TCP proxy (L4 passthrough) | 6h | ⬜ |
| 6.7 | UDP proxy (L4 passthrough) | 5h | ⬜ |
| 6.8 | TLS / TCP / UDP end-to-end tests (TC-060 … TC-064) | 8h | ⬜ |

**Total**: ~42h

### Phase 6 Acceptance

HTTPS requests terminate at the VIP. TCP and UDP traffic is forwarded. TLS 1.0 and 1.1 are
rejected.

---

## Phase 7: GUI

**Goal**: Web management interface.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 7.1 | Typed API client + auth interceptor + error handling | 6h | ⬜ |
| 7.2 | Login screen + session restore + logout | 6h | ⬜ |
| 7.3 | App shell, routing, layout, error boundary | 4h | ⬜ |
| 7.4 | Tenant switcher | 3h | ⬜ |
| 7.5 | Tree view component (d3-hierarchy) | 8h | ⬜ |
| 7.6 | Sankey diagram component (d3-sankey) | 8h | ⬜ |
| 7.7 | Rolling update UI — step, progress bar, advance / pause / rollback buttons | 6h | ⬜ |
| 7.8 | DNS zone editor — add / edit / delete records | 6h | ⬜ |
| 7.9 | Health dashboard — tenant and worker status | 4h | ⬜ |
| 7.10 | Real-time updates via SSE with reconnect | 5h | ⬜ |
| 7.11 | GUI tests — Vitest unit tests + Playwright e2e (TC-070 … TC-077) | 8h | ⬜ |

**Total**: ~64h

### Phase 7 Acceptance

The GUI shows all tenants, the tree view displays the hierarchy, the Sankey diagram shows traffic
splits, and a rolling update is controllable from the UI.

---

## Phase 8: Security & Hardening

**Goal**: Production-grade security posture.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 8.1 | Threat model + security review of the TLS, auth, and config-parsing paths | 6h | ⬜ |
| 8.2 | systemd hardening profile — `NoNewPrivileges`, `ProtectSystem=strict`, `RestrictAddressFamilies`, capability bounding set | 3h | ⬜ |
| 8.3 | Input validation hardening + `proptest`/fuzzing of the config parser | 6h | ⬜ |
| 8.4 | Non-root operation + capability drop where the platform allows it | 5h | ⬜ |
| 8.5 | Dependency audit — `cargo deny` advisory + licence policy, SBOM generation | 4h | ⬜ |
| 8.6 | API penetration test — auth bypass, privilege escalation, resource exhaustion | 6h | ⬜ |

**Total**: ~30h

### Phase 8 Acceptance

`cargo deny` reports zero advisories and zero licence violations. The API survives an auth-bypass
attempt. The config parser survives fuzzing.

---

## Phase 9: Release, Packaging & Operations

**Goal**: Shippable, installable, operable.

| # | Task | Estimated | Status |
|---|------|-----------|--------|
| 9.1 | Prometheus exposition endpoint + metric documentation | 4h | ⬜ |
| 9.2 | Release CI — binaries for `x86_64` and `aarch64`, checksums, GitHub release, container image | 8h | ⬜ |
| 9.3 | RPM and DEB packaging with pre/post install scripts | 8h | ⬜ |
| 9.4 | Container image (multi-stage, non-root, `NET_ADMIN` only where needed) | 4h | ⬜ |
| 9.5 | Performance testing and tuning against the budgets in `11-non-functional-requirements.md` | 10h | ⬜ |
| 9.6 | Documentation — getting started, configuration, operations, troubleshooting, architecture | 12h | ⬜ |
| 9.7 | Operator runbook — health checks, rollouts, DNS changes, scaling, backups, **upgrade procedure** | 4h | ⬜ |
| 9.8 | Release rehearsal on a clean VM — install, configure, run the acceptance suite | 4h | ⬜ |

**Total**: ~54h

### Phase 9 Acceptance

A clean VM can be brought up from the package alone, with the acceptance suite passing against it.

---

## Summary

| Phase | Hours | Notes |
|-------|-------|-------|
| 0. Foundation | ~48h | **New** — repo, CI, test harness |
| 1. Core Engine | ~85h | Health checker, affinity, logging, metrics added |
| 2. HA and VIP | ~54h | Graceful shutdown, port allocation, netns rig added |
| 3. Rolling Updates | ~45h | State persistence, notification added |
| 4. Multi-Tenant & Manager | ~58h | **Auth added** — was entirely missing |
| 5. DNS Server | ~29h | Zone write-back added |
| 6. TLS and TCP/UDP | ~42h | Client certs, resumption added |
| 7. GUI | ~64h | SSE, app shell, tests added |
| 8. Security & Hardening | ~30h | Renamed, expanded |
| 9. Release & Operations | ~54h | **New** — packaging, release CI, runbook |
| **Total** | **~509h** | Was ~270h |

### Estimate Notes

- The increase from ~270h to ~509h is almost entirely **previously unestimated work**: the test
  harness, the health checker, session affinity, authentication, packaging, and the security work.
- At 4 productive hours/day, ~509h ≈ **32 working days** for one engineer, or ~8 weeks.
- Phases 5, 6, and 7 can partially overlap once Phase 1 and 4 are done.
- Estimates carry ~30% uncertainty. Re-estimate at the end of each phase.

---

## Requirement Traceability

Which task satisfies which requirement. A requirement with no task is a gap; this table is
checked during review.

| Requirement | Task |
|-------------|-------|
| G1.1 systemd service per tenant | 4.3, 4.4 |
| G1.2 per-tenant config path | 1.3, 4.6 |
| G1.3 resource limits | 4.3 |
| G1.4 no shared state | 4.4, 4.10 |
| G1.5 reload scoped to one tenant | 4.5, 4.10 |
| G2.1 4-level hierarchy | 1.1, 1.5 |
| G2.2 weights sum to 100 | 1.4 (V3, V4) |
| G2.3 weighted random selection | 1.6 |
| G2.4 prefix path matching | 1.7 |
| G2.5 exact path matching | 1.7 |
| G3.1 VIP via netlink | 2.1 |
| G3.2 active/passive | 2.4 |
| G3.3 gRPC sync | 2.2, 2.3 |
| G3.4 failover < 1s | 2.5, 2.9 |
| G3.5 VIP release/acquire | 2.1, 2.5, 2.6 |
| G3.6 recovered worker → standby | 2.4 |
| G4.1–G4.4 blue/green modes | 3.1, 3.2, 3.9 |
| G4.5 error-rate rollback | 3.4, 3.6 |
| G4.6 p99 latency rollback | 3.5, 3.6 |
| G4.7 rollback window | 3.4 |
| G4.8 immediate shift on rollback | 3.6 |
| G4.9 alert on rollback | 3.8 |
| G5.1–G5.7 session affinity | 1.10 |
| G6.1–G6.6 DNS | 5.1–5.3, 5.4, 5.5 |
| G7.1–G7.6 TLS | 6.1–6.5 |
| G8.1 login | 4.8, 7.2 |
| G8.2 tenant switcher | 7.4 |
| G8.3 tree view | 7.5 |
| G8.4 Sankey | 7.6 |
| G8.5 rolling update UI | 7.7 |
| G8.6 DNS zone editor | 5.6, 7.8 |
| G8.7 health dashboard | 7.9 |
| G9.1–G9.3 reload + validation | 1.4, 1.13 |
| G9.4 atomic swap | 1.13 |
| G9.5 no dropped connections | 1.13, 2.6 |
| G9.6 reload < 100ms | 1.12, 1.13 |
