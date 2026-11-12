# Orchestrator — API Reference

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
