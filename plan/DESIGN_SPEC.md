# Orchestrator — Complete Design Specification

> **This file is generated. Do not edit it.**
>
> It is the concatenation of the numbered design documents in [`plan/`](.), which are the
> source of truth. To change the design, edit the numbered document and run
> `make design-spec`.
>
> | Document | Subject |
> |----------|---------|
> | [`01-project-overview.md`](01-project-overview.md) | Project Overview |
> | [`02-goals-and-acceptance-criteria.md`](02-goals-and-acceptance-criteria.md) | Goals & Acceptance Criteria |
> | [`03-architecture.md`](03-architecture.md) | Architecture |
> | [`04-code-standards.md`](04-code-standards.md) | Code Standards |
> | [`05-implementation-plan.md`](05-implementation-plan.md) | Implementation Plan |
> | [`06-tracker.md`](06-tracker.md) | Tracker |
> | [`07-config-schema.md`](07-config-schema.md) | Configuration Schema |
> | [`08-api-reference.md`](08-api-reference.md) | API Reference |
> | [`09-dependencies.md`](09-dependencies.md) | Dependency & Tooling Manifest |
> | [`10-test-strategy.md`](10-test-strategy.md) | Test Strategy |
> | [`11-non-functional-requirements.md`](11-non-functional-requirements.md) | Non-Functional Requirements & Definition of Done |
> | [`12-repo-layout-and-tooling.md`](12-repo-layout-and-tooling.md) | Repository Layout & Tooling |

---


## Problem Statement

Modern applications need fine-grained traffic control — routing percentages, canary deployments, blue-green rollouts, multi-datacenter failover, DNS-based routing. Existing solutions (F5, commercial load balancers) are:

- **Proprietary and expensive** — hardware appliances with vendor lock-in
- **Complex to automate** — closed config languages, heavy management planes
- **Not designed for modern apps** — built for monolithic datacenters, not distributed microservices
- **No DNS-level traffic routing** — L4/L7 proxy only, DNS is a separate product
- **Slow to provision** — days or weeks to set up new routing rules

## Solution

**Orchestrator** is an open-source, application-level traffic manager that:

- Routes traffic across regions, data centers, clusters, and endpoints with configurable weights
- Manages HA worker pairs with Linux kernel VIP failover
- Handles blue-green and canary rolling updates with auto-rollback on error thresholds
- Serves authoritative DNS for tenant zones
- Terminates TLS at the VIP level
- Runs per-tenant as an isolated process
- Provides a web GUI for configuration and visualization

## Key Differentiators vs F5

| F5 | Orchestrator |
|----|-------------|
| Hardware appliance, expensive | Commodity hardware, open source |
| Proprietary config language | YAML (human-readable, version-controllable) |
| Heavy centralized management | Lightweight YAML + GUI |
| L4/L7 proxy only | L4/L7 proxy + DNS-level routing |
| Monolithic binary | Per-tenant process isolation |
| Slow to provision | Spin up a tenant in seconds |
| Vendor lock-in | Open source, community-driven |
| Complex automation | GitOps-friendly, hot-reloadable |

## What It Does

1. **Routes traffic** to Kubernetes clusters and VM pools with weighted splits
2. **Manages HA worker pairs** with VIP failover via Linux kernel iproute2
3. **Handles blue-green + canary rolling updates** with auto-rollback on error thresholds
4. **Serves authoritative DNS** for tenant zones (basic A/SRV records, forwards everything else upstream)
5. **Terminates TLS** at the VIP level

## What It Does Not Do (Yet)

- Full DNS server (forwarding handled upstream)
- Advanced metrics (stubbed, Prometheus-ready, but not first priority)
- RBAC for GUI (username/password only, auth enhancements later)
- Kubernetes dependency (this is standalone)


---


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


---


## High-Level Design

```
┌──────────────────────────────────────────────────────────────┐
│                       Host (Linux)                            │
│                                                               │
│  orchestrator-manager (systemd)                               │
│  ├── Orchestrates tenant processes                           │
│  ├── Manages VIP pool                                        │
│  └── Watches config changes                                  │
│                                                               │
│  orchestrator-tenant-a (systemd) ── sync ── orchestrator-tenant-a (standby)
│  ├── Control Plane                                           │
│  │   ├── Config Engine                                       │
│  │   ├── Health Checker                                      │
│  │   ├── VIP Manager                                         │
│  │   ├── Rolling Update Manager                              │
│  │   ├── Session Store                                       │
│  │   └── REST API (axum)                                     │
│  ├── Data Plane                                              │
│  │   ├── HTTP/HTTPS Proxy (hyper)                            │
│  │   ├── TCP Proxy (tokio)                                   │
│  │   └── UDP Proxy (tokio)                                   │
│  ├── DNS Server (hickory-server)                             │
│  └── Sync Peer (gRPC to partner worker)                      │
│                                                               │
│  orchestrator-tenant-b (systemd) ── sync ── orchestrator-tenant-b (standby)
│  ... (more tenants)                                           │
│                                                               │
│  orchestrator-gui (separate web app)                          │
│  └── React + TypeScript, talks to tenant REST APIs           │
└──────────────────────────────────────────────────────────────┘
```

## The Three Pieces

| Piece | What | Deployed |
|-------|------|----------|
| `orchestrator` | Per-tenant: control plane + data plane proxy + DNS + VIP + sync | One process per tenant (×2 for HA) |
| `orchestrator-manager` | Tenant lifecycle, VIP pool, process supervisor | Once per host |
| `orchestrator-gui` | Web UI: tree view, Sankey diagram, DNS editor, rolling updates | Separate web app |

## Process Model

Each tenant runs as an independent systemd service.

```
Host:
  orchestrator-manager (systemd)
    ├── orchestrator-app-frontend (systemd) — Worker A (active)
    │       └── syncs with orchestrator-app-frontend (systemd) — Worker B (standby)
    ├── orchestrator-payments (systemd) — Worker A (active)
    │       └── syncs with orchestrator-payments (systemd) — Worker B (standby)
    └── orchestrator-search (systemd) — Worker A (active, no standby)
```

## Per-Tenant Process

### Control Plane

Manages the tenant's configuration, health, and routing state.

| Component | Responsibility |
|-----------|---------------|
| **Config Engine** | Parses tenant.yaml, validates, maintains in-memory routing graph |
| **Health Checker** | Probes all backends, marks unhealthy backends, triggers failover |
| **VIP Manager** | Acquires/releases VIP via iproute2 netlink |
| **Rolling Update Manager** | Manages blue-green rollout state, monitors metrics for rollback |
| **REST API** | Exposes configuration and metrics API for GUI (axum) |

### Data Plane

Handles the actual packet forwarding and routing.

| Component | Responsibility |
|-----------|---------------|
| **HTTP/HTTPS Proxy** | Terminates TLS, matches routes, forwards to backends (hyper) |
| **TCP Proxy** | L4 passthrough for non-HTTP traffic (tokio) |
| **UDP Proxy** | L4 passthrough for non-DNS UDP traffic (tokio) |
| **Session Store** | In-memory LRU cache for sticky sessions |

### DNS Server

Built using `hickory-server` (formerly `trust-dns-server`, renamed upstream — the old name is
pinned to an unmaintained 0.23 line).

| Feature | Detail |
|---------|--------|
| Authoritative for | Tenant DNS zone(s) |
| Records | Generated from tenant config |
| Non-zone queries | Forwarded to upstream DNS resolver |
| Supported record types | A, AAAA, SRV, CNAME |
| Zone transfers | `AXFR`/`IXFR` refused |

## Sync Channel

gRPC-based communication between active and standby workers.

### Messages

| Message | Purpose |
|---------|---------|
| `RoutingTable` | Full routing table snapshot |
| `HealthReport` | Health status of all backends |
| `StickySession` | Sticky session table entries (for failover) |
| `UpdateState` | Current rolling update state |
| `Heartbeat` | Keepalive |

### Active/Passive Design

- Only the active worker processes traffic
- Standby mirrors all state but does not route
- When ready for active/active, both workers route and sync coordinates decisions
- The sync protocol supports both modes — only the config flag changes

### Failover Sequence

1. The active worker stops sending heartbeats
2. After `peer_timeout` (default 1s) with no heartbeat, the standby declares the peer dead
3. The standby acquires the VIP — an address add on the interface
4. The standby promotes itself to active and begins routing
5. The recovered worker starts, sees a live peer, and comes up as standby (G3.6)

Total time to traffic-serving is bounded by `peer_timeout` plus the time to add the address —
comfortably under the 1-second requirement of G3.4.

### Split-Brain Prevention

If both workers believe they are active, they would both try to add the VIP. The kernel resolves
this: `ip addr add` on an address already present on the interface fails with `EEXIST`. The worker
that loses the race knows it did not acquire the VIP and demotes itself to standby. The VIP address
itself is the single source of truth for who is active — there is no separate consensus protocol.

## VIP Management

Uses Linux kernel `iproute2` via netlink. No custom protocol, no keepalived.

### Operations

```
ip addr add <VIP>/32 dev <interface>   # acquire VIP
ip addr del <VIP>/32 dev <interface>   # release VIP
```

The kernel handles ARP automatically. Standard HA cluster behavior.

### Implementation

```rust
// VIP binding via netlink.
//
// `rtnetlink` is used rather than the raw `netlink-packet-route` API: the raw API
// requires hand-building every netlink message, and we only need typed address
// add/delete operations.
use rtnetlink::{Handle, AddressAddBuilder, AddressReplaceBuilder};

let handle = Handle::new()?;
handle.address().add(AddressAddBuilder::new())
    .interface(iface_index)
    .address(vip_cidr)?
    .execute().await?;
```

`rtnetlink` also exposes `AddressList`, which is how the worker checks whether the peer already
holds the VIP before attempting acquisition.

### VIP Semantics

The VIP is an IP address added to an interface. There is **no ARP priority**, no gratuitous ARP
broadcast, and no keepalived-style priority model.

- **Acquire**: add the address to the interface
- **Release**: delete the address from the interface
- **Pre-check**: the acquirer lists the interface's addresses first. If the VIP is already present,
  it is assumed to be owned by the peer and acquisition is refused

> The `--vip-priority` flag from the original systemd example has been **removed**. It implied a
> model this design does not use. See `07-config-schema.md` §7.

## Manager

The `orchestrator-manager` is a single binary that:

1. Reads tenant registry from `/etc/orchestrator/registry.yaml`
2. Starts/stops per-tenant systemd services
3. Manages VIP pool assignments
4. Watches for config changes and signals **only the affected tenant** to reload (G1.5)
5. Monitors all tenant processes for health
6. Validates port allocations and refuses conflicting tenants before starting them
7. Serves the management API and the static GUI bundle

### GUI Delivery

The GUI is a static bundle embedded in the manager binary with `rust-embed` and served by the
manager. There is no separate web server and no Node.js runtime in production. The manager proxies
API calls to the selected tenant's REST API, which means the GUI only ever talks to one origin and
no CORS configuration is needed.

## Process Isolation

Each tenant runs as an independent systemd service.

### Isolation Mechanisms

- A separate systemd service per tenant, each with its own cgroup
- Resource limits via systemd (`MemoryMax`, `CPUQuota`)
- Separate config file per tenant
- No shared memory between tenants
- No shared state between tenants — the manager communicates with tenants over the network, never
  through shared state

> Tenants do **not** get a separate network namespace. A tenant must be reachable on the host
> network to own a VIP, so `PrivateNetwork=yes` would defeat the purpose. Isolation comes from
> service boundaries and cgroups, not from namespaces.

### systemd Example

```ini
[Service]
ExecStart=/usr/local/bin/orchestrator \
    --config /etc/orchestrator/tenants/app-frontend/tenant.yaml \
    --node-id worker-a \
    --http-bind 0.0.0.0:18080 \
    --https-bind 0.0.0.0:18443 \
    --metrics-bind 127.0.0.1:19090 \
    --sync-bind 0.0.0.0:19091 \
    --dns-bind 0.0.0.0:15353 \
    --sync-peer 10.0.2.11:19091
Restart=on-failure
RestartSec=3
MemoryMax=512M
CPUQuota=100%
PrivateNetwork=no
ProtectSystem=strict
ProtectHome=yes
NoNewPrivileges=yes
ReadWritePaths=/var/lib/orchestrator/tenants/app-frontend
```

> **Ports are per-tenant and per-worker.** Every tenant on a host needs its own port for every
> listener. The original example hardcoded `--metrics-port 9090`, which would collide across
> tenants on the same host. Ports are allocated in `registry.yaml` and validated by the manager
> for conflicts before any service starts. See `07-config-schema.md` §8.

## Health Checking

Each endpoint in the hierarchy is probed independently.

| Aspect | Behaviour |
|--------|-----------|
| Probe types | HTTP (`GET <path>`) or TCP (connect only) |
| Interval | `upstream.health_check.interval`, default 10s |
| Timeout | `upstream.health_check.timeout`, default 2s |
| Becoming healthy | After `healthy_threshold` consecutive successes (default 2) |
| Becoming unhealthy | After `unhealthy_threshold` consecutive failures (default 3) |
| State storage | In-memory, replicated to the standby via the `HealthReport` sync message |
| Effect on routing | Unhealthy endpoints are removed from the weighted selection immediately |
| Effect when all are unhealthy | The route returns `503` with `X-Orchestrator-Reason: no-healthy-endpoints` |

Health checks are **not** weighted — every endpoint is probed on the same schedule regardless of
its share of traffic. A 5% endpoint still needs to be known-healthy.

## Graceful Shutdown

`SIGTERM` starts a controlled shutdown:

1. The listener stops accepting new connections
2. In-flight requests drain for up to `drain_timeout` (default 30s)
3. The VIP is released so the peer can take over cleanly
4. The process exits `0`

A `SIGKILL` cannot be handled — the VIP leaks until the peer detects the missing heartbeat within
`peer_timeout` and takes over. That path is the one covered by TC-021.

## Configuration Reload

Two mechanisms, both ending at the same place:

| Mechanism | Who triggers it | When |
|-----------|-----------------|------|
| `SIGHUP` | The manager, after its inotify watcher sees the file change | The primary path |
| Filesystem watch | The tenant itself, via `notify` | Used when the tenant runs without a manager |

Either way the sequence is identical:

1. Read the file
2. Parse and validate — **all** errors are collected, not just the first
3. On failure: log the errors, keep the current config running, increment
   `orchestrator_config_reload_total{result="failure"}` (G9.3)
4. On success: build the new routing table and swap it atomically via `arc-swap` (G9.4)
5. Old connections keep running against the old table until they finish (G9.5)

The atomic swap is a pointer swap, not a rebuild in place. Readers never see a partially-built
routing table, and no reader-side lock is needed.

## Port Allocation

Each tenant needs a distinct set of ports on a host:

| Port | Purpose | Default |
|------|---------|---------|
| HTTP | Plaintext proxy listener | 8080 |
| HTTPS | TLS proxy listener | 8443 |
| Metrics | Prometheus exposition | 9090 |
| Sync | gRPC worker sync channel | 9091 |
| DNS | Authoritative DNS | 53 |

The manager allocates these from `registry.yaml` and **refuses to start a tenant whose ports
conflict** with another tenant on the same host. The manager API returns `409` on a conflict
before anything is written to disk.

## Config Delivery

Config is mounted as a volume into each tenant process. The instance is immutable; config is mutable.

```
Instance Image (immutable)
├── orchestrator binary (no config, no dependencies)
└── Updated once per release

Config Volume (per-tenant, mutable)
├── /etc/orchestrator/tenants/<name>/tenant.yaml
└── /etc/orchestrator/tenants/<name>/ssl/
```

Config changes are hot-reloaded via SIGHUP signal. No restart required.

## Migration Path: systemd → Custom Process Manager

systemd is used for now. A custom Rust-based process manager can replace it later with zero tenant config changes.

| Aspect | systemd | Custom Manager |
|--------|---------|---------------|
| Service management | systemd unit files | Rust process groups |
| Config delivery | bind mount | in-process volume |
| Reload signal | SIGHUP | custom signal or API |
| Resource limits | cgroups | Rust cgroup bindings |
| Tenant config | Same YAML | Same YAML |

## DNS + Traffic Routing

Both DNS-level and VIP-level routing are supported:

| Level | How | Precision |
|-------|-----|-----------|
| DNS | Returns different VIPs per query | TTL-dependent, client-side |
| VIP | Single VIP, server-side weighted routing | Immediate, server-side |

The DNS server returns VIPs. The VIP then routes traffic to backends according to configured weights.


---


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


---


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


---


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


---


Normative specification of `tenant.yaml` and `registry.yaml`. Every field used by any other
document must appear here. Anything not listed here is rejected by the validator with
`unknown field` errors.

---

## 1. Global Rules

| Rule | Value |
|------|-------|
| Format | YAML 1.2, parsed with `serde-saphyr` (pure Rust) |
| Root document | Single mapping (no multi-doc). Duplicate keys are rejected |
| Unknown fields | **Rejected** (`deny_unknown_fields`) — typos fail loudly instead of silently doing nothing |
| `version` field | **Required**, integer, currently `1` |
| Duration strings | `s`, `m`, `h` suffixes — `30s`, `5m`, `1h`. Bare integers are rejected |
| Size strings | `K`, `M`, `G` suffixes — `512M`, `2G` |
| Byte sizes | `u64` |
| Weights | `u32`, `0..=100` |
| Durations in config | `humantime`-style strings, parsed into `std::time::Duration` |

### Schema Versioning

`version` is the **config schema version**, not the product version.

- The validator accepts `version <= CURRENT_SCHEMA_VERSION`.
- If a config declares a `version` greater than the binary understands, the binary **refuses to
  load it** and logs `config schema version {v} is newer than supported {current}`.
- New optional fields may be added without bumping `version`. Removing or changing the meaning of
  a field **requires** a bump.
- The binary ships a `migrate` subcommand: `orchestrator migrate --from 1 --to 2 --in tenant.yaml
  --out tenant.v2.yaml`. Migrations are additive and total (never lossy) — if a migration cannot
  represent a config losslessly, it errors instead of guessing.

This matters because the tool is explicitly GitOps-driven: a config written by a newer binary must
fail closed on an older binary rather than be partially applied.

---

## 2. `tenant.yaml` — Full Example

```yaml
version: 1

tenant:
  name: app-frontend
  zone: app-frontend.internal
  description: "Public frontend for the app"

# ---------------------------------------------------------------------------
# Virtual IP
# ---------------------------------------------------------------------------
vip:
  address: 10.0.0.100/32
  interface: eth0

# ---------------------------------------------------------------------------
# Listeners
# ---------------------------------------------------------------------------
listeners:
  http:
    bind: 0.0.0.0:8080
  https:
    bind: 0.0.0.0:8443
    tls:
      cert: /etc/orchestrator/tenants/app-frontend/ssl/fullchain.pem
      key: /etc/orchestrator/tenants/app-frontend/ssl/privkey.pem
      ca: /etc/orchestrator/tenants/app-frontend/ssl/ca.pem   # optional, for client auth
      client_auth: none          # none | optional | required
      min_version: "1.2"         # "1.2" | "1.3"
      alpn: ["h2", "http/1.1"]
  tcp:
    - name: postgres
      bind: 0.0.0.0:5432
      target: 10.1.2.10:5432
  udp:
    - name: wireguard
      bind: 0.0.0.0:51820
      target: 10.1.2.11:51820

# ---------------------------------------------------------------------------
# Upstream / proxy behaviour
# ---------------------------------------------------------------------------
upstream:
  connect_timeout: 2s
  response_timeout: 30s
  idle_timeout: 60s
  max_connections_per_endpoint: 1024
  retries: 1
  health_check:
    type: http                 # http | tcp
    path: /healthz             # http only
    interval: 10s
    timeout: 2s
    healthy_threshold: 2
    unhealthy_threshold: 3

# ---------------------------------------------------------------------------
# Routing hierarchy: region -> datacenter -> cluster -> endpoint
# ---------------------------------------------------------------------------
hierarchy:
  regions:
    - name: us-east
      weight: 60
      datacenters:
        - name: dc-1
          weight: 70
          clusters:
            - name: prod
              weight: 100
              endpoints:
                - name: ep-a
                  weight: 60
                  address: 10.1.1.5:8080
                - name: ep-b
                  weight: 40
                  address: 10.1.1.6:8080
    - name: eu-west
      weight: 40
      datacenters:
        - name: dc-2
          weight: 100
          clusters:
            - name: prod
              weight: 100
              endpoints:
                - name: ep-c
                  weight: 100
                  address: 10.2.1.5:8080

# ---------------------------------------------------------------------------
# Routes
# ---------------------------------------------------------------------------
routes:
  - name: api-v2
    enabled: true
    match:
      path: /api/v2
      type: prefix             # prefix | exact
    scope:
      region: "*"              # "*" = any region, otherwise an exact region name
    affinity:
      method: cookie           # none | cookie | header | client-ip
      cookie_name: ORCHSID
      header_name: X-Session-ID
      ttl: 30m

  - name: health
    enabled: true
    match:
      path: /health
      type: exact
    scope:
      region: "*"
    affinity:
      method: none

# ---------------------------------------------------------------------------
# Blue/green rollout (attached to a route)
# ---------------------------------------------------------------------------
rollout:
  route: api-v2
  strategy: blue-green         # blue-green | canary
  blue:
    region: us-east
    datacenter: dc-1
    cluster: prod
  green:
    region: us-east
    datacenter: dc-1
    cluster: prod-green
  steps: [0, 25, 50, 75, 100]
  step_duration: 5m
  mode: gradual                # gradual | instant | manual
  auto_rollback:
    enabled: true
    error_rate_threshold: 0.05
    latency_p99_threshold: 2s
    window: 1m
    min_samples: 100
  notify:
    webhook: https://hooks.internal/orchestrator
    events: [rollback, complete, failed]

# ---------------------------------------------------------------------------
# DNS
# ---------------------------------------------------------------------------
dns:
  enabled: true
  listen: 0.0.0.0:53
  upstream:
    - 1.1.1.1
    - 8.8.8.8
  ttl: 60
  records:
    - name: api
      type: A
      value: 10.0.0.100
      ttl: 30
    - name: _http._api
      type: SRV
      priority: 10
      weight: 10
      port: 443
      target: api.app-frontend.internal

# ---------------------------------------------------------------------------
# Observability
# ---------------------------------------------------------------------------
metrics:
  enabled: true
  bind: 127.0.0.1:9090

logging:
  level: info                  # error | warn | info | debug | trace
  format: json                 # json | compact

# ---------------------------------------------------------------------------
# Worker identity / HA
# ---------------------------------------------------------------------------
worker:
  mode: active-passive         # active-passive | active-active
  node_id: worker-a            # stable identifier, distinguishes the two workers
  sync:
    listen: 0.0.0.0:9091
    peer: 10.0.2.11:9091
    heartbeat_interval: 300ms
    peer_timeout: 1s
```

---

## 3. Field Reference

### 3.1 `tenant`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | Must match the directory name under `/etc/orchestrator/tenants/` and the registry entry |
| `zone` | string | yes | Authoritative DNS zone. Must be a valid DNS name |
| `description` | string | no | Free text, shown in the GUI |

### 3.2 `vip`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `address` | CIDR | yes | IPv4/IPv6 CIDR. The host part must be a single address (`/32` or `/128`) |
| `interface` | string | yes | Link name, e.g. `eth0`. Must exist at runtime |
| `priority` | u8 | no | Advisory priority used only for ordering *among multiple VIPs on the same host*. **Not** an ARP priority — see [§7](#7-vip-semantics) |

### 3.3 `listeners`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `http.bind` | socket | yes | Plaintext listener. Optional at the top level — omit the `http` key entirely to disable |
| `https.bind` | socket | no | TLS listener |
| `https.tls.cert` | path | yes (if `https`) | PEM chain. Hot-reloadable (G9.6 / TC-084) |
| `https.tls.key` | path | yes (if `https`) | PEM private key. Hot-reloadable |
| `https.tls.ca` | path | no | CA bundle for client-cert verification |
| `https.tls.client_auth` | enum | no | `none` (default) / `optional` / `required` |
| `https.tls.min_version` | string | no | `1.2` (default) / `1.3`. Values below 1.2 are rejected at validation time — TLS 1.0/1.1 must never be negotiable (G7.2, TC-063) |
| `https.tls.alpn` | string[] | no | Default `["h2", "http/1.1"]` |
| `tcp[]` | list | no | Name, `bind`, `target` — raw L4 passthrough |
| `udp[]` | list | no | Name, `bind`, `target` — raw L4 passthrough |

### 3.4 `upstream`

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `connect_timeout` | duration | `2s` | Backend connect timeout |
| `response_timeout` | duration | `30s` | Time to first byte from backend |
| `idle_timeout` | duration | `60s` | Keep-alive idle timeout for pooled connections |
| `max_connections_per_endpoint` | u32 | `1024` | Connection pool cap |
| `retries` | u32 | `1` | Retries on **connect** failure only. Never retry a request that has begun receiving a response body |
| `health_check.type` | enum | `http` | `http` or `tcp` |
| `health_check.path` | string | `/healthz` | HTTP only |
| `health_check.interval` | duration | `10s` | Probe period |
| `health_check.timeout` | duration | `2s` | Per-probe timeout |
| `health_check.healthy_threshold` | u32 | `2` | Consecutive successes before an endpoint is marked healthy |
| `health_check.unhealthy_threshold` | u32 | `3` | Consecutive failures before an endpoint is marked unhealthy |

### 3.5 `hierarchy`

Four levels, each with `name` and `weight`. The three inner levels are nested inside their parent.
An endpoint has an extra `address` field (`host:port`).

```
regions[]        name, weight, datacenters[]
datacenters[]    name, weight, clusters[]
clusters[]       name, weight, endpoints[]
endpoints[]      name, weight, address
```

**Name uniqueness**: names must be unique *among siblings*. A cluster named `prod` may exist under
two different datacenters.

### 3.6 `routes`

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `name` | string | yes | Unique within the tenant |
| `enabled` | bool | no | Default `true` |
| `match.path` | string | yes | See [§4](#4-route-matching) |
| `match.type` | enum | no | `prefix` (default) / `exact` |
| `scope.region` | string | no | `"*"` (default, any region) or an exact region name |
| `affinity.method` | enum | no | `none` (default) / `cookie` / `header` / `client-ip` |
| `affinity.cookie_name` | string | no | Required when method is `cookie` |
| `affinity.header_name` | string | no | Required when method is `header` |
| `affinity.ttl` | duration | no | Default `30m` (G5.5) |

### 3.7 `rollout`

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `route` | string | — | Name of the route this rollout applies to. Required |
| `strategy` | enum | — | `blue-green` / `canary`. Required |
| `blue` / `green` | ref | — | Hierarchy reference (`region`/`datacenter`/`cluster`) |
| `steps` | u32[] | — | Percentage of traffic to green at each step. Must start at `0`, end at `100`, and be strictly increasing |
| `step_duration` | duration | — | How long each step runs before auto-advancing |
| `mode` | enum | `gradual` | `gradual` / `instant` / `manual` (G4.2, G4.3, G4.4) |
| `auto_rollback.enabled` | bool | `true` | |
| `auto_rollback.error_rate_threshold` | f64 | `0.05` | 0.05 = 5% (G4.5) |
| `auto_rollback.latency_p99_threshold` | duration | `2s` | (G4.6) |
| `auto_rollback.window` | duration | `1m` | Evaluation window (G4.7) |
| `auto_rollback.min_samples` | u32 | `100` | Do not evaluate until this many requests land in the window — prevents rollback on a single failure |
| `notify.webhook` | url | — | HTTP POST on rollback / completion (G4.9) |
| `notify.events` | string[] | — | `rollback` / `complete` / `failed` |

### 3.8 `dns`

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `enabled` | bool | `true` | |
| `listen` | socket | — | UDP port (G6.1). TCP transport on the same port is optional |
| `upstream` | ip[] | — | Forwarders for non-authoritative queries (G6.5) |
| `ttl` | u32 | `60` | Default TTL for generated records (G6.4) |
| `records[]` | list | — | Explicit records; see below |

Record fields:

| Type | Fields |
|------|--------|
| `A` | `name`, `type`, `value` (IPv4), `ttl` |
| `AAAA` | `name`, `type`, `value` (IPv6), `ttl` |
| `SRV` | `name`, `type`, `priority`, `weight`, `port`, `target`, `ttl` |
| `CNAME` | `name`, `type`, `value`, `ttl` |

**Automatic zone generation** (G6.6): if `records` is empty or omitted, the DNS server generates
an `A` record for every route's public hostname pointing at the tenant VIP, using `dns.ttl`.

### 3.9 `worker`

| Field | Type | Default | Notes |
|-------|------|---------|-------|
| `mode` | enum | `active-passive` | `active-passive` / `active-active` (experimental) |
| `node_id` | string | — | Stable identifier for this worker |
| `sync.listen` | socket | — | gRPC sync server bind address |
| `sync.peer` | socket | — | Peer worker address |
| `sync.heartbeat_interval` | duration | `300ms` | |
| `sync.peer_timeout` | duration | `1s` | Must be < 1s to satisfy G3.4 |

---

## 4. Route Matching

### 4.1 Matching Rules

1. Routes are evaluated **independently** — every route whose `match` succeeds is a candidate.
2. Within a candidate set, **longest prefix wins**. `/api/v2/users` beats `/api/v2` beats `/`.
3. `type: exact` matches only when the path is byte-identical.
4. `type: prefix` matches when the request path starts with the configured prefix.
5. **Path boundaries are respected**: a `prefix` of `/api` matches `/api` and `/api/v2` but **not**
   `/apiv2`. A prefix matches only if the next character in the request path is `/` or the request
   path is exactly equal to the prefix.
6. Query strings are **not** part of the match. `?foo=bar` is ignored.
7. The fragment (`#`) is not sent to the server and is not part of the match.
8. Matching is **case-sensitive** on the path (RFC 3986 §6.2.2.1).
9. If no route matches, the proxy returns **`404`**.
10. If a route matches but every endpoint in its scope is unhealthy, the proxy returns **`503`** and
    sets header `X-Orchestrator-Reason: no-healthy-endpoints`.

### 4.2 Wildcard Syntax

The `*` wildcard described in G2.4 (`/api/v2/*`) is **not** a literal character pattern. It is
normalised at parse time: a configured path ending in `/*` has the `/*` stripped and is treated as a
`prefix` match. `/api/v2/*` and `/api/v2` are therefore equivalent. This keeps the config readable
while keeping the matcher implementation simple and auditable.

---

## 5. Weight Composition

For an endpoint `e` nested at region `r` → datacenter `d` → cluster `c`:

```
P(e) = S(r) × S(d) × S(c) × S(e)
```

where `S(x)` is the **share** a node receives from its parent:

```
S(x) = 1.0                    if x is the only child of its parent
S(x) = weight(x) / 100        otherwise
```

- Sibling weights must sum to exactly `100` at every level (G2.2). A sum of `0` is an error (no
  reachable endpoint).
- **A lone child receives its parent's whole share, regardless of its declared weight.** Without
  this rule, a hierarchy whose only datacenter is declared at weight 70 would silently drop 30%
  of its region's traffic — the weights would not compose to 1.0 and a share of every request
  would be lost. The validator accepts any weight for a lone child; the declared value is
  advisory in that case.
- The product across all levels always sums to 1.0 across the leaves, so the distribution is
  well-formed.

### Selection Algorithm

Selection is **weighted random** using a per-request random draw (G2.3), implemented as a
cumulative-distribution lookup over the leaf list:

```
r = uniform(0.0, 1.0)
walk leaves in config order, accumulating P(e); return the first leaf where r <= cumulative
```

This is statistically correct, O(n) with n = number of leaves, and does not require a per-leaf
random call. **Not** smooth weighted round-robin: the requirement explicitly asks for random
selection, and a deterministic round-robin would make the distribution tests pass trivially while
masking real randomness bugs.

### Worked Example

```
regions:
  us-east (60)
    datacenters:
      dc-1 (70)          # the only datacenter — receives all of us-east's share
        clusters:
          prod (100)      # the only cluster
            endpoints:
              ep-a (60)
              ep-b (40)
  eu-west (40)
    datacenters:
      dc-2 (100)
        clusters:
          prod (100)
            endpoints:
              ep-c (100)
```

| Leaf | Computation | Effective share |
|------|-------------|-----------------|
| `us-east/dc-1/prod/ep-a` | 0.60 × 1.0 × 1.0 × 0.60 | **36.0%** |
| `us-east/dc-1/prod/ep-b` | 0.60 × 1.0 × 1.0 × 0.40 | **24.0%** |
| `eu-west/dc-2/prod/ep-c` | 0.40 × 1.0 × 1.0 × 1.0 | **40.0%** |

Total = **100.0%**.

Note that `dc-1`'s declared weight of 70 does not appear in the computation. It has no sibling to
compete with, so it takes the whole of `us-east`'s 60%. If `us-east` gained a second datacenter at
weight 30, that datacenter would immediately start taking 30% of the region's traffic and `dc-1`
would drop to 70% of it — with no change to `dc-1`'s own configuration.

### Scope Restriction

`scope.region` narrows a route to a single region. The weights **within** the selected region are
re-normalised to sum to 1.0. A route scoped to `us-east` distributes `ep-a` / `ep-b` at 60/40
(their intra-region ratio), *not* at 36/24.

---

## 6. Session Affinity

| Method | Key derivation | Behaviour |
|--------|----------------|-----------|
| `cookie` | Value of the named cookie | If absent, the request is treated as a new session: a backend is chosen by weight and a `Set-Cookie` is returned |
| `header` | Value of the named header | Same as cookie but no `Set-Cookie` is emitted |
| `client-ip` | Client IP address | Same as cookie but no `Set-Cookie` is emitted |
| `none` | — | Pure weighted random, no state kept |

- The session key is hashed (`ahash`) and mapped to a backend by **consistent hashing over the
  healthy endpoint set**, not by a stored pointer. This means the mapping survives an endpoint
  being marked unhealthy, and the standby worker can rebuild the same mapping from the synced
  session table (G3.3, `StickySession` message).
- Expiry is lazy: entries older than `affinity.ttl` are treated as absent and evicted on access
  (G5.7). The store is a bounded LRU (`moka`), capped at 100k entries per tenant.
- `Set-Cookie` attributes: `Path=/`, `HttpOnly`, `SameSite=Lax`, `Secure` when the request arrived
  over TLS.
- The session store is **per tenant process** and is replicated to the standby via the
  `StickySession` sync message.

---

## 7. VIP Semantics

The VIP is an IP address added to an interface. There is no ARP priority, no gratuitous ARP
broadcast, and no `net.ipv4.ip_nonlocal_bind` trickery.

- **Acquire**: `ip addr add <vip>/32 dev <iface>` (via `rtnetlink`)
- **Release**: `ip addr del <vip>/32 dev <iface>`
- **Pre-check**: the acquirer checks whether the address is already present on the interface. If it
  is present and owned by the peer, acquisition is refused.

> The `--vip-priority` flag shown in the architecture document's systemd example is **removed**.
> It implies an ARP-priority model (keepalived style) that this design does not use. A VIP is
> either present on the interface or it is not.

**Graceful release**: on `SIGTERM` the worker stops accepting new connections, drains in-flight
requests (up to `drain_timeout`, default 30s), then releases the VIP. A hard kill (`SIGKILL`)
leaks the VIP until the peer takes it over — the peer detects this via the missing heartbeat
within `peer_timeout` and performs the takeover (G3.4, TC-021).

---

## 8. `registry.yaml` — Host-Level Tenant Registry

Read by `orchestrator-manager` from `/etc/orchestrator/registry.yaml`.

```yaml
version: 1

defaults:
  interface: eth0
  resources:
    memory_max: 512M
    cpu_quota: 100%
  tls:
    cert_dir: /etc/orchestrator/tenants/{tenant}/ssl

tenants:
  - name: app-frontend
    enabled: true
    config: /etc/orchestrator/tenants/app-frontend/tenant.yaml
    vip: 10.0.0.100/32
    interface: eth0
    workers:
      - node_id: worker-a
        host: 10.0.2.10
        ports:
          http: 18080
          https: 18443
          metrics: 19090
          sync: 19091
          dns: 15353
      - node_id: worker-b
        host: 10.0.2.11
        ports:
          http: 18080
          https: 18443
          metrics: 19090
          sync: 19091
          dns: 15353
    resources:
      memory_max: 512M
      cpu_quota: 100%
```

### Registry Field Reference

| Field | Type | Required | Notes |
|-------|------|----------|-------|
| `tenants[].name` | string | yes | Must match `tenant.yaml`'s `tenant.name` |
| `tenants[].enabled` | bool | no | Default `true`. Disabled tenants are not started |
| `tenants[].config` | path | yes | Absolute path to `tenant.yaml` |
| `tenants[].vip` | CIDR | yes | Must match `tenant.yaml`'s `vip.address` |
| `tenants[].interface` | string | yes | Link name for the VIP |
| `tenants[].workers[]` | list | yes | One or two workers. Two = active/passive pair (G3.1) |
| `tenants[].workers[].node_id` | string | yes | Matches `worker.node_id` in `tenant.yaml` |
| `tenants[].workers[].host` | ip | yes | Address of the worker host |
| `tenants[].workers[].ports.*` | u16 | yes | **Per-tenant, per-worker port allocation.** Required — see below |
| `tenants[].resources.*` | string | no | systemd `MemoryMax` / `CPUQuota` (G1.3) |

### Port Allocation

Every tenant on a host needs its **own** port for every listener. The architecture document's
systemd example hardcodes `--metrics-port 9090`, which would collide across tenants on the same
host. The registry therefore carries an explicit port map per worker, and the manager generates
the systemd unit with these values substituted.

The manager validates at load time that **no two workers on the same host bind the same port**,
and refuses to start a tenant whose ports conflict.

---

## 9. Validation Rules (Normative)

The validator rejects a config with a clear error when any of the following hold. These map
one-to-one onto `ConfigError` variants in `orchestrator-core`.

| # | Rule | Error variant |
|---|------|---------------|
| V1 | `version` missing or newer than supported | `UnsupportedVersion` |
| V2 | Unknown field present anywhere | `UnknownField` |
| V3 | Sibling weights do not sum to 100 | `WeightSumMismatch` |
| V4 | A weight is outside 0–100 | `InvalidWeight` |
| V5 | A hierarchy level has zero children | `EmptyHierarchyLevel` |
| V6 | Duplicate sibling names | `DuplicateName` |
| V7 | A route references a region that does not exist | `UnknownRegion` |
| V8 | A rollout references a route / blue / green target that does not exist | `UnknownRolloutTarget` |
| V9 | `rollout.steps` does not start at 0, end at 100, or is not strictly increasing | `InvalidRolloutSteps` |
| V10 | `affinity.method` is `cookie`/`header` without the corresponding name field | `MissingAffinityTarget` |
| V11 | `tls.min_version` is below `1.2` | `InsecureTlsVersion` |
| V12 | `tls.cert`/`tls.key` are not readable (when TLS is enabled) | `TlsMaterialUnavailable` |
| V13 | The VIP address is not a host address (`/32` or `/128`) | `InvalidVipAddress` |
| V14 | A duration string is malformed | `InvalidDuration` |
| V15 | `dns.upstream` is empty while DNS is enabled | `NoUpstreamDns` |
| V16 | Two routes have the same name | `DuplicateName` |
| V17 | `route.match.path` does not start with `/` | `InvalidPath` |

Validation is **total**: the validator collects every error and reports them together rather than
failing on the first, so an operator fixing a config sees the full list. On a hot-reload, a
validation failure means the **previous config stays active** and the error is logged (G9.3,
TC-081).


---


Two REST APIs exist:

| API | Served by | Audience |
|-----|-----------|----------|
| **Manager API** | `orchestrator-manager` | Operator / CLI / automation |
| **Tenant API** | `orchestrator` (per tenant) | GUI, monitoring, rollout control |

Both are HTTP/1.1 + JSON. The Tenant API additionally supports HTTP/2 over TLS (ALPN `h2`).

---

## 1. Common Conventions

| Aspect | Convention |
|--------|------------|
| Base path | `/api` |
| Content type | `application/json` |
| Auth | `Authorization: Bearer <token>` for all endpoints except `/api/login` and `/api/health` |
| Errors | RFC 9457 problem details (`application/problem+json`) |
| Timestamps | RFC 3339, UTC, e.g. `2026-11-12T09:14:22Z` |
| Durations | Human-readable strings (`"30s"`) |
| Pagination | `?limit=` (default 100, max 1000) + `?offset=` |
| Idempotency | `POST` endpoints that create resources accept `Idempotency-Key` |

### Error Shape

```json
{
  "type": "https://orchestrator.dev/errors/validation-failed",
  "title": "Validation failed",
  "status": 422,
  "detail": "config validation produced 2 errors",
  "errors": [
    { "field": "hierarchy.regions[0].weight", "message": "sibling weights sum to 90, expected 100" },
    { "field": "routes[1].match.path", "message": "path must start with '/'" }
  ],
  "instance": "/api/tenants/app-frontend/reload"
}
```

### Status Codes

| Code | Meaning |
|------|---------|
| `200` | Success |
| `201` | Created |
| `400` | Malformed request |
| `401` | Missing or invalid token |
| `403` | Authenticated but not authorised |
| `404` | Not found |
| `409` | Conflicting state (e.g. rolling update already active) |
| `422` | Semantically invalid (validation errors) |
| `429` | Rate limited |
| `500` | Internal error — details are logged, never returned |
| `503` | Component not ready (e.g. no healthy endpoints) |

---

## 2. Authentication

The GUI authenticates with a username and password. The credentials are **not** in any config
file (per the code standards' "no secrets in config" rule).

| Item | Value |
|------|-------|
| Credential store | `/etc/orchestrator/auth.json`, mode `0600`, owned by the manager user |
| Password hashing | Argon2id (via the `argon2` crate), per-user random salt |
| Token format | Opaque random 256-bit token (base64url) |
| Token lifetime | 12 hours, sliding — each authenticated request refreshes the expiry |
| Token storage (server) | In-memory hash set, lost on manager restart |
| Token storage (client) | `HttpOnly`, `SameSite=Strict`, `Secure` cookie |

### `POST /api/login`

```json
// request
{ "username": "admin", "password": "hunter2" }

// 200 response
{
  "token": "8f2c…",
  "expires_at": "2026-11-12T21:14:22Z",
  "user": { "username": "admin", "role": "admin" }
}
```

**Failure** returns `401` with a problem-details body. The error message is always the literal
string `"Invalid credentials"` — it never distinguishes "unknown user" from "wrong password" (no
user enumeration).

### `POST /api/logout`

Invalidates the presented token. Returns `204`.

### `GET /api/me`

Returns the authenticated user. Used by the GUI to restore a session on page load.

---

## 3. Manager API

Base URL: `http://<manager-host>:8081`

### Tenants

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/tenants` | List all registered tenants with status |
| `GET` | `/api/tenants/{name}` | Get one tenant's registry entry |
| `POST` | `/api/tenants` | Register a new tenant (creates the directory, writes a template `tenant.yaml`, installs the systemd unit) |
| `PATCH` | `/api/tenants/{name}` | Update the registry entry (ports, resources, VIP, enabled) |
| `DELETE` | `/api/tenants/{name}` | Deregister the tenant, stop its services, remove its unit file. Requires `?purge=true` to also delete the config directory |
| `GET` | `/api/tenants/{name}/health` | Aggregated health of the tenant's workers |
| `POST` | `/api/tenants/{name}/start` | Start the tenant's systemd service(s) |
| `POST` | `/api/tenants/{name}/stop` | Stop the tenant's systemd service(s) |
| `POST` | `/api/tenants/{name}/restart` | Restart the tenant's systemd service(s) |
| `POST` | `/api/tenants/{name}/reload` | Send `SIGHUP` to the tenant process(es) to trigger a config reload |
| `GET` | `/api/tenants/{name}/metrics` | Prometheus-format metrics for all workers of the tenant |

**`POST /api/tenants` request body**

```json
{
  "name": "payments",
  "vip": "10.0.0.101/32",
  "interface": "eth0",
  "workers": [
    { "node_id": "worker-a", "host": "10.0.2.10", "ports": { "http": 18090, "https": 18493, "metrics": 19100, "sync": 19101, "dns": 15363 } },
    { "node_id": "worker-b", "host": "10.0.2.11", "ports": { "http": 18090, "https": 18493, "metrics": 19100, "sync": 19101, "dns": 15363 } }
  ],
  "resources": { "memory_max": "512M", "cpu_quota": "100%" }
}
```

Returns `201` with the created registry entry. **Port conflicts on the same host are rejected with
`409`** before anything is written.

### Cluster / VIP Pool

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/vip-pool` | All VIPs and their assignments |
| `POST` | `/api/vip-pool/allocate` | Allocate a free VIP from the configured pool |
| `POST` | `/api/vip-pool/release` | Release a VIP back to the pool |

### System

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/health` | Liveness probe of the manager. No auth |
| `GET` | `/api/version` | Build version, git SHA, schema version. No auth |
| `GET` | `/api/events` | Server-sent events stream of tenant lifecycle events |

---

## 4. Tenant API

Base URL: `http://<tenant-host>:<http_port>/api` (or `https://` for the TLS listener)

### Routing

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/config` | The active, validated configuration |
| `GET` | `/api/routes` | The active routing table, including resolved weights per leaf |
| `GET` | `/api/routes/{name}` | One route |
| `GET` | `/api/hierarchy` | The hierarchy tree as nested JSON |
| `GET` | `/api/hierarchy/effective-weights` | Flattened list of endpoints with their **effective** probability, computed with the multiplicative rule |

**`GET /api/routes/{name}` response**

```json
{
  "name": "api-v2",
  "match": { "path": "/api/v2", "type": "prefix" },
  "scope": { "region": "*" },
  "affinity": { "method": "cookie", "cookie_name": "ORCHSID", "ttl": "30m" },
  "endpoints": [
    { "name": "us-east/dc-1/prod/ep-a", "address": "10.1.1.5:8080", "effective_weight": 0.252, "healthy": true },
    { "name": "us-east/dc-1/prod/ep-b", "address": "10.1.1.6:8080", "effective_weight": 0.168, "healthy": true },
    { "name": "eu-west/dc-2/prod/ep-c", "address": "10.2.1.5:8080", "effective_weight": 0.400, "healthy": true }
  ],
  "rollout": { "state": "idle", "current_step": 0, "green_percent": 0 }
}
```

### Health

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/health` | Aggregate process health. No auth |
| `GET` | `/api/endpoints` | Per-endpoint health, last probe result, latency |
| `GET` | `/api/worker` | This worker's role (`active` / `standby`), VIP ownership, sync peer status |

### Rolling Updates

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/rollout` | Current rollout state for the tenant |
| `POST` | `/api/rollout` | Start a rollout for a route |
| `POST` | `/api/rollout/advance` | Advance one step (manual mode) |
| `POST` | `/api/rollout/rollback` | Trigger an immediate rollback |
| `POST` | `/api/rollout/promote` | Skip to the final step (100% green) |
| `POST` | `/api/rollout/pause` | Pause an auto-advancing rollout |
| `POST` | `/api/rollout/resume` | Resume a paused rollout |

**`POST /api/rollout` request**

```json
{
  "route": "api-v2",
  "strategy": "blue-green",
  "blue": { "region": "us-east", "datacenter": "dc-1", "cluster": "prod" },
  "green": { "region": "us-east", "datacenter": "dc-1", "cluster": "prod-green" },
  "steps": [0, 25, 50, 75, 100],
  "step_duration": "5m",
  "mode": "gradual",
  "auto_rollback": {
    "enabled": true,
    "error_rate_threshold": 0.05,
    "latency_p99_threshold": "2s",
    "window": "1m",
    "min_samples": 100
  }
}
```

**`GET /api/rollout` response**

```json
{
  "route": "api-v2",
  "state": "in_progress",
  "current_step": 2,
  "steps": [0, 25, 50, 75, 100],
  "green_percent": 50,
  "started_at": "2026-11-12T09:00:00Z",
  "step_started_at": "2026-11-12T09:10:00Z",
  "metrics": {
    "window": { "error_rate": 0.004, "latency_p99_ms": 312, "samples": 4821 }
  },
  "rollback": null
}
```

`state` is one of `idle` | `in_progress` | `paused` | `rolling_back` | `rolled_back` | `completed` | `failed`.

Starting a rollout on a route that already has one running returns `409`.

### DNS

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/dns/zones` | All zones and their records |
| `PUT` | `/api/dns/zones/{zone}` | Replace a zone's records |
| `POST` | `/api/dns/zones/{zone}/records` | Add one record |
| `DELETE` | `/api/dns/zones/{zone}/records/{name}` | Delete one record |

**Writing a DNS record updates the in-memory zone immediately and writes the change back to
`tenant.yaml`** (TC-076). The write is atomic — temp file + `rename` — and triggers a config
reload of the DNS component only.

### Sessions

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/sessions` | Current sticky-session count and eviction stats |
| `DELETE` | `/api/sessions` | Flush the session store |

### Metrics

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/metrics` | Prometheus exposition format. No auth, bound to `127.0.0.1` by default |

---

## 5. Metrics Exposed

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `orchestrator_requests_total` | counter | `route`, `endpoint`, `status` | Total proxied requests |
| `orchestrator_request_duration_seconds` | histogram | `route`, `endpoint` | End-to-end request latency |
| `orchestrator_backend_failures_total` | counter | `endpoint`, `kind` | Backend connect/timeout errors |
| `orchestrator_endpoint_healthy` | gauge | `endpoint` | `1` healthy, `0` unhealthy |
| `orchestrator_vip_owned` | gauge | — | `1` if this worker owns the VIP |
| `orchestrator_sync_peer_up` | gauge | — | `1` if the sync peer is reachable |
| `orchestrator_rollout_green_percent` | gauge | `route` | Current green traffic percentage |
| `orchestrator_rollout_rollbacks_total` | counter | `route`, `reason` | Rollbacks triggered |
| `orchestrator_sessions_active` | gauge | `route` | Entries in the sticky-session store |
| `orchestrator_config_reload_total` | counter | `result` | `success` / `failure` |
| `orchestrator_config_reload_duration_seconds` | histogram | — | Reload latency (must be < 0.1s, G9.6) |

---

## 6. Real-Time Updates (GUI)

The GUI needs live updates for the health dashboard and the rolling-update progress bar.

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/api/events` | `text/event-stream` of server-sent events |
| `GET` | `/api/events/ws` | WebSocket alternative, same event schema |

Event types: `health`, `rollout`, `config_reloaded`, `vip_changed`, `dns_changed`, `metrics`.

Each event carries a monotonically increasing `seq`. On reconnect the client sends
`Last-Event-ID` to resume; the server replays from its ring buffer (last 1000 events).


---


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


---


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

| Area | Target | Gate today | Rationale |
|------|--------|------------|-----------|
| `orchestrator-core` (config, weights, matching, validation) | **95%** | 95% | Pure logic, cheap to cover, highest bug density |
| `orchestrator` control plane and data plane | 80% | 45% | The gate rises as Phases 1–6 land the health checker, the VIP manager, and the REST API |
| `orchestrator-manager` | 70% | 55% | The gate rises as Phase 4 lands the supervisor, the watcher, and the API |
| `orchestrator-testkit` | 70% | 90% | The fixtures are exercised by every other crate's tests |

**The gate is a ratchet, not the target.** CI fails if coverage drops below the *gate*, so it
cannot regress. The *target* is the figure the crate must reach when its phase is complete —
raising the gate is part of that phase's Definition of Done.

`scripts/check-coverage.sh` enforces the gate, and `.github/workflows/ci.yml` runs it after
`cargo llvm-cov`.

---

## 6. Flakiness Policy

- **No test may sleep for real time to satisfy a duration requirement.** Use `FakeClock` or
  `tokio::time::pause()`. The only real sleeps permitted are in system tests that measure genuine
  wall-clock failover (< 1s, G3.4).
- A test that flakes is **disabled and tracked**, never silently retried. Retrying hides bugs.
- The nightly job runs the full suite 10× to surface order-dependence and flakes.


---


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


---


The physical structure of the repository, the CI pipeline, packaging, and the development
environment. This is the contract for where code lives.

---

## 1. Repository Layout

```
orchestrator/
├── Cargo.toml                       # Rust workspace
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml                        # cargo-deny licence/advisory policy
├── package.json                     # GUI workspace root (npm)
├── .nvmrc
├── .editorconfig
├── .gitignore
├── LICENSE
├── README.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
├── Makefile                         # The one entry point for common tasks
│
├── plan/                            # The documentation set (this directory)
│   ├── 01-project-overview.md
│   ├── 02-goals-and-acceptance-criteria.md
│   ├── 03-architecture.md
│   ├── 04-code-standards.md
│   ├── 05-implementation-plan.md
│   ├── 06-tracker.md
│   ├── 07-config-schema.md
│   ├── 08-api-reference.md
│   ├── 09-dependencies.md
│   ├── 10-test-strategy.md
│   ├── 11-non-functional-requirements.md
│   └── 12-repo-layout-and-tooling.md
│
├── crates/
│   ├── orchestrator-core/           # Shared types, config model, validation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs             # ConfigError, VipError, RolloutError, …
│   │       ├── config/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs         # TenantConfig and all sub-types
│   │       │   ├── parse.rs         # YAML → TenantConfig
│   │       │   ├── validate.rs      # All validation rules V1–V17
│   │       │   └── duration.rs      # "30s" / "5m" parsing
│   │       ├── hierarchy.rs         # The 4-level tree, weight composition
│   │       ├── weights.rs           # Weighted selection, effective weights
│   │       ├── routing.rs           # Route matching, longest-prefix-wins
│   │       ├── affinity.rs          # Session key derivation
│   │       ├── rollout.rs           # Rollout model + state machine
│   │       ├── dns.rs               # DNS record types
│   │       ├── vip.rs               # VIP types
│   │       ├── metrics.rs           # Metric name constants
│   │       └── version.rs           # SCHEMA_VERSION, product version
│   │
│   ├── orchestrator/                # Per-tenant binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cli.rs               # Argument parsing
│   │       ├── runtime.rs           # Tokio runtime setup, signal handling
│   │       ├── control_plane/
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs        # Config engine + hot reload
│   │       │   ├── health.rs        # Health checker
│   │       │   ├── vip_manager.rs   # VIP acquire/release via rtnetlink
│   │       │   ├── rolling_update.rs
│   │       │   ├── session_store.rs
│   │       │   └── api.rs           # Tenant REST API
│   │       ├── data_plane/
│   │       │   ├── mod.rs
│   │       │   ├── http.rs          # HTTP/HTTPS proxy
│   │       │   ├── tcp.rs
│   │       │   ├── udp.rs
│   │       │   ├── tls.rs           # TLS acceptor, cert hot-reload
│   │       │   └── headers.rs       # X-Forwarded-*, X-Request-Id
│   │       ├── sync/
│   │       │   ├── mod.rs
│   │       │   ├── channel.rs       # gRPC sync channel
│   │       │   ├── messages.rs      # Sync message types
│   │       │   └── state_machine.rs # Active/passive state machine
│   │       ├── dns_server/
│   │       │   ├── mod.rs
│   │       │   ├── server.rs
│   │       │   └── zone.rs
│   │       ├── vip/
│   │       │   ├── mod.rs
│   │       │   └── netlink.rs       # Linux netlink binding
│   │       └── metrics.rs           # Prometheus metrics
│   │
│   ├── orchestrator-manager/        # Host-level manager binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── registry.rs          # registry.yaml model
│   │       ├── vip_pool.rs
│   │       ├── supervisor.rs        # systemd unit generation + control
│   │       ├── config_watcher.rs    # inotify via notify
│   │       ├── auth.rs              # Argon2 credential store, token issuing
│   │       └── api.rs               # Manager REST API
│   │
│   ├── orchestrator-proto/          # gRPC/protobuf definitions
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   └── proto/
│   │       └── sync.proto
│   │
│   └── orchestrator-testkit/        # Shared test fixtures (dev-dependency)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── mock_backend.rs
│           ├── load_generator.rs
│           ├── test_config.rs
│           ├── fake_clock.rs
│           └── cluster.rs           # netns-based TestCluster (Linux)
│
├── orchestrator-gui/                # React + TypeScript web app
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   ├── e2e/                         # Playwright
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── theme.ts
│       ├── api/
│       │   ├── client.ts            # Fetch wrapper with auth
│       │   ├── types.ts
│       │   └── endpoints.ts
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
│       └── hooks/
│
├── systemd/
│   ├── orchestrator-manager.service
│   └── orchestrator-tenant.service.template
│
├── packaging/
│   ├── deb/
│   │   ├── control
│   │   ├── postinst
│   │   └── postrm
│   ├── rpm/
│   │   └── orchestrator.spec
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
│   ├── tenant.yaml                  # A complete, working example
│   ├── registry.yaml
│   └── hierarchy.yaml
│
├── system-tests/                    # Privileged Linux tests
│   ├── run.sh
│   ├── Cargo.toml
│   └── tests/
│       ├── tenant_isolation.rs
│       └── vip_failover.rs
│
├── scripts/
│   ├── bootstrap.sh                 # One-command dev environment
│   ├── gen-systemd-unit.sh
│   └── check-manifest.sh            # Verifies Cargo.toml ↔ 09-dependencies.md
│
└── .github/
    ├── workflows/
    │   ├── ci.yml
    │   ├── release.yml
    │   └── nightly.yml
    ├── ISSUE_TEMPLATE/
    │   ├── bug_report.md
    │   └── feature_request.md
    ├── PULL_REQUEST_TEMPLATE.md
    └── dependabot.yml
```

---

## 2. Workspace Layout

```toml
[workspace]
resolver = "2"
members = [
    "crates/orchestrator-core",
    "crates/orchestrator",
    "crates/orchestrator-manager",
    "crates/orchestrator-proto",
    "crates/orchestrator-testkit",
]
```

### Dependency Graph

```
                     orchestrator-proto
                       ↙           ↘
        orchestrator-core        (generated pb)
              ↑        ↑
              │        │
      orchestrator   orchestrator-manager

    orchestrator-testkit ──▶ orchestrator-core   (dev-dependency only)
```

Rules:

- `orchestrator-core` depends on **no other** orchestrator crate. It is the shared vocabulary.
- `orchestrator-proto` depends on nothing internal.
- `orchestrator` and `orchestrator-manager` are **siblings** — neither depends on the other.
  The manager talks to tenants over the network (REST/gRPC), not over Rust types.
- No cycles. Enforced by CI with `cargo metadata` + a graph check.

---

## 3. Build Commands

The `Makefile` is the entry point. No one memorises cargo flags.

| Command | Does |
|---------|------|
| `make build` | `cargo build --workspace` |
| `make release` | `cargo build --workspace --release` + GUI build |
| `make test` | `cargo test --workspace` |
| `make test-system` | The privileged Linux system tests |
| `make lint` | `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo deny` + `npm run lint` |
| `make fmt` | `cargo fmt` + `npm run format` |
| `make docs` | `cargo doc --workspace --no-deps` + GUI docs |
| `make deps` | `cargo outdated` + `npm outdated` |
| `make gui` | `npm --prefix orchestrator-gui run dev` |
| `make dev-stack` | `scripts/bootstrap.sh` — a local 2-tenant stack with mock backends |
| `make check` | `lint` + `test` + `build` — what CI runs locally |

---

## 4. CI Pipeline

### `ci.yml`

| Job | Runs on | Steps |
|-----|---------|-------|
| `lint` | ubuntu-latest | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo deny check`, `npm run lint`, `npx tsc --noEmit` |
| `unit` | ubuntu-latest | `cargo test --workspace --lib` — matrix: stable, 1.99, MSRV |
| `integration` | ubuntu-latest | `cargo test --workspace --tests` |
| `component` | ubuntu-latest | `cargo test -p orchestrator --test component` |
| `system` | ubuntu-latest | `sudo ./system-tests/run.sh` — netns + systemd |
| `gui` | ubuntu-latest | `npm run test`, `npm run build` |
| `e2e` | ubuntu-latest | `npm --prefix orchestrator-gui run e2e` |
| `manifest` | ubuntu-latest | `scripts/check-manifest.sh` — no undocumented dependency |
| `coverage` | ubuntu-latest | `cargo llvm-cov` → Codecov, threshold check |
| `build` | ubuntu-latest | Release build, `x86_64` + `aarch64` cross |

### `release.yml`

Triggered by a tag `v*`:

1. Verify the tag matches `Cargo.toml` and `package.json` versions
2. Build release binaries for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`
3. Build the GUI bundle and embed it in the binary with `rust-embed` (the GUI ships inside the
   tenant binary and is served by the manager — no separate web server)
4. Build the DEB and RPM packages
5. Build and push a container image to GHCR
6. Generate a checksum file and the release notes from `CHANGELOG.md`
7. Create the GitHub release with the artifacts attached

### `nightly.yml`

Runs the full suite plus the 10× flake check and the load tests. Publishes a performance
comparison against the previous nightly.

---

## 5. Branching & Release Process

| Branch | Purpose | Merges to |
|--------|---------|-----------|
| `main` | Released code. Always green | — |
| `develop` | Integration branch | `main` on release |
| `feature/<name>` | One feature | `develop` |
| `fix/<name>` | One bug | `develop` |
| `release/<version>` | Release prep | `main` |

**Commit message format** — Conventional Commits:

```
feat(rollout): support manual step advancement
fix(vip): release the VIP before exiting on SIGTERM
docs(api): document the rollout endpoints
```

**Pull request requirements**:

- Linked to an issue or a tracker task
- Tests added for new behaviour
- `make check` passes locally
- The tracker in `06-tracker.md` is updated
- Any new dependency is added to `09-dependencies.md`

---

## 6. Packaging

### DEB

```
orchestrator_1.0.0_amd64.deb
├── /usr/local/bin/orchestrator
├── /usr/local/bin/orchestrator-manager
├── /lib/systemd/system/orchestrator-manager.service
├── /etc/orchestrator/            # registry.yaml, auth.json
└── /usr/share/doc/orchestrator/
```

### RPM

Mirrors the DEB layout with a `.spec` file, plus:

- `%post` creates the `orchestrator` system user and group
- `%post` creates `/etc/orchestrator` with `0700`
- `%preun` stops running tenants on uninstall

### Container

```dockerfile
FROM debian:bookworm-slim
RUN useradd --system --no-create-home --uid 1500 orchestrator
COPY --from=builder /out/orchestrator /usr/local/bin/
USER orchestrator
EXPOSE 8080 8443 9090 9091
ENTRYPOINT ["/usr/local/bin/orchestrator"]
```

The container image is **not** the primary deployment mechanism — systemd units are. The container
exists for CI, for Kubernetes-style environments, and for the manager's own packaging.

---

## 7. Development Environment

### Prerequisites

| Tool | Version |
|------|---------|
| Rust | 1.99 (via `rustup`, pinned by `rust-toolchain.toml`) |
| Node.js | 22 LTS (`nvm use`) |
| npm | 10 (ships with Node 22) |
| Docker | For the local stack |
| `iproute2` | Linux only — for the VIP tests |

### One-command setup

```bash
./scripts/bootstrap.sh
```

This installs the toolchain, fetches dependencies, builds the workspace, generates a self-signed
certificate, writes `examples/tenant.yaml` to `/tmp/`, and starts a local stack with mock
backends.

### The local dev stack

`docker-compose.dev.yml` brings up:

| Service | Purpose |
|---------|---------|
| `mock-backend-a` / `-b` / `-c` | Mock backends returning their hostname, with configurable latency and failure rate |
| `orchestrator` | The tenant binary with the example config, VIP `10.0.0.100` on the compose network |
| `orchestrator-standby` | The second worker of the same tenant |
| `orchestrator-manager` | The manager with the example registry |
| `orchestrator-gui` | Vite dev server with a proxy to the manager |

Because the VIP cannot be added inside an unprivileged container, the dev stack uses a
**loopback alias** (`ip addr add 10.0.0.100/32 dev lo`) inside the container, which requires
`--cap-add NET_ADMIN`. The `system-tests/` suite covers the real thing with network namespaces.

---

## 8. Repository Governance

| File | Purpose |
|------|---------|
| `LICENSE` | **Apache-2.0**. Permissive, patent-granting, compatible with GPLv3 linking, and the default for infrastructure software consumed by both open and closed products |
| `README.md` | What it is, why, quick start, architecture diagram, status |
| `CHANGELOG.md` | Keep a Changelog format, one section per release |
| `CONTRIBUTING.md` | How to build, test, and submit a PR |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 2.1 |
| `SECURITY.md` | How to report a vulnerability, supported versions, disclosure timeline |
| `.github/PULL_REQUEST_TEMPLATE.md` | Checklist based on the Definition of Done |

### Why Apache-2.0

The project is infrastructure that enterprises deploy. Apache-2.0 is the most permissive licence
with an explicit patent grant, so downstream vendors can build commercial products on top without
the copyleft obligations that would block adoption. It matches the Rust ecosystem norm (tokio,
hyper, axum are all MIT/Apache-2.0).

---

## 9. Required Repository Settings

| Setting | Value | Why |
|---------|-------|-----|
| Default branch | `main` | |
| Merge commits | Allowed, squash preferred | |
| Branch protection on `main` | Required checks: `lint`, `unit`, `integration`, `component`, `manifest` | |
| Issue templates | Bug report, feature request | |
| Dependabot | Cargo + npm, weekly | |
| Secret scanning | Enabled | |
| Push protection | Enabled | |
| Discussions | Enabled | |
| Wiki | Disabled — `docs/` is the source of truth | |

---

## 10. Documentation Structure

`docs/` is user-facing and is written for an operator, not a developer.

| Document | Audience | Contents |
|----------|----------|----------|
| `docs/getting-started.md` | New user | Install, first tenant, first route, verify it works |
| `docs/configuration.md` | Operator | Every config field, with a full annotated example |
| `docs/architecture.md` | Developer | How the pieces fit together, with diagrams |
| `docs/api-reference.md` | Integrator | The REST API, mirroring `08-api-reference.md` |
| `docs/deployment.md` | Operator | systemd install, TLS certs, network requirements, firewall rules |
| `docs/operations.md` | Operator | Day-2: health checks, rollouts, DNS changes, scaling, backups |
| `docs/troubleshooting.md` | Operator | Symptoms → diagnosis → fix |
| `docs/architecture.md` | Developer | Component design, data flow, extension points |

The `plan/` directory is the **internal** design record. It is committed to the repository because
the design rationale and the acceptance criteria are part of the project's value, but it is not
part of the published documentation site.


---

