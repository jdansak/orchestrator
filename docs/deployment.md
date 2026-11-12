# Deployment

How Orchestrator is installed and run on a host.

---

## Requirements

| | |
|---|---|
| OS | Linux, kernel 5.10 or newer |
| Init | systemd 245 or newer |
| Privileges | `CAP_NET_ADMIN` for the VIP. Everything else runs as the unprivileged `orchestrator` user |
| DNS | The host must be able to bind UDP 53, or the tenant's `dns.listen` must use a different port with a local redirect |

---

## Install

```bash
sudo dpkg -i orchestrator_0.1.0_amd64.deb
# or
sudo rpm -i orchestrator-0.1.0.x86_64.rpm
```

The package creates:

| Path | Owner | Mode | Purpose |
|------|-------|------|---------|
| `/usr/local/bin/orchestrator` | root | 0755 | The tenant binary |
| `/usr/local/bin/orchestrator-manager` | root | 0755 | The manager binary |
| `/lib/systemd/system/orchestrator-manager.service` | root | 0644 | The manager unit |
| `/etc/orchestrator/` | `orchestrator` | **0700** | The registry and the credentials |
| `/etc/orchestrator/tenants/` | `orchestrator` | 0750 | The tenant configs |
| `/var/lib/orchestrator/` | `orchestrator` | 0750 | The runtime state |

> `/etc/orchestrator` is **0700** because it holds the credential file. A world-readable config
> directory is a vulnerability, not a convenience.

---

## The `orchestrator` user

The binaries run as an unprivileged `orchestrator` user. The tenant unit grants exactly one
capability:

```ini
CapabilityBoundingSet=CAP_NET_ADMIN
AmbientCapabilities=CAP_NET_ADMIN
```

That is what the VIP needs and nothing more. If your platform does not support ambient
capabilities, run the tenant with `User=root` and rely on the other hardening directives —
but prefer the capability.

---

## Network requirements

| Port | Direction | Purpose |
|------|-----------|---------|
| 8080 | Inbound | The plaintext proxy listener |
| 8443 | Inbound | The TLS proxy listener |
| 9090 | Local | The Prometheus exporter. **Loopback only** |
| 9091 | Peer | The gRPC sync channel between the two workers |
| 53 | Inbound | The DNS server |
| The VIP | Inbound | The address the tenant owns |

The VIP must be routable to the clients that use it. It is an address on the host's interface —
there is no NAT and no port mapping.

---

## Firewall

```bash
# The proxy listeners and DNS
sudo firewall-cmd --permanent --add-port=8080/tcp
sudo firewall-cmd --permanent --add-port=8443/tcp
sudo firewall-cmd --permanent --add-port=53/udp
sudo firewall-cmd --permanent --add-port=53/tcp
sudo firewall-cmd --reload

# The metrics endpoint must NOT be exposed
```

Do not open port 9090. It is bound to `127.0.0.1` by default, and a Prometheus scraper on the
same host reaches it there.

---

## The worker pair

Two workers on two hosts, sharing one VIP:

```yaml
# /etc/orchestrator/registry.yaml
tenants:
  - name: app-frontend
    config: /etc/orchestrator/tenants/app-frontend/tenant.yaml
    vip: 10.0.0.100/32
    interface: eth0
    workers:
      - node_id: worker-a
        host: 10.0.2.10
        ports: { http: 18080, https: 18443, metrics: 19090, sync: 19091, dns: 15353 }
      - node_id: worker-b
        host: 10.0.2.11
        ports: { http: 18080, https: 18443, metrics: 19090, sync: 19091, dns: 15353 }
```

Each worker gets the same `tenant.yaml` but a different `node_id`. The active worker owns the
VIP; the standby takes over within a second if it disappears.

---

## Upgrading

1. Install the new package — the binaries are replaced, the configuration is untouched.
2. Validate: `orchestrator --config <path> --check`.
3. Restart the tenants **one at a time**. The VIP moves to the standby before the old active
   exits, so there is no gap in service.

```bash
sudo systemctl restart orchestrator-app-frontend
# wait for it to settle before moving to the next tenant
sudo systemctl restart orchestrator-payments
```

> Never upgrade two tenants that share a VIP at the same time.

---

## Uninstalling

```bash
sudo systemctl disable --now orchestrator-manager
sudo apt-get remove orchestrator
```

The configuration is **kept**. Removing it would silently break every tenant the operator has
configured. Use `apt-get purge` to remove the runtime state as well.
