# Configuration

Orchestrator is configured with YAML. There are two files:

| File | Read by | Purpose |
|------|----------|---------|
| `/etc/orchestrator/tenants/<tenant>/tenant.yaml` | The tenant process | Everything about one tenant |
| `/etc/orchestrator/registry.yaml` | The manager | Which tenants exist on this host |

The **normative** schema — every field, every default, every validation rule — is
[`plan/07-config-schema.md`](../plan/07-config-schema.md). This page is the operator's view.

---

## The rules that bite

### Unknown fields are rejected

A typo fails loudly instead of silently doing nothing.

```yaml
connnect_timeout: 5s     # rejected: unknown field
```

### Durations are strings with a unit

`30s`, `5m`, `1h30m`, `500ms`. A bare number is rejected — `timeout: 30` is ambiguous, and
guessing is how a 30-second timeout ends up meaning 30 milliseconds.

### Weights sum to 100 at every level

```yaml
hierarchy:
  regions:
    - name: us-east
      weight: 60
    - name: eu-west
      weight: 40     # 60 + 40 = 100 ✓
```

A lone child takes its parent's whole share, whatever weight it declares.

### The VIP must be a host address

`10.0.0.100/32` for IPv4, `2001:db8::1/128` for IPv6. A `/24` is rejected.

### TLS 1.0 and 1.1 are never negotiable

`min_version` accepts `"1.2"` and `"1.3"`. Anything lower is rejected at validation time.

### No secrets in the config

Certificates are referenced by path. Passwords live in a separate credential file. Nothing in
`tenant.yaml` or `registry.yaml` is a secret.

---

## A complete example

```yaml
version: 1

tenant:
  name: app-frontend
  zone: app-frontend.internal
  description: "Public frontend for the app"

vip:
  address: 10.0.0.100/32
  interface: eth0

listeners:
  http:
    bind: 0.0.0.0:8080
  https:
    bind: 0.0.0.0:8443
    tls:
      cert: /etc/orchestrator/tenants/app-frontend/ssl/fullchain.pem
      key: /etc/orchestrator/tenants/app-frontend/ssl/privkey.pem
      client_auth: none
      min_version: "1.2"
      alpn: ["h2", "http/1.1"]

upstream:
  connect_timeout: 2s
  response_timeout: 30s
  idle_timeout: 60s
  max_connections_per_endpoint: 1024
  retries: 1
  health_check:
    type: http
    path: /healthz
    interval: 10s
    timeout: 2s
    healthy_threshold: 2
    unhealthy_threshold: 3

hierarchy:
  regions:
    - name: us-east
      weight: 60
      datacenters:
        - name: dc-1
          weight: 100
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

routes:
  - name: api-v2
    enabled: true
    match:
      path: /api/v2
      type: prefix
    scope:
      region: "*"
    affinity:
      method: cookie
      cookie_name: ORCHSID
      ttl: 30m

rollout:
  route: api-v2
  strategy: blue-green
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
  mode: gradual
  auto_rollback:
    enabled: true
    error_rate_threshold: 0.05
    latency_p99_threshold: 2s
    window: 1m
    min_samples: 100
  notify:
    webhook: https://hooks.internal/orchestrator
    events: [rollback, complete, failed]

dns:
  enabled: true
  listen: 0.0.0.0:53
  upstream: [1.1.1.1, 8.8.8.8]
  ttl: 60
  records:
    - name: api
      type: A
      value: 10.0.0.100
      ttl: 30

metrics:
  enabled: true
  bind: 127.0.0.1:9090

logging:
  level: info
  format: json

worker:
  mode: active-passive
  node_id: worker-a
  sync:
    listen: 0.0.0.0:9091
    peer: 10.0.2.11:9091
    heartbeat_interval: 300ms
    peer_timeout: 1s
```

---

## Changing the configuration

```bash
# Validate before applying
sudo orchestrator --config /etc/orchestrator/tenants/app-frontend/tenant.yaml --check

# Apply
sudo systemctl reload orchestrator-app-frontend
```

A reload that fails validation is **rejected**. The previous configuration keeps serving and
the error is logged with every problem listed.

The `version` field is the **config schema version**, not the product version. A config written
for a newer schema is refused outright rather than partially applied.
