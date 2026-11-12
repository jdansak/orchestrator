# Getting Started

This guide takes you from a clean machine to a running tenant that routes traffic.

---

## 1. What you need

| | |
|---|---|
| **OS** | Linux (kernel 5.10 or newer) for the real thing. macOS and Windows work for development. |
| **RAM** | 1 GB per tenant is plenty to start |
| **Ports** | 8080 (HTTP), 8443 (HTTPS), 9090 (metrics), 9091 (worker sync), 53 (DNS) — all configurable |
| **Privileges** | `CAP_NET_ADMIN` for the VIP. Everything else runs unprivileged. |

You do **not** need Kubernetes. Orchestrator routes to any address that speaks HTTP.

---

## 2. Install

### From a package

```bash
sudo dpkg -i orchestrator_0.1.0_amd64.deb     # Debian / Ubuntu
sudo rpm -i orchestrator-0.1.0.x86_64.rpm     # RHEL / Fedora
```

### From source

```bash
git clone https://github.com/Jdansak/orchestrator.git
cd orchestrator
make build
sudo make install
```

---

## 3. Create a tenant

A tenant is a directory under `/etc/orchestrator/tenants/` with a `tenant.yaml` and a `ssl/`
directory.

```bash
sudo mkdir -p /etc/orchestrator/tenants/app-frontend/ssl
sudo cp examples/tenant.yaml /etc/orchestrator/tenants/app-frontend/tenant.yaml
```

Edit it. The minimum a tenant needs is a name, a zone, a VIP, and one endpoint:

```yaml
version: 1

tenant:
  name: app-frontend
  zone: app-frontend.internal

vip:
  address: 10.0.0.100/32
  interface: eth0

hierarchy:
  regions:
    - name: us-east
      weight: 100
      datacenters:
        - name: dc-1
          weight: 100
          clusters:
            - name: prod
              weight: 100
              endpoints:
                - name: ep-a
                  weight: 100
                  address: 127.0.0.1:3000

routes:
  - name: all
    match:
      path: /
      type: prefix
```

### Check it before you start it

```bash
sudo orchestrator --config /etc/orchestrator/tenants/app-frontend/tenant.yaml --check
```

```
{"level":"INFO","message":"configuration is valid"}
```

The validator reports **every** problem at once, so you do not fix them one at a time:

```
configuration is invalid:
3 validation errors:
  - hierarchy.regions[0].weight: sibling weights sum to 90, expected 100
  - routes[1].match.path: path 'api' must start with '/'
  - rollout.green.cluster: unknown cluster 'prod-green' in datacenter 'dc-1'
```

### See the resolved routing table

```bash
sudo orchestrator --config /etc/orchestrator/tenants/app-frontend/tenant.yaml --print-routes
```

```
ENDPOINT                                 ADDRESS                      WEIGHT
us-east/dc-1/prod/ep-a                   10.1.1.5:8080              36.0000%
us-east/dc-1/prod/ep-b                   10.1.1.6:8080              24.0000%
eu-west/dc-2/prod/ep-c                   10.2.1.5:8080              40.0000%
```

The weights are **effective** weights — the result of composing every level of the hierarchy.

---

## 4. Run it

### By hand

```bash
sudo orchestrator --config /etc/orchestrator/tenants/app-frontend/tenant.yaml
```

### As a systemd service

The manager generates the unit from the registry:

```bash
sudo cp examples/registry.yaml /etc/orchestrator/registry.yaml
sudo systemctl enable --now orchestrator-manager
```

The manager starts, stops, and reloads the tenants listed in the registry.

---

## 5. Verify it works

```bash
# The VIP is on the interface
ip addr show dev eth0 | grep 10.0.0.100

# Traffic is routed
curl -H 'Host: app-frontend.internal' http://10.0.0.100/

# The metrics endpoint is live
curl http://127.0.0.1:9090/metrics | head
```

---

## 6. Change the configuration

Edit `tenant.yaml` and reload:

```bash
sudo systemctl reload orchestrator-app-frontend
```

The new routing table is swapped in atomically within 100 ms. **Connections that are already
open keep running against the old table until they finish.** If the new config is invalid, the
reload is rejected, the error is logged, and the old configuration keeps serving.

---

## 7. TLS

```bash
sudo cp fullchain.pem /etc/orchestrator/tenants/app-frontend/ssl/
sudo cp privkey.pem   /etc/orchestrator/tenants/app-frontend/ssl/
sudo chmod 0600 /etc/orchestrator/tenants/app-frontend/ssl/privkey.pem
```

Point the config at them:

```yaml
listeners:
  https:
    bind: 0.0.0.0:8443
    tls:
      cert: /etc/orchestrator/tenants/app-frontend/ssl/fullchain.pem
      key: /etc/orchestrator/tenants/app-frontend/ssl/privkey.pem
      min_version: "1.2"
      alpn: ["h2", "http/1.1"]
```

TLS 1.0 and 1.1 are never negotiable. A config asking for a lower minimum is rejected.

---

## 8. Next

| I want to… | Read |
|-------------|------|
| Understand every config field | [`plan/07-config-schema.md`](../plan/07-config-schema.md) |
| Run an HA pair | `plan/03-architecture.md` — "Sync Channel" |
| Do a blue-green rollout | `plan/08-api-reference.md` — "Rolling Updates" |
| Serve DNS for my zone | `plan/07-config-schema.md` — "DNS" |
| Automate it | `plan/08-api-reference.md` |
