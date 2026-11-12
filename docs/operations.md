# Operations

Day-2 tasks for running Orchestrator.

---

## Health

| Check | Command |
|-------|---------|
| Is the tenant running? | `systemctl status orchestrator-<tenant>` |
| Does the tenant own its VIP? | `ip addr show dev <iface> \| grep <vip>` |
| Is the config valid? | `orchestrator --config <path> --check` |
| Are the endpoints healthy? | `curl http://127.0.0.1:9090/metrics \| grep endpoint_healthy` |
| What is the routing table? | `orchestrator --config <path> --print-routes` |

---

## Rolling updates

A blue-green rollout shifts traffic from `blue` to `green` in steps. If the error rate or the
p99 latency exceeds the threshold, the traffic returns to `blue` automatically.

| Action | API |
|--------|-----|
| Start | `POST /api/tenants/:name/rollout` |
| Advance one step | `POST /api/tenants/:name/rollout/advance` |
| Skip to the end | `POST /api/tenants/:name/rollout/promote` |
| Roll back now | `POST /api/tenants/:name/rollout/rollback` |
| Pause | `POST /api/tenants/:name/rollout/pause` |
| Resume | `POST /api/tenants/:name/rollout/resume` |

### The `min_samples` guard

Auto-rollback is not evaluated until the window holds `min_samples` requests. Without it, a
single failed request during a quiet period would roll back a healthy release.

### What a rollback does

1. The green target is taken out of the routing table **immediately**
2. All traffic returns to blue
3. A `rollback` notification is sent to the configured webhook
4. The rollout state becomes `rolled_back`

---

## DNS

| Action | API |
|--------|-----|
| List the zones | `GET /api/tenants/:name/dns/zones` |
| Add a record | `POST /api/tenants/:name/dns/zones/:zone/records` |
| Delete a record | `DELETE /api/tenants/:name/dns/zones/:zone/records/:name` |

Writing a record through the API updates the in-memory zone immediately **and** writes the
change back to `tenant.yaml` atomically. The file is never left half-written.

---

## Scaling

### Adding a tenant

```bash
curl -X POST http://manager:8081/api/tenants \
  -H "Authorization: Bearer $TOKEN" \
  -H 'content-type: application/json' \
  -d '{
    "name": "payments",
    "vip": "10.0.0.101/32",
    "interface": "eth0",
    "workers": [
      { "node_id": "worker-a", "host": "10.0.2.10",
        "ports": { "http": 18090, "https": 18453, "metrics": 19100, "sync": 19101, "dns": 15363 } }
    ]
  }'
```

The manager refuses a port conflict **before** writing anything. Two tenants on the same host
cannot bind the same port.

### Limits

| Limit | Value |
|-------|-------|
| Tenants per host | 50 |
| Routes per tenant | 1,000 |
| Endpoints per tenant | 10,000 |
| Concurrent connections per tenant | 65,535 |
| Sticky sessions per tenant | 100,000 (LRU) |
| Config file size | 4 MiB |
| DNS records per zone | 10,000 |

---

## Upgrading

Orchestrator is designed to be upgraded in place.

1. Install the new package. The binaries are replaced; the configuration is untouched.
2. `orchestrator --check` the config against the new binary. If the schema version changed, the
   `orchestrator migrate` subcommand converts it:
   ```bash
   sudo orchestrator migrate --from 1 --to 2 --in tenant.yaml --out tenant.v2.yaml
   ```
3. Restart the tenants one at a time. The VIP moves to the standby before the old active
   exits, so there is no gap in service.

**Never** upgrade more than one tenant at a time on a shared VIP.
