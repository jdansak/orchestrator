# Orchestrator documentation

This directory is the **operator-facing** documentation. It is written for someone running
Orchestrator, not for someone working on it.

The **design record** — the goals, the architecture, the acceptance criteria, and the
implementation plan — lives in [`../plan/`](../plan/). That directory is committed to the
repository because the design rationale is part of the project's value.

| Document | Audience | Contents |
|----------|----------|----------|
| [Getting started](getting-started.md) | New user | Install, create a tenant, verify it works |
| [Configuration](configuration.md) | Operator | Every config field, with a full annotated example |
| [Architecture](architecture.md) | Developer | How the pieces fit together |
| [API reference](api-reference.md) | Integrator | The REST APIs |
| [Operations](operations.md) | Operator | Day-2: health, rollouts, DNS, scaling |
| [Troubleshooting](troubleshooting.md) | Operator | Symptoms → diagnosis → fix |

## Conventions

| | |
|---|---|
| Config paths | `/etc/orchestrator/` |
| Tenant configs | `/etc/orchestrator/tenants/<tenant>/tenant.yaml` |
| The registry | `/etc/orchestrator/registry.yaml` |
| Credentials | `/etc/orchestrator/auth.json`, mode `0600` |
| Runtime state | `/var/lib/orchestrator/tenants/<tenant>/` |
| Logs | The systemd journal, and `/var/log/orchestrator/<tenant>.log` |
