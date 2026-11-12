# Orchestrator — Repository Layout & Tooling

The physical structure of the repository, the CI pipeline, packaging, and the development
environment. This is the contract for where code lives.

---

## 1. Repository Layout

```
orchestrator/
├── Cargo.toml                       # Rust workspace
├── Cargo.lock
├── rust-toolchain.toml
├── rustfmt.toml
├── clippy.toml
├── deny.toml                        # cargo-deny licence/advisory policy
├── package.json                     # GUI workspace root (npm)
├── .nvmrc
├── .editorconfig
├── .gitignore
├── LICENSE
├── README.md
├── CHANGELOG.md
├── CONTRIBUTING.md
├── CODE_OF_CONDUCT.md
├── SECURITY.md
├── Makefile                         # The one entry point for common tasks
│
├── plan/                            # The documentation set (this directory)
│   ├── 01-project-overview.md
│   ├── 02-goals-and-acceptance-criteria.md
│   ├── 03-architecture.md
│   ├── 04-code-standards.md
│   ├── 05-implementation-plan.md
│   ├── 06-tracker.md
│   ├── 07-config-schema.md
│   ├── 08-api-reference.md
│   ├── 09-dependencies.md
│   ├── 10-test-strategy.md
│   ├── 11-non-functional-requirements.md
│   └── 12-repo-layout-and-tooling.md
│
├── crates/
│   ├── orchestrator-core/           # Shared types, config model, validation
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs             # ConfigError, VipError, RolloutError, …
│   │       ├── config/
│   │       │   ├── mod.rs
│   │       │   ├── model.rs         # TenantConfig and all sub-types
│   │       │   ├── parse.rs         # YAML → TenantConfig
│   │       │   ├── validate.rs      # All validation rules V1–V17
│   │       │   └── duration.rs      # "30s" / "5m" parsing
│   │       ├── hierarchy.rs         # The 4-level tree, weight composition
│   │       ├── weights.rs           # Weighted selection, effective weights
│   │       ├── routing.rs           # Route matching, longest-prefix-wins
│   │       ├── affinity.rs          # Session key derivation
│   │       ├── rollout.rs           # Rollout model + state machine
│   │       ├── dns.rs               # DNS record types
│   │       ├── vip.rs               # VIP types
│   │       ├── metrics.rs           # Metric name constants
│   │       └── version.rs           # SCHEMA_VERSION, product version
│   │
│   ├── orchestrator/                # Per-tenant binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── cli.rs               # Argument parsing
│   │       ├── runtime.rs           # Tokio runtime setup, signal handling
│   │       ├── control_plane/
│   │       │   ├── mod.rs
│   │       │   ├── engine.rs        # Config engine + hot reload
│   │       │   ├── health.rs        # Health checker
│   │       │   ├── vip_manager.rs   # VIP acquire/release via rtnetlink
│   │       │   ├── rolling_update.rs
│   │       │   ├── session_store.rs
│   │       │   └── api.rs           # Tenant REST API
│   │       ├── data_plane/
│   │       │   ├── mod.rs
│   │       │   ├── http.rs          # HTTP/HTTPS proxy
│   │       │   ├── tcp.rs
│   │       │   ├── udp.rs
│   │       │   ├── tls.rs           # TLS acceptor, cert hot-reload
│   │       │   └── headers.rs       # X-Forwarded-*, X-Request-Id
│   │       ├── sync/
│   │       │   ├── mod.rs
│   │       │   ├── channel.rs       # gRPC sync channel
│   │       │   ├── messages.rs      # Sync message types
│   │       │   └── state_machine.rs # Active/passive state machine
│   │       ├── dns_server/
│   │       │   ├── mod.rs
│   │       │   ├── server.rs
│   │       │   └── zone.rs
│   │       ├── vip/
│   │       │   ├── mod.rs
│   │       │   └── netlink.rs       # Linux netlink binding
│   │       └── metrics.rs           # Prometheus metrics
│   │
│   ├── orchestrator-manager/        # Host-level manager binary
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs
│   │       ├── registry.rs          # registry.yaml model
│   │       ├── vip_pool.rs
│   │       ├── supervisor.rs        # systemd unit generation + control
│   │       ├── config_watcher.rs    # inotify via notify
│   │       ├── auth.rs              # Argon2 credential store, token issuing
│   │       └── api.rs               # Manager REST API
│   │
│   ├── orchestrator-proto/          # gRPC/protobuf definitions
│   │   ├── Cargo.toml
│   │   ├── build.rs
│   │   └── proto/
│   │       └── sync.proto
│   │
│   └── orchestrator-testkit/        # Shared test fixtures (dev-dependency)
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs
│           ├── mock_backend.rs
│           ├── load_generator.rs
│           ├── test_config.rs
│           ├── fake_clock.rs
│           └── cluster.rs           # netns-based TestCluster (Linux)
│
├── orchestrator-gui/                # React + TypeScript web app
│   ├── package.json
│   ├── tsconfig.json
│   ├── vite.config.ts
│   ├── index.html
│   ├── e2e/                         # Playwright
│   └── src/
│       ├── main.tsx
│       ├── App.tsx
│       ├── theme.ts
│       ├── api/
│       │   ├── client.ts            # Fetch wrapper with auth
│       │   ├── types.ts
│       │   └── endpoints.ts
│       ├── auth/
│       │   ├── AuthContext.tsx
│       │   └── LoginPage.tsx
│       ├── components/
│       │   ├── TenantSwitcher.tsx
│       │   ├── TreeView.tsx
│       │   ├── SankeyDiagram.tsx
│       │   ├── RollingUpdate.tsx
│       │   ├── DnsZoneEditor.tsx
│       │   └── HealthDashboard.tsx
│       ├── pages/
│       │   ├── Dashboard.tsx
│       │   ├── TenantDetail.tsx
│       │   ├── Rollouts.tsx
│       │   ├── Dns.tsx
│       │   └── Settings.tsx
│       └── hooks/
│
├── systemd/
│   ├── orchestrator-manager.service
│   └── orchestrator-tenant.service.template
│
├── packaging/
│   ├── deb/
│   │   ├── control
│   │   ├── postinst
│   │   └── postrm
│   ├── rpm/
│   │   └── orchestrator.spec
│   └── Makefile
│
├── docs/
│   ├── architecture.md
│   ├── getting-started.md
│   ├── configuration.md
│   ├── api-reference.md
│   ├── deployment.md
│   ├── operations.md
│   └── troubleshooting.md
│
├── examples/
│   ├── tenant.yaml                  # A complete, working example
│   ├── registry.yaml
│   └── hierarchy.yaml
│
├── system-tests/                    # Privileged Linux tests
│   ├── run.sh
│   ├── Cargo.toml
│   └── tests/
│       ├── tenant_isolation.rs
│       └── vip_failover.rs
│
├── scripts/
│   ├── bootstrap.sh                 # One-command dev environment
│   ├── gen-systemd-unit.sh
│   └── check-manifest.sh            # Verifies Cargo.toml ↔ 09-dependencies.md
│
└── .github/
    ├── workflows/
    │   ├── ci.yml
    │   ├── release.yml
    │   └── nightly.yml
    ├── ISSUE_TEMPLATE/
    │   ├── bug_report.md
    │   └── feature_request.md
    ├── PULL_REQUEST_TEMPLATE.md
    └── dependabot.yml
```

---

## 2. Workspace Layout

```toml
[workspace]
resolver = "2"
members = [
    "crates/orchestrator-core",
    "crates/orchestrator",
    "crates/orchestrator-manager",
    "crates/orchestrator-proto",
    "crates/orchestrator-testkit",
]
```

### Dependency Graph

```
                     orchestrator-proto
                       ↙           ↘
        orchestrator-core        (generated pb)
              ↑        ↑
              │        │
      orchestrator   orchestrator-manager

    orchestrator-testkit ──▶ orchestrator-core   (dev-dependency only)
```

Rules:

- `orchestrator-core` depends on **no other** orchestrator crate. It is the shared vocabulary.
- `orchestrator-proto` depends on nothing internal.
- `orchestrator` and `orchestrator-manager` are **siblings** — neither depends on the other.
  The manager talks to tenants over the network (REST/gRPC), not over Rust types.
- No cycles. Enforced by CI with `cargo metadata` + a graph check.

---

## 3. Build Commands

The `Makefile` is the entry point. No one memorises cargo flags.

| Command | Does |
|---------|------|
| `make build` | `cargo build --workspace` |
| `make release` | `cargo build --workspace --release` + GUI build |
| `make test` | `cargo test --workspace` |
| `make test-system` | The privileged Linux system tests |
| `make lint` | `cargo fmt --check` + `cargo clippy -- -D warnings` + `cargo deny` + `npm run lint` |
| `make fmt` | `cargo fmt` + `npm run format` |
| `make docs` | `cargo doc --workspace --no-deps` + GUI docs |
| `make deps` | `cargo outdated` + `npm outdated` |
| `make gui` | `npm --prefix orchestrator-gui run dev` |
| `make dev-stack` | `scripts/bootstrap.sh` — a local 2-tenant stack with mock backends |
| `make check` | `lint` + `test` + `build` — what CI runs locally |

---

## 4. CI Pipeline

### `ci.yml`

| Job | Runs on | Steps |
|-----|---------|-------|
| `lint` | ubuntu-latest | `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo deny check`, `npm run lint`, `npx tsc --noEmit` |
| `unit` | ubuntu-latest | `cargo test --workspace --lib` — matrix: stable, 1.99, MSRV |
| `integration` | ubuntu-latest | `cargo test --workspace --tests` |
| `component` | ubuntu-latest | `cargo test -p orchestrator --test component` |
| `system` | ubuntu-latest | `sudo ./system-tests/run.sh` — netns + systemd |
| `gui` | ubuntu-latest | `npm run test`, `npm run build` |
| `e2e` | ubuntu-latest | `npm --prefix orchestrator-gui run e2e` |
| `manifest` | ubuntu-latest | `scripts/check-manifest.sh` — no undocumented dependency |
| `coverage` | ubuntu-latest | `cargo llvm-cov` → Codecov, threshold check |
| `build` | ubuntu-latest | Release build, `x86_64` + `aarch64` cross |

### `release.yml`

Triggered by a tag `v*`:

1. Verify the tag matches `Cargo.toml` and `package.json` versions
2. Build release binaries for `x86_64-unknown-linux-gnu` and `aarch64-unknown-linux-gnu`
3. Build the GUI bundle and embed it in the binary with `rust-embed` (the GUI ships inside the
   tenant binary and is served by the manager — no separate web server)
4. Build the DEB and RPM packages
5. Build and push a container image to GHCR
6. Generate a checksum file and the release notes from `CHANGELOG.md`
7. Create the GitHub release with the artifacts attached

### `nightly.yml`

Runs the full suite plus the 10× flake check and the load tests. Publishes a performance
comparison against the previous nightly.

---

## 5. Branching & Release Process

| Branch | Purpose | Merges to |
|--------|---------|-----------|
| `main` | Released code. Always green | — |
| `develop` | Integration branch | `main` on release |
| `feature/<name>` | One feature | `develop` |
| `fix/<name>` | One bug | `develop` |
| `release/<version>` | Release prep | `main` |

**Commit message format** — Conventional Commits:

```
feat(rollout): support manual step advancement
fix(vip): release the VIP before exiting on SIGTERM
docs(api): document the rollout endpoints
```

**Pull request requirements**:

- Linked to an issue or a tracker task
- Tests added for new behaviour
- `make check` passes locally
- The tracker in `06-tracker.md` is updated
- Any new dependency is added to `09-dependencies.md`

---

## 6. Packaging

### DEB

```
orchestrator_1.0.0_amd64.deb
├── /usr/local/bin/orchestrator
├── /usr/local/bin/orchestrator-manager
├── /lib/systemd/system/orchestrator-manager.service
├── /etc/orchestrator/            # registry.yaml, auth.json
└── /usr/share/doc/orchestrator/
```

### RPM

Mirrors the DEB layout with a `.spec` file, plus:

- `%post` creates the `orchestrator` system user and group
- `%post` creates `/etc/orchestrator` with `0700`
- `%preun` stops running tenants on uninstall

### Container

```dockerfile
FROM debian:bookworm-slim
RUN useradd --system --no-create-home --uid 1500 orchestrator
COPY --from=builder /out/orchestrator /usr/local/bin/
USER orchestrator
EXPOSE 8080 8443 9090 9091
ENTRYPOINT ["/usr/local/bin/orchestrator"]
```

The container image is **not** the primary deployment mechanism — systemd units are. The container
exists for CI, for Kubernetes-style environments, and for the manager's own packaging.

---

## 7. Development Environment

### Prerequisites

| Tool | Version |
|------|---------|
| Rust | 1.99 (via `rustup`, pinned by `rust-toolchain.toml`) |
| Node.js | 22 LTS (`nvm use`) |
| npm | 10 (ships with Node 22) |
| Docker | For the local stack |
| `iproute2` | Linux only — for the VIP tests |

### One-command setup

```bash
./scripts/bootstrap.sh
```

This installs the toolchain, fetches dependencies, builds the workspace, generates a self-signed
certificate, writes `examples/tenant.yaml` to `/tmp/`, and starts a local stack with mock
backends.

### The local dev stack

`docker-compose.dev.yml` brings up:

| Service | Purpose |
|---------|---------|
| `mock-backend-a` / `-b` / `-c` | Mock backends returning their hostname, with configurable latency and failure rate |
| `orchestrator` | The tenant binary with the example config, VIP `10.0.0.100` on the compose network |
| `orchestrator-standby` | The second worker of the same tenant |
| `orchestrator-manager` | The manager with the example registry |
| `orchestrator-gui` | Vite dev server with a proxy to the manager |

Because the VIP cannot be added inside an unprivileged container, the dev stack uses a
**loopback alias** (`ip addr add 10.0.0.100/32 dev lo`) inside the container, which requires
`--cap-add NET_ADMIN`. The `system-tests/` suite covers the real thing with network namespaces.

---

## 8. Repository Governance

| File | Purpose |
|------|---------|
| `LICENSE` | **Apache-2.0**. Permissive, patent-granting, compatible with GPLv3 linking, and the default for infrastructure software consumed by both open and closed products |
| `README.md` | What it is, why, quick start, architecture diagram, status |
| `CHANGELOG.md` | Keep a Changelog format, one section per release |
| `CONTRIBUTING.md` | How to build, test, and submit a PR |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 2.1 |
| `SECURITY.md` | How to report a vulnerability, supported versions, disclosure timeline |
| `.github/PULL_REQUEST_TEMPLATE.md` | Checklist based on the Definition of Done |

### Why Apache-2.0

The project is infrastructure that enterprises deploy. Apache-2.0 is the most permissive licence
with an explicit patent grant, so downstream vendors can build commercial products on top without
the copyleft obligations that would block adoption. It matches the Rust ecosystem norm (tokio,
hyper, axum are all MIT/Apache-2.0).

---

## 9. Required Repository Settings

| Setting | Value | Why |
|---------|-------|-----|
| Default branch | `main` | |
| Merge commits | Allowed, squash preferred | |
| Branch protection on `main` | Required checks: `lint`, `unit`, `integration`, `component`, `manifest` | |
| Issue templates | Bug report, feature request | |
| Dependabot | Cargo + npm, weekly | |
| Secret scanning | Enabled | |
| Push protection | Enabled | |
| Discussions | Enabled | |
| Wiki | Disabled — `docs/` is the source of truth | |

---

## 10. Documentation Structure

`docs/` is user-facing and is written for an operator, not a developer.

| Document | Audience | Contents |
|----------|----------|----------|
| `docs/getting-started.md` | New user | Install, first tenant, first route, verify it works |
| `docs/configuration.md` | Operator | Every config field, with a full annotated example |
| `docs/architecture.md` | Developer | How the pieces fit together, with diagrams |
| `docs/api-reference.md` | Integrator | The REST API, mirroring `08-api-reference.md` |
| `docs/deployment.md` | Operator | systemd install, TLS certs, network requirements, firewall rules |
| `docs/operations.md` | Operator | Day-2: health checks, rollouts, DNS changes, scaling, backups |
| `docs/troubleshooting.md` | Operator | Symptoms → diagnosis → fix |
| `docs/architecture.md` | Developer | Component design, data flow, extension points |

The `plan/` directory is the **internal** design record. It is committed to the repository because
the design rationale and the acceptance criteria are part of the project's value, but it is not
part of the published documentation site.
