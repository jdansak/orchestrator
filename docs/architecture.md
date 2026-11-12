# Architecture

The **normative** architecture is [`plan/03-architecture.md`](../plan/03-architecture.md). This
page is the reading version.

---

## The three processes

```
┌──────────────────────────────────────────────────────────────┐
│                       Host (Linux)                            │
│                                                               │
│  orchestrator-manager (systemd)                               │
│  ├── Starts and stops tenant services                        │
│  ├── Allocates VIPs from the pool                            │
│  ├── Watches for config changes                              │
│  └── Serves the management API and the GUI                   │
│                                                               │
│  orchestrator-app-frontend (systemd) ── sync ── standby      │
│  ├── Control Plane: config, health, VIP, rollouts            │
│  ├── Data Plane: HTTP/HTTPS, TCP, UDP proxies                │
│  ├── DNS Server                                              │
│  └── Sync Peer (gRPC)                                        │
└──────────────────────────────────────────────────────────────┘
```

| Piece | Runs | Scope |
|-------|------|-------|
| `orchestrator` | Once per tenant, times the number of workers | One tenant |
| `orchestrator-manager` | Once per host | Every tenant on the host |
| `orchestrator-gui` | A static bundle served by the manager | Every tenant on the host |
| `orchestrator-core` | A library | Shared types and logic |

---

## Why a process per tenant

A tenant is an isolation boundary. Its own process means:

- Its own configuration file
- Its own VIP and listeners
- Its own resource limits, enforced by cgroups
- Its own memory

A crash in one tenant is a crash in one process. The others do not notice.

The tenant does **not** get its own network namespace — it has to be reachable on the host
network to own a VIP.

---

## The request path

```
client
  │
  ▼
┌──────────────────────┐
│ VIP (on the NIC)     │   the kernel does the address ownership
└──────────┬───────────┘
           ▼
┌──────────────────────┐
│ TLS termination      │   rustls; TLS 1.2 minimum
└──────────┬───────────┘
           ▼
┌──────────────────────┐
│ Route matching       │   longest prefix wins; path boundaries respected
└──────────┬───────────┘
           ▼
┌──────────────────────┐
│ Session affinity     │   cookie / header / client-ip, or none
└──────────┬───────────┘
           ▼
┌──────────────────────┐
│ Backend selection    │   weighted random over the healthy endpoints
└──────────┬───────────┘
           ▼
       backend
```

Every step is defined in [`plan/07-config-schema.md`](../plan/07-config-schema.md) and
[`plan/11-non-functional-requirements.md`](../plan/11-non-functional-requirements.md).

---

## Hot reload

The routing table is held behind an `ArcSwap`. A reload is a **pointer swap**, not a rebuild in
place:

1. Read the file
2. Parse and validate — every problem is collected, not just the first
3. On failure: log, and keep the running config
4. On success: build the new table and swap the pointer

Readers never see a partially-built table, and no reader-side lock is needed. Connections that
are already open finish against the old table.

---

## The worker pair

Two workers share a VIP. The active one owns the address; the standby mirrors its state over a
gRPC channel.

```
active                          standby
  │                                │
  ├── heartbeat (300ms) ──────────►│
  │◄────── heartbeat ───────────────┤
  │                                │
  │  ← active dies ────────────────┤
  │                                │
  │                    peer_timeout (1s) elapses
  │                                │
  │                    ip addr add <vip>  → becomes active
```

The VIP address itself is the single source of truth for who is active. If both workers ever
believed they were active, `ip addr add` would fail for one of them with `EEXIST`, and that
worker would demote itself. There is no separate consensus protocol.

---

## The crate layout

```
orchestrator-core ──► orchestrator
       ▲
       │
       └──► orchestrator-manager

orchestrator-proto ──► orchestrator, orchestrator-manager
orchestrator-testkit ──► orchestrator-core   (dev only)
```

`orchestrator` and `orchestrator-manager` are **siblings**. Neither depends on the other — the
manager talks to tenants over the network, not over Rust types.
