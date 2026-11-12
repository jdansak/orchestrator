# Orchestrator — Test Strategy

How the acceptance criteria in `02-goals-and-acceptance-criteria.md` are actually proven.
Every `TC-xxx` must map to an automated test. A test case that only exists in a document is not
acceptance criteria — it is a wish.

---

## 1. Test Layers

| Layer | Where | Runs in CI | Needs privileges | Purpose |
|-------|-------|-----------|------------------|---------|
| **Unit** | `crates/*/src/**` `#[cfg(test)]` | yes | no | Pure logic: weights, matching, validation, state machines |
| **Integration** | `crates/*/tests/**` | yes | no | Cross-module behaviour with real HTTP servers on ephemeral ports |
| **Component** | `crates/orchestrator/tests/component/**` | yes | no | Whole data plane: proxy, TLS, DNS, sessions, rollout |
| **System** | `system-tests/` | separate (Linux runners) | **yes** (root, netns, systemd) | VIP failover, tenant isolation, resource limits |
| **E2E GUI** | `orchestrator-gui/e2e/` | separate | no | Playwright flows against a live stack |
| **Load** | `system-tests/load/` | nightly | no | Performance budgets |

---

## 2. Test Infrastructure

A shared test harness crate, `orchestrator-testkit`, provides the fixtures every test needs.

### 2.1 `MockBackend`

A real HTTP server (axum) that:

- Responds with a configured status code, body, and latency
- Records every request it receives with a timestamp, path, headers, and the peer address
- Can be told to fail a percentage of requests (`fail_rate`), to be slow (`latency`), or to
  start/stop on command (to simulate a backend dying)
- Echoes back a unique request ID so the caller can correlate

```rust
let backend = MockBackend::start().await;
backend.set_response(ResponseSpec { status: 200, latency: Duration::ZERO, body: "ok".into() });
backend.set_fail_rate(0.06);              // for the auto-rollback test
let hits = backend.hits();               // what actually received traffic
```

This is the single most important piece of infrastructure. **Without it there is no way to prove
G2, G4, or G5**, because all of those require observing which backend received which request.

### 2.2 `LoadGenerator`

A concurrent HTTP client that fires a fixed number of requests and reports the distribution:

```rust
let result = LoadGenerator::new(url)
    .requests(10_000)
    .concurrency(50)
    .run()
    .await;

assert_within(result.distribution("us-east"), 0.60, 0.05);  // TC-010
```

`assert_within` is a shared helper that takes the expected proportion and a tolerance and fails
with a readable message showing the observed count and the binomial confidence interval.

### 2.3 `Distribution` helper

For weighted-routing tests the expected share is a **binomial** distribution. The helper computes
the exact binomial CDF so the assertion is statistically sound rather than an arbitrary ±5%:

```rust
assert_binomial(observed, trials, expected_p, 0.001);
```

This avoids both false failures on a correct implementation and false passes on a broken one.

### 2.4 `TestCluster`

For system tests: spins up N network namespaces with veth pairs, assigns IPs, and lets a test
move a VIP between namespaces. This is how the VIP failover test proves G3.4 without needing two
physical hosts.

```rust
let cluster = TestCluster::new(2).await;   // two namespaces, a veth bridge
cluster.worker_a().start().await;
cluster.worker_b().start().await;
assert!(cluster.worker_a().has_vip().await);
cluster.worker_a().kill().await;
assert!(cluster.worker_b().acquired_vip().within(Duration::from_secs(1)).await);
```

### 2.5 `FakeClock`

Rollout timing, health-check intervals, and session expiry are all time-dependent. The harness
exposes a controllable clock so tests do not sleep for real minutes:

```rust
let clock = FakeClock::new();
clock.advance(Duration::from_secs(61)).await;   // triggers the 1-minute rollback window
```

### 2.6 `TestConfig` builder

Typed builders for `tenant.yaml` so a test never hand-writes YAML:

```rust
let config = TestConfig::builder("app-frontend")
    .hierarchy(hierarchy![("us-east", 60, ["ep-a", "ep-b"]), ("eu-west", 40, ["ep-c"])])
    .route(route!("api", "/api/v2", prefix))
    .build();
```

---

## 3. Acceptance Test Map

Every acceptance criterion in `02-goals-and-acceptance-criteria.md`, with the test that proves it.

### Goal 1 — Multi-Tenant Isolation

| TC | Test | Type |
|----|------|------|
| TC-001 | `system/tenant_isolation.rs::crash_one_tenant_others_survive` | system |
| TC-002 | `system/tenant_isolation.rs::reload_is_scoped_to_one_tenant` | system |
| TC-003 | `system/tenant_isolation.rs::cpu_quota_is_enforced` | system (reads `/sys/fs/cgroup/…/cpu.max`) |
| TC-004 | `system/tenant_isolation.rs::memory_max_triggers_oom_kill` | system |
| TC-005 | `system/tenant_isolation.rs::deleting_config_stops_only_that_tenant` | system |

### Goal 2 — Weighted Routing

| TC | Test | Type |
|----|------|------|
| TC-010 | `component/weighted_routing.rs::region_weights_are_respected` | component |
| TC-011 | `component/weighted_routing.rs::datacenter_weights_are_respected` | component |
| TC-012 | `component/weighted_routing.rs::cluster_weights_are_respected` | component |
| TC-013 | `component/weighted_routing.rs::canary_split_is_respected` | component |
| TC-014 | `component/weighted_routing.rs::single_target_receives_all_traffic` | component |
| TC-015 | `component/route_matching.rs::exact_path_matching` | component |
| — | `unit/route_matching.rs` — prefix boundaries, longest-prefix-wins, query-string handling | unit |
| — | `unit/weights.rs` — multiplicative composition, weight-sum validation, `proptest` fuzz | unit |

### Goal 3 — HA / VIP Failover

| TC | Test | Type |
|----|------|------|
| TC-020 | `system/vip_failover.rs::active_owns_vip_standby_does_not` | system |
| TC-021 | `system/vip_failover.rs::standby_takes_over_within_one_second` | system |
| TC-022 | `system/vip_failover.rs::few_requests_dropped_during_failover` | system |
| TC-023 | `system/vip_failover.rs::recovered_worker_becomes_standby` | system |
| TC-024 | `system/vip_failover.rs::active_continues_when_standby_is_killed` | system |
| TC-025 | `system/vip_failover.rs::vip_released_when_both_stopped` | system |
| TC-026 | `system/vip_failover.rs::vip_visible_on_active_interface_only` | system |

### Goal 4 — Blue/Green Rollout

| TC | Test | Type |
|----|------|------|
| TC-030 | `component/rollout.rs::initial_state_is_all_blue` | component |
| TC-031 | `component/rollout.rs::advancing_step_shifts_traffic` | component |
| TC-032 | `component/rollout.rs::error_rate_triggers_auto_rollback` | component |
| TC-033 | `component/rollout.rs::rollback_returns_traffic_to_blue` | component |
| TC-034 | `component/rollout.rs::p99_latency_triggers_auto_rollback` | component |
| TC-035 | `component/rollout.rs::healthy_rollout_completes` | component |
| TC-036 | `component/rollout.rs::manual_advance_shifts_one_step` | component |
| TC-037 | `component/rollout.rs::instant_rollback_switches_immediately` | component |
| — | `unit/rollout.rs` — step validation, state machine transitions | unit |

### Goal 5 — Session Affinity

| TC | Test | Type |
|----|------|------|
| TC-040 | `component/affinity.rs::cookie_affinity_pins_to_one_backend` | component |
| TC-041 | `component/affinity.rs::distinct_cookies_spread_across_backends` | component |
| TC-042 | `component/affinity.rs::client_ip_affinity_pins_to_one_backend` | component |
| TC-043 | `component/affinity.rs::distinct_ips_spread_across_backends` | component |
| TC-044 | `component/affinity.rs::expired_session_gets_new_backend` | component (FakeClock) |
| TC-045 | `component/affinity.rs::none_affinity_distributes_evenly` | component |
| TC-046 | `component/affinity.rs::header_affinity_pins_to_one_backend` | component |

### Goal 6 — DNS

| TC | Test | Type |
|----|------|------|
| TC-050 | `component/dns.rs::a_record_returns_vip` | component |
| TC-051 | `component/dns.rs::srv_record_returns_priority_weight_port_target` | component |
| TC-052 | `component/dns.rs::ttl_is_honoured` | component (FakeClock) |
| TC-053 | `component/dns.rs::non_authoritative_query_is_forwarded` | component (stub upstream) |
| TC-054 | `component/dns.rs::unknown_zone_returns_nxdomain` | component |
| TC-055 | `component/dns.rs::responds_on_configured_port` | component |

### Goal 7 — TLS

| TC | Test | Type |
|----|------|------|
| TC-060 | `component/tls.rs::handshake_succeeds` | component |
| TC-061 | `component/tls.rs::http2_alpn_negotiates` | component |
| TC-062 | `component/tls.rs::expired_certificate_is_rejected` | component (`rcgen` + `rcgen` expiry) |
| TC-063 | `component/tls.rs::tls_1_0_and_1_1_are_rejected` | component |
| TC-064 | `component/tls.rs::client_certificate_accepted_or_rejected_per_config` | component |

### Goal 8 — GUI

| TC | Test | Type |
|----|------|------|
| TC-070 | `e2e/login.spec.ts::valid_login_redirects_to_dashboard` | e2e |
| TC-071 | `e2e/login.spec.ts::invalid_login_shows_error` | e2e |
| TC-072 | `e2e/tenant-switcher.spec.ts::switching_tenant_updates_tree` | e2e |
| TC-073 | `e2e/sankey.spec.ts::selecting_node_updates_sankey` | e2e |
| TC-074 | `e2e/rollout.spec.ts::progress_bar_shows_current_step` | e2e |
| TC-075 | `e2e/rollout.spec.ts::rollback_button_triggers_rollback` | e2e |
| TC-076 | `e2e/dns-editor.spec.ts::adding_record_saves_and_applies` | e2e |
| TC-077 | `e2e/health-dashboard.spec.ts::status_is_correct` | e2e |

### Goal 9 — Hot Reload

| TC | Test | Type |
|----|------|------|
| TC-080 | `component/reload.rs::weight_change_applied_within_100ms` | component |
| TC-081 | `component/reload.rs::invalid_weights_are_rejected_and_logged` | component |
| TC-082 | `component/reload.rs::no_requests_dropped_during_reload` | component |
| TC-083 | `component/reload.rs::dns_ttl_change_is_picked_up` | component |
| TC-084 | `component/reload.rs::tls_cert_change_is_picked_up` | component |

---

## 4. CI Strategy

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]

jobs:
  lint:        # fmt + clippy + cargo-deny + tsc + eslint     — ubuntu-latest
  unit:        # cargo test --workspace            — ubuntu-latest, matrix of 3 rust versions
  integration: # cargo test --workspace --test '*' — ubuntu-latest
  component:   # cargo test -p orchestrator --test component
  build:       # cargo build --release + npm build + npm test

  system:      # the privileged tests
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - run: sudo apt-get install -y iproute2
      - run: sudo ./system-tests/run.sh          # netns + systemd-in-container

  e2e:         # Playwright
```

**System tests need a Linux host with `CAP_NET_ADMIN`.** They run on `ubuntu-latest` GitHub
runners using `unshare --net` network namespaces and `systemd-run --scope` for the resource-limit
checks. They are a **separate job** so a failure there does not hide unit-test failures.

### Locating a Test by Its Criterion

A single test runner resolves a criterion ID to its tests:

```bash
cargo test --test acceptance -- --tags tc-010
```

`nextest` is used for the test runner so tags can be filtered per criterion. The CI publishes a
coverage matrix mapping every `TC-xxx` in `02-goals-and-acceptance-criteria.md` to its test, and
**fails if any criterion has no test** — this is the mechanism that stops criteria from rotting.

---

## 5. Coverage Requirements

| Area | Minimum line coverage | Rationale |
|------|----------------------|-----------|
| `orchestrator-core` (config, weights, matching, validation) | **95%** | Pure logic, cheap to cover, highest bug density |
| `orchestrator` control plane | 80% | State machines and the rollout engine |
| `orchestrator` data plane | 70% | Proxy paths |
| `orchestrator-manager` | 70% | |
| `orchestrator-gui` | 70% | |

CI fails if coverage drops below the threshold for a crate that has no `[coverage:ignore]`
annotation.

---

## 6. Flakiness Policy

- **No test may sleep for real time to satisfy a duration requirement.** Use `FakeClock` or
  `tokio::time::pause()`. The only real sleeps permitted are in system tests that measure genuine
  wall-clock failover (< 1s, G3.4).
- A test that flakes is **disabled and tracked**, never silently retried. Retrying hides bugs.
- The nightly job runs the full suite 10× to surface order-dependence and flakes.
