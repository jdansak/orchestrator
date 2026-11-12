# Orchestrator — Goals & Acceptance Criteria

## Goal 1: Multi-Tenant Process Isolation

Each tenant runs as a completely isolated process with its own configuration, VIP, and resources. A crash or misconfiguration in one tenant cannot affect any other tenant.

### Requirements

- **G1.1**: Each tenant is managed as an independent systemd service
- **G1.2**: Each tenant reads its own config file from `/etc/orchestrator/tenants/<tenant-name>/tenant.yaml`
- **G1.3**: Tenants have resource limits (CPU, memory) enforced at the OS level
- **G1.4**: No shared memory, no shared state between tenant processes
- **G1.5**: Config changes to one tenant do not trigger reloads in other tenants

### Test Cases

- [ ] **TC-001**: Spin up 5 tenants, crash tenant #3 — confirm tenants #1, #2, #4, #5 continue operating
- [ ] **TC-002**: Modify tenant.yaml for tenant A — confirm only tenant A reloads
- [ ] **TC-003**: Set CPU limit to 500m on tenant — confirm kernel enforces limit (top shows capped)
- [ ] **TC-004**: Set memory limit to 256Mi on tenant — confirm kernel OOM-kills tenant if exceeded
- [ ] **TC-005**: Delete config file for tenant A — confirm tenant A stops, other tenants unaffected

---

## Goal 2: Weighted Traffic Routing Across Hierarchy

Traffic is routed through a hierarchy: region → datacenter → cluster → endpoint, with configurable weights at each level. Weights compose multiplicatively.

### Requirements

- **G2.1**: Config supports a 4-level hierarchy (region → datacenter → cluster → endpoint)
- **G2.2**: Each sibling in the hierarchy has a weight (0-100) that must sum to 100
- **G2.3**: Traffic is distributed according to configured weights using weighted random selection
- **G2.4**: Route matching supports path-based prefix matching (e.g., `/api/v2/*`)
- **G2.5**: Route matching supports exact path matching (e.g., `/health`)

### Test Cases

- [ ] **TC-010**: Configure region A at 60%, region B at 40% — send 10,000 requests, verify ~6,000 to A and ~4,000 to B (±5%)
- [ ] **TC-011**: Configure datacenter A at 70%, datacenter B at 30% within a region — verify proportional distribution
- [ ] **TC-012**: Configure cluster A at 80%, cluster B at 20% within a DC — verify proportional distribution
- [ ] **TC-013**: Send 100 requests to `/api/v2/*` with blue(80%)/green(20%) canary — verify ~80 go to blue, ~20 to green
- [ ] **TC-014**: Send 100 requests to `/api/v1/*` with single target — verify 100% go to target
- [ ] **TC-015**: Send 100 requests to `/health` — verify exact path matching works

---

## Goal 3: HA Worker Pairs with VIP Failover

Each tenant has a pair of workers (active/passive) that share a virtual IP. The active worker owns the VIP. If the active fails, the standby takes over within 1 second.

### Requirements

- **G3.1**: Each worker pair shares a single VIP configured via Linux kernel `iproute2` (netlink)
- **G3.2**: Active/passive mode: only the active worker processes traffic
- **G3.3**: Workers communicate via gRPC sync channel for state synchronization
- **G3.4**: Failover time: standby becomes active within 1 second of active failure
- **G3.5**: VIP is released from failed worker and acquired by standby
- **G3.6**: When original active recovers, it becomes standby (preemptive mode)

### Test Cases

- [ ] **TC-020**: Start worker A (active) and worker B (standby) — verify A has VIP, B does not
- [ ] **TC-021**: Kill worker A process — verify B acquires VIP within 1 second
- [ ] **TC-022**: Send traffic during failover — verify no more than 2-3 requests are dropped
- [ ] **TC-023**: Restart worker A — verify it becomes standby, B remains active
- [ ] **TC-024**: Kill worker B — verify worker A continues serving as active (no panic)
- [ ] **TC-025**: Verify VIP is correctly released when both workers are stopped
- [ ] **TC-026**: Verify `ip addr show` shows VIP only on active worker's interface

---

## Goal 4: Blue-Green Rolling Updates with Auto-Rollback

Support blue-green deployments with gradual traffic shifting. If error rate exceeds threshold, automatically rollback to the previous version.

### Requirements

- **G4.1**: Support blue-green strategy with blue and green targets
- **G4.2**: Support gradual transition with configurable steps and durations
- **G4.3**: Support instant transition (for emergency switching)
- **G4.4**: Support manual transition (operator clicks "next step")
- **G4.5**: Auto-rollback on error rate threshold (configurable, default 5%)
- **G4.6**: Auto-rollback on latency p99 threshold (configurable, default 2s)
- **G4.7**: Rollback window: metrics evaluated over configurable time window (default 1 minute)
- **G4.8**: During rollback, traffic shifts immediately to the healthy target
- **G4.9**: Alert/notification when rollback triggers

### Test Cases

- [ ] **TC-030**: Start a blue-green rollout: blue 100% → green 0% — verify all traffic goes to blue
- [ ] **TC-031**: Advance to blue 75% / green 25% — verify traffic split is correct (±3%)
- [ ] **TC-032**: Simulate 6% error rate on green — verify auto-rollback triggers within 1 minute
- [ ] **TC-033**: After rollback, verify all traffic is back on blue
- [ ] **TC-034**: Simulate p99 latency of 3s on green (with 2s threshold) — verify rollback triggers
- [ ] **TC-035**: Complete a full rollout (blue 0% → green 100%) with healthy green — verify no rollbacks
- [ ] **TC-036**: Trigger manual step in rollout — verify traffic shifts by one step
- [ ] **TC-037**: Trigger instant rollback during active rollout — verify immediate switch to blue

---

## Goal 5: Session Affinity (Configurable)

Support configurable session affinity (sticky sessions) at the route level. When enabled, requests from the same session always go to the same backend.

### Requirements

- **G5.1**: Support `cookie` method: set a session cookie, read it on subsequent requests
- **G5.2**: Support `header` method: read a specified header value for session identification
- **G5.3**: Support `client-ip` method: hash the client IP to select backend
- **G5.4**: Support `none` method: no affinity, pure weighted random
- **G5.5**: Session duration is configurable (default 30 minutes)
- **G5.6**: Session affinity is configurable per-route (not globally)
- **G5.7**: When affinity is enabled and session expires, request gets a new backend assignment

### Test Cases

- [ ] **TC-040**: Send 10 requests with same cookie value — verify all go to same backend
- [ ] **TC-041**: Send 10 requests with different cookie values — verify distribution across backends
- [ ] **TC-042**: Send 10 requests with same client IP (client-ip method) — verify all go to same backend
- [ ] **TC-043**: Send 10 requests with different client IPs — verify distribution across backends
- [ ] **TC-044**: Set session duration to 1 second, wait 2 seconds, send request — verify new backend assignment
- [ ] **TC-045**: Configure route with `none` affinity — verify requests distribute evenly
- [ ] **TC-046**: Configure header method with `X-Session-ID` — verify requests with same header go to same backend

---

## Goal 6: DNS Server for Tenant Zones

Each tenant's orchestrator runs a DNS server that serves authoritative records for the tenant's DNS zone. Non-zone queries are forwarded upstream.

### Requirements

- **G6.1**: DNS server listens on configurable UDP port (default 53)
- **G6.2**: Supports A record resolution (returns IPv4 address)
- **G6.3**: Supports SRV record resolution (returns priority, weight, port, target)
- **G6.4**: Supports configurable TTL per record
- **G6.5**: Non-authoritative queries are forwarded to configurable upstream DNS
- **G6.6**: DNS records are generated from tenant config (automatic zone generation)

### Test Cases

- [ ] **TC-050**: Query `api.app-frontend.internal` — verify returns configured VIP IP address
- [ ] **TC-051**: Query `_http._api.app-frontend.internal` SRV record — verify returns correct priority, weight, port, target
- [ ] **TC-052**: Set TTL to 30s, wait 35s, query again — verify new response (TTL honored)
- [ ] **TC-053**: Query `google.com` — verify forwarded to upstream DNS and resolved correctly
- [ ] **TC-054**: Query unknown tenant zone — verify NXDOMAIN response
- [ ] **TC-055**: Verify DNS server responds to queries on configured port

---

## Goal 7: TLS Termination at VIP

All HTTPS traffic terminates at the orchestrator. The orchestrator handles TLS handshakes and forwards plaintext HTTP to backends.

### Requirements

- **G7.1**: TLS termination configured with cert/key paths in tenant config
- **G7.2**: Supports TLS 1.2 and TLS 1.3
- **G7.3**: Supports common cipher suites (AES-GCM, ChaCha20-Poly1305)
- **G7.4**: HTTP/2 support over TLS
- **G7.5**: Client certificate verification (optional, configurable)
- **G7.6**: TLS session resumption (session tickets)

### Test Cases

- [ ] **TC-060**: Send HTTPS request to VIP — verify TLS handshake succeeds
- [ ] **TC-061**: Send HTTP/2 request — verify protocol negotiation succeeds
- [ ] **TC-062**: Send request with expired cert — verify connection rejected
- [ ] **TC-063**: Verify TLS 1.0 and TLS 1.1 connections are rejected
- [ ] **TC-064**: Send request with client certificate (when enabled) — verify accepted/rejected per config

---

## Goal 8: Web GUI

Web-based management interface for all tenant configuration and monitoring.

### Requirements

- **G8.1**: Login screen with username/password authentication
- **G8.2**: Tenant switcher (select which tenant to manage)
- **G8.3**: Tree view showing tenant hierarchy (region → datacenter → cluster → endpoint)
- **G8.4**: Sankey diagram showing traffic split visualization at selected level
- **G8.5**: Rolling update UI showing current step, progress bar, rollback button
- **G8.6**: DNS zone editor (add/edit/delete records)
- **G8.7**: Health dashboard showing status of all tenants and workers

### Test Cases

- [ ] **TC-070**: Log in with valid credentials — verify redirected to dashboard
- [ ] **TC-071**: Log in with invalid credentials — verify "invalid credentials" message
- [ ] **TC-072**: Switch between tenants — verify tree view updates to show selected tenant
- [ ] **TC-073**: Select a node in tree — verify Sankey diagram shows children and weights
- [ ] **TC-074**: View active rolling update — verify progress bar shows current step
- [ ] **TC-075**: Click rollback button during active update — verify rollback is triggered
- [ ] **TC-076**: Add DNS record in zone editor — verify record appears in config and DNS server
- [ ] **TC-077**: View health dashboard — verify all tenants show correct status

---

## Goal 9: Configuration Hot Reload

Configuration changes are detected and applied without restarting the tenant process.

### Requirements

- **G9.1**: Filesystem watcher detects changes to tenant.yaml
- **G9.2**: Config is validated before applying (weights sum to 100, no circular refs)
- **G9.3**: On validation failure, config change is rejected and error logged
- **G9.4**: On validation success, routing table is swapped atomically
- **G9.5**: Active connections are not dropped during reload
- **G9.6**: Reload happens within 100ms of config change

### Test Cases

- [ ] **TC-080**: Change weight from 70 to 75 while traffic is flowing — verify reload succeeds within 100ms
- [ ] **TC-081**: Set weights that don't sum to 100 — verify reload is rejected, error logged
- [ ] **TC-082**: Send 10,000 requests during config change — verify no requests are dropped
- [ ] **TC-083**: Change DNS record TTL — verify DNS server picks up new value without restart
- [ ] **TC-084**: Change TLS cert path — verify TLS picks up new cert without restart

---

## Goal 10: L4 / L7 Proxying and Forwarding Semantics

The orchestrator proxies real traffic, not just test requests. This goal covers the behaviour of
the proxy itself: correct header handling, correct failure responses, correct timeouts, and
transparent L4 passthrough.

> This goal was missing from the original plan. TCP and UDP proxying were scheduled in Phase 6 with
> no requirements and no test cases, so there was nothing to hold the implementation to.

### Requirements

- **G10.1**: HTTP requests are forwarded with the original method, path, query, and body
- **G10.2**: `X-Forwarded-For`, `X-Forwarded-Proto`, and `X-Real-IP` are set by the proxy, and any
  client-supplied value of the same name is **overwritten**, never appended
- **G10.3**: A unique request ID is attached to every request and echoed in the response
- **G10.4**: When no route matches, the proxy returns `404`
- **G10.5**: When a route matches but every endpoint is unhealthy, the proxy returns `503` with
  `X-Orchestrator-Reason: no-healthy-endpoints`
- **G10.6**: When a backend is unreachable, the proxy returns `502` after exhausting retries
- **G10.7**: When a backend exceeds the response timeout, the proxy returns `504`
- **G10.8**: Request and response bodies are streamed, not buffered in full
- **G10.9**: HTTP/1.1 keep-alive is used for upstream connections; connections are reused from a pool
- **G10.10**: TCP listeners forward raw byte streams transparently in both directions
- **G10.11**: UDP listeners forward datagrams to the configured target
- **G10.12**: A TCP or UDP listener that is not configured in the tenant config is not bound

### Test Cases

- [ ] **TC-090**: Send a request with a body — verify the backend receives the same method, path, query, and body
- [ ] **TC-091**: Send a request carrying `X-Forwarded-For: 1.2.3.4` — verify the backend sees the real client IP, not `1.2.3.4`
- [ ] **TC-092**: Send a request to an unmatched path — verify `404`
- [ ] **TC-093**: Mark every endpoint of a route unhealthy — verify `503` and the `X-Orchestrator-Reason` header
- [ ] **TC-094**: Point a route at a dead backend — verify `502`
- [ ] **TC-095**: Point a route at a backend that never responds — verify `504` after the response timeout
- [ ] **TC-096**: Send a 10 MB request body — verify it is streamed and the backend receives it intact
- [ ] **TC-097**: Send 100 sequential requests over one connection — verify the upstream connection is reused
- [ ] **TC-098**: Send traffic through a TCP listener — verify it arrives byte-identical at the target
- [ ] **TC-099**: Send a datagram through a UDP listener — verify it arrives at the target

---

## Goal 11: API Security and Authentication

The management API is the control plane of the traffic manager. It must be authenticated, it must
not leak information, and it must resist abuse. This is separate from Goal 8, which covers the
GUI's login screen — that is only the visible surface of this requirement.

### Requirements

- **G11.1**: All API endpoints require authentication except `/api/login`, `/api/health`, and
  `/api/version`
- **G11.2**: Passwords are stored as Argon2id hashes with per-user salts, in a file with mode `0600`
- **G11.3**: No secret material is stored in `tenant.yaml` or `registry.yaml`
- **G11.4**: Login failures always return the identical message `"Invalid credentials"` regardless
  of whether the username exists
- **G11.5**: Tokens are opaque, cryptographically random, and expire after 12 hours
- **G11.6**: Tokens are compared in constant time
- **G11.7**: The metrics listener binds to loopback by default and is not exposed on the public interface
- **G11.8**: API errors never include internal details — stack traces, file paths, or internal
  identifiers are logged, not returned
- **G11.9**: Request bodies are size-limited to prevent resource exhaustion
- **G11.10**: The DNS server refuses zone transfers (`AXFR`/`IXFR`) and `ANY` queries that would
  leak the full zone

### Test Cases

- [ ] **TC-100**: Call any protected endpoint without a token — verify `401`
- [ ] **TC-101**: Call any protected endpoint with a garbage token — verify `401`
- [ ] **TC-102**: Log in with a valid password — verify a token is returned and accepted
- [ ] **TC-103**: Log in with a wrong password for a **known** username — verify the error message
- [ ] **TC-104**: Log in with an **unknown** username — verify the error message is byte-identical to TC-103
- [ ] **TC-105**: Inspect the credential file — verify it contains only an Argon2id hash and not the password
- [ ] **TC-106**: Inspect `tenant.yaml` and `registry.yaml` — verify neither contains a secret
- [ ] **TC-107**: Use a token after it expires — verify `401`
- [ ] **TC-108**: Send a 100 MB request body to an API endpoint — verify `413` and no memory spike
- [ ] **TC-109**: Send an `AXFR` query to the DNS server — verify it is refused
- [ ] **TC-110**: Trigger an internal error — verify the response body contains no internal detail

---

## Traceability

Every requirement maps to a task in `05-implementation-plan.md`. See the **Requirement
Traceability** table at the end of that document. Every requirement also maps to at least one test
case in `10-test-strategy.md`.
