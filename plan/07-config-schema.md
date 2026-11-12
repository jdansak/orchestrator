# Orchestrator — Configuration Schema

Normative specification of `tenant.yaml` and `registry.yaml`. Every field used by any other
document must appear here. Anything not listed here is rejected by the validator with
`unknown field` errors.

---

## 1. Global Rules

| Rule | Value |
|------|-------|
| Format | YAML 1.2, parsed with `serde_yaml_ng` |
| Root document | Single mapping (no multi-doc) |
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
