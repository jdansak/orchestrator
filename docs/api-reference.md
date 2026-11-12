# API reference

The **normative** API contract is
[`plan/08-api-reference.md`](../plan/08-api-reference.md). This page is the quick reference.

---

## Authentication

```bash
TOKEN=$(curl -s -X POST http://manager:8081/api/login \
  -H 'content-type: application/json' \
  -d '{"username":"admin","password":"..."}' | jq -r .token)

curl -H "Authorization: Bearer $TOKEN" http://manager:8081/api/tenants
```

Every endpoint requires the token except `/api/login`, `/api/health`, and `/api/version`.

Tokens are opaque, expire after 12 hours, and are refreshed on each authenticated request.

---

## Manager API — `http://<manager>:8081`

### Tenants

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/tenants` | List the tenants |
| `POST` | `/api/tenants` | Register a tenant |
| `GET` | `/api/tenants/:name` | One tenant |
| `PATCH` | `/api/tenants/:name` | Update the registry entry |
| `DELETE` | `/api/tenants/:name` | Deregister the tenant |
| `GET` | `/api/tenants/:name/health` | The tenant's health |
| `POST` | `/api/tenants/:name/start` | Start it |
| `POST` | `/api/tenants/:name/stop` | Stop it |
| `POST` | `/api/tenants/:name/restart` | Restart it |
| `POST` | `/api/tenants/:name/reload` | Reload its configuration |
| `GET` | `/api/tenants/:name/metrics` | Its metrics |

### System

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/health` | Liveness. Public |
| `GET` | `/api/version` | The build version. Public |
| `GET` | `/api/events` | Server-sent events |

---

## Tenant API — `http://<tenant>:<port>`

| Method | Path | Purpose |
|--------|------|---------|
| `GET` | `/api/config` | The active configuration |
| `GET` | `/api/routes` | The routing table |
| `GET` | `/api/routes/:name` | One route |
| `GET` | `/api/hierarchy` | The hierarchy tree |
| `GET` | `/api/hierarchy/effective-weights` | The composed weights |
| `GET` | `/api/endpoints` | Per-endpoint health |
| `GET` | `/api/worker` | This worker's role and VIP state |
| `GET` | `/api/rollout` | The current rollout |
| `POST` | `/api/rollout` | Start a rollout |
| `POST` | `/api/rollout/advance` | Advance one step |
| `POST` | `/api/rollout/rollback` | Roll back now |
| `POST` | `/api/rollout/promote` | Skip to 100% green |
| `POST` | `/api/rollout/pause` | Pause |
| `POST` | `/api/rollout/resume` | Resume |
| `GET` | `/api/dns/zones` | The DNS zones |
| `PUT` | `/api/dns/zones/:zone` | Replace a zone's records |
| `POST` | `/api/dns/zones/:zone/records` | Add a record |
| `DELETE` | `/api/dns/zones/:zone/records/:name` | Delete a record |
| `GET` | `/api/sessions` | The sticky-session stats |
| `DELETE` | `/api/sessions` | Flush the session store |
| `GET` | `/api/events` | Server-sent events |
| `GET` | `/api/health` | Liveness. Public |

---

## Errors

RFC 9457 problem details:

```json
{
  "type": "https://orchestrator.dev/errors/validation-failed",
  "title": "Validation failed",
  "status": 422,
  "detail": "config validation produced 2 errors",
  "errors": [
    { "field": "hierarchy.regions[0].weight", "message": "sibling weights sum to 90, expected 100" }
  ]
}
```

| Code | Meaning |
|------|---------|
| `400` | Malformed request |
| `401` | Missing or invalid token |
| `404` | Not found |
| `409` | Conflicting state — a rollout is already running, or a port conflict |
| `422` | Semantically invalid |
| `429` | Rate limited |
| `500` | Internal error. The details are logged, never returned |
| `503` | Not ready — no healthy endpoints |
