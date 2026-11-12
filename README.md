# Orchestrator

**An open-source, application-level traffic manager for Kubernetes clusters and VM pools.**

Orchestrator routes traffic across regions, data centres, clusters, and endpoints with
configurable weights. It manages HA worker pairs with Linux kernel VIP failover, handles
blue-green and canary rollouts with automatic rollback on error thresholds, serves authoritative
DNS for tenant zones, and terminates TLS at the VIP.

It is a standalone traffic manager. There is no Kubernetes dependency, no control plane to
operate, and no hardware appliance. It runs on commodity Linux.

---

## Why not an existing load balancer?

| F5 / commercial appliances | Orchestrator |
|---------------------------|-------------|
| Hardware appliance, expensive | Commodity hardware, open source |
| Proprietary config language | YAML — human-readable, version-controllable |
| Heavy centralised management | A lightweight YAML file and a web GUI |
| L4/L7 proxy only | L4/L7 proxy **and** DNS-level routing |
| Monolithic binary | Per-tenant process isolation |
| Days to weeks to provision | A tenant in seconds |
| Vendor lock-in | Open source, community-driven |
| Complex automation | GitOps-friendly, hot-reloadable |

---

## Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                       Host (Linux)                            │
│                                                               │
│  orchestrator-manager (systemd)                               │
│  ├── Orchestrates tenant processes                            │
│  ├── Manages the VIP pool                                     │
│  └── Watches config changes                                   │
│                                                               │
│  orchestrator-app-frontend (systemd) ── sync ── standby      │
│  ├── Control Plane: config engine, health, VIP, rollouts      │
│  ├── Data Plane: HTTP/HTTPS proxy, TCP proxy, UDP proxy       │
│  ├── DNS Server (hickory-server)                              │
│  └── Sync Peer (gRPC)                                         │
│                                                               │
│  orchestrator-gui — React + TypeScript, served by the manager │
└──────────────────────────────────────────────────────────────┘
```

Every tenant runs as its own systemd service with its own configuration, VIP, listeners, and
resource limits. A crash or a misconfiguration in one tenant cannot affect any other.

---

## Status

**Pre-alpha.** The design is complete and the foundation is in place; the feature phases are
tracked in [`plan/05-implementation-plan.md`](plan/05-implementation-plan.md).

| Phase | Status |
|-------|--------|
| 0. Foundation — workspace, CI, test harness | ✅ |
| 1. Core engine — config, validation, routing, affinity | 🔨 in progress |
| 2. HA and VIP | ⬜ |
| 3. Rolling updates | ⬜ |
| 4. Multi-tenant and manager | ⬜ |
| 5. DNS server | ⬜ |
| 6. TLS and TCP/UDP | ⬜ |
| 7. GUI | ⬜ |
| 8. Security and hardening | ⬜ |
| 9. Release, packaging, operations | ⬜ |

**Do not deploy this to production yet.**

---

## Quick start

### Prerequisites

| Tool | Version |
|------|---------|
| Rust | 1.99 stable — `rustup toolchain install 1.99` |
| Node.js | 22 LTS (only for the GUI) |
| Linux | kernel 5.10+ (only for the VIP and the system tests) |

### Build

```bash
git clone https://github.com/Jdansak/orchestrator.git
cd orchestrator
make build
```

### Validate a configuration

```bash
cargo run -p orchestrator -- --config examples/tenant.yaml --check
```

### See the resolved routing table

```bash
cargo run -p orchestrator -- --config examples/tenant.yaml --print-routes
```

```
ENDPOINT                                  ADDRESS                 WEIGHT
us-east/dc-1/prod/ep-a                    10.1.1.5:8080            36.0000%
us-east/dc-1/prod/ep-b                    10.1.1.6:8080            24.0000%
eu-west/dc-2/prod/ep-c                    10.2.1.5:8080            40.0000%
```

### Run

```bash
cargo run -p orchestrator -- --config examples/tenant.yaml --http-bind 127.0.0.1:8080
```

### Test

```bash
make test          # unit and integration tests
make lint          # fmt, clippy, cargo-deny, and the GUI linters
make check         # everything CI runs
```

---

## Configuration

A tenant is described by one YAML file. The full schema is in
[`plan/07-config-schema.md`](plan/07-config-schema.md).

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
    match:
      path: /api/v2
      type: prefix
    affinity:
      method: cookie
      cookie_name: ORCHSID
      ttl: 30m
```

Weights at every level must sum to 100. A lone child takes its parent's whole share. Changes are
hot-reloaded — send `SIGHUP`, and the routing table is swapped atomically within 100 ms without
dropping a connection.

---

## Documentation

| Document | Purpose |
|----------|---------|
| [`plan/01-project-overview.md`](plan/01-project-overview.md) | The problem and the approach |
| [`plan/02-goals-and-acceptance-criteria.md`](plan/02-goals-and-acceptance-criteria.md) | 11 goals, their requirements, and their test cases |
| [`plan/03-architecture.md`](plan/03-architecture.md) | How it works |
| [`plan/04-code-standards.md`](plan/04-code-standards.md) | How to write the code |
| [`plan/05-implementation-plan.md`](plan/05-implementation-plan.md) | The build order |
| [`plan/06-tracker.md`](plan/06-tracker.md) | What is done and what is next |
| [`plan/07-config-schema.md`](plan/07-config-schema.md) | The configuration schema |
| [`plan/08-api-reference.md`](plan/08-api-reference.md) | The REST APIs |
| [`plan/09-dependencies.md`](plan/09-dependencies.md) | Every dependency, with versions |
| [`plan/10-test-strategy.md`](plan/10-test-strategy.md) | How the goals are proven |
| [`plan/11-non-functional-requirements.md`](plan/11-non-functional-requirements.md) | Performance, security, and the Definition of Done |
| [`plan/12-repo-layout-and-tooling.md`](plan/12-repo-layout-and-tooling.md) | Repository layout, CI, and packaging |

---

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). The short version:

1. Read `plan/04-code-standards.md` and `plan/07-config-schema.md`
2. Open an issue before starting anything larger than a bug fix
3. Keep `make check` green
4. Add a test for every behaviour you add — a criterion with no test is not done
5. Update the tracker in `plan/06-tracker.md`

## Licence

Apache-2.0. See [`LICENSE`](LICENSE).

## Security

Report vulnerabilities privately — see [`SECURITY.md`](SECURITY.md). Do not open a public issue.
