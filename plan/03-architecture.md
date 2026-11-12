# Orchestrator — Architecture

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
