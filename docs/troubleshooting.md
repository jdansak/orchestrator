# Troubleshooting

| Symptom | Likely cause | Fix |
|---------|--------------|-----|
| **The config is rejected on start-up** | A validation rule failed | Run `orchestrator --config <path> --check`. Every problem is listed at once |
| **The config is rejected on reload** | Same | The old config is still serving. Read the log line — it lists every problem |
| **`unknown field` in the error** | A typo, or a field from a newer schema | Fix the typo, or check the schema version |
| **`sibling weights sum to 90, expected 100`** | The weights do not add up at that level | Make them sum to 100, or make the level have a single child |
| **`the VIP must be a host address`** | The prefix is not `/32` or `/128` | Use `10.0.0.100/32` |
| **Traffic gets a `404`** | No route matched the path | Check the route's `match.path` and `match.type`. The longest prefix wins |
| **Traffic gets a `503`** | Every endpoint of the route is unhealthy | Check `orchestrator_endpoint_healthy` in the metrics. The backend is probably down |
| **Traffic gets a `502`** | The backend could not be reached | Check the endpoint address and the network route |
| **Traffic gets a `504`** | The backend did not respond in time | Raise `upstream.response_timeout`, or fix the backend |
| **The split is not what the config says** | The weights compose across every level | Run `orchestrator --print-routes` to see the **effective** weights |
| **The VIP is on both workers** | The standby has not yet noticed the active is gone | Check `worker.sync.peer_timeout` and the sync channel |
| **The failover took longer than a second** | The peer timeout is too long | Lower `worker.sync.peer_timeout` — it must be under 1s |
| **A session is not sticky** | The cookie is not being sent, or the method is `none` | Check `affinity.method` and `affinity.cookie_name`. The cookie must be returned by the client |
| **The DNS server does not answer** | DNS is not enabled, or there is no upstream | `dns.enabled` must be `true` and `dns.upstream` must not be empty |
| **The DNS server refuses to answer** | The zone does not match the query | The tenant is authoritative for `tenant.zone` only |
| **`AXFR` is refused** | By design | Orchestrator does not serve zone transfers |
| **The metrics port is already in use** | Two tenants on the host share a port | The manager detects this and refuses to start. Allocate a different port |
| **The manager refuses to register a tenant** | A port conflict on that host | Change the port in the registry entry |
| **A tenant will not start** | Its config is missing or invalid | `journalctl -u orchestrator-<tenant>` |
| **The tenant is `config-missing`** | The manager started it before the config existed | Create the config and reload |

## Getting more information

```bash
# Everything the tenant logged
journalctl -u orchestrator-app-frontend -f

# The effective routing table
sudo orchestrator --config /etc/orchestrator/tenants/app-frontend/tenant.yaml --print-routes

# The metrics
curl -s http://127.0.0.1:9090/metrics | grep orchestrator_

# The worker's role and VIP ownership
curl -s http://127.0.0.1:9090/api/worker | jq
```

## Reporting a bug

Include the version, the config with secrets removed, and the relevant logs. See
[`CONTRIBUTING.md`](../CONTRIBUTING.md).
