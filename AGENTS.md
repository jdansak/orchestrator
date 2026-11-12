# Orchestrator — Project Documentation

This is the documentation set for **Orchestrator**, an open-source, application-level traffic manager for Kubernetes clusters and VM pools.

## Documents

| Document | File | Description |
|----------|------|-------------|
| Project Overview | [`plan/01-project-overview.md`](plan/01-project-overview.md) | Problem statement, solution, key differentiators vs F5 |
| Goals & Acceptance Criteria | [`plan/02-goals-and-acceptance-criteria.md`](plan/02-goals-and-acceptance-criteria.md) | 11 goals with requirements and test cases to prove each goal |
| Architecture | [`plan/03-architecture.md`](plan/03-architecture.md) | High-level design, component breakdown, VIP management, DNS, isolation, manager |
| Code Standards | [`plan/04-code-standards.md`](plan/04-code-standards.md) | Language, naming, error handling, async patterns, logging, testing, CI/CD, security, performance |
| Implementation Plan | [`plan/05-implementation-plan.md`](plan/05-implementation-plan.md) | 10 phases with tasks, estimates, phase-level acceptance criteria, and a requirement traceability table |
| Tracker | [`plan/06-tracker.md`](plan/06-tracker.md) | Running checklist of all tasks and test cases by phase and by goal |
| Config Schema | [`plan/07-config-schema.md`](plan/07-config-schema.md) | Normative spec for `tenant.yaml` and `registry.yaml`, route matching, weight composition, session affinity, validation rules |
| API Reference | [`plan/08-api-reference.md`](plan/08-api-reference.md) | Manager and Tenant REST APIs, authentication, metrics, event stream |
| Dependencies | [`plan/09-dependencies.md`](plan/09-dependencies.md) | Every third-party dependency with a pinned version and rationale, licence policy |
| Test Strategy | [`plan/10-test-strategy.md`](plan/10-test-strategy.md) | Test layers, testkit fixtures, the full acceptance-criteria-to-test map, CI strategy |
| Non-Functional Requirements | [`plan/11-non-functional-requirements.md`](plan/11-non-functional-requirements.md) | Performance budgets, resilience, scalability limits, security, Definition of Done, release criteria |
| Repo Layout & Tooling | [`plan/12-repo-layout-and-tooling.md`](plan/12-repo-layout-and-tooling.md) | Directory layout, CI pipeline, branching, packaging, dev environment, governance |

## Quick Reference

- **What it is**: Standalone traffic manager, no K8 dependency, runs on commodity Linux
- **Language**: Rust (2021 edition), 1.99 stable
- **Four pieces**: `orchestrator` (per-tenant), `orchestrator-manager` (single), `orchestrator-gui` (web), `orchestrator-core` (shared types)
- **Process model**: systemd, one process per tenant
- **VIP**: Linux kernel iproute2 via netlink (`rtnetlink`)
- **TLS termination**: At the VIP, using rustls
- **DNS**: Authoritative for tenant zones, forwards everything else upstream (`hickory-server`)
- **Target platform**: Linux. Everything else is a development convenience

## How to Use

1. Read `./plan/01-project-overview.md` to understand what we're building and why
2. Read `./plan/02-goals-and-acceptance-criteria.md` to understand what "done" looks like
3. Read `./plan/03-architecture.md` to understand how it works
4. Read `./plan/04-code-standards.md` before writing any code
5. Read `./plan/07-config-schema.md` before touching config parsing or writing a config
6. Follow `./plan/05-implementation-plan.md` for the build order
7. Track progress in `./plan/06-tracker.md`

## Working Agreements

- **A requirement with no task is a gap.** The traceability table at the end of
  `05-implementation-plan.md` maps every `G-x.y` requirement to a task. Keep it current.
- **A criterion with no test is not done.** `10-test-strategy.md` maps every `TC-xxx` to an
  automated test. CI fails if any criterion is unmapped.
- **No new dependency without a manifest entry.** `09-dependencies.md` lists every crate;
  `scripts/check-manifest.sh` enforces it in CI.
- **The Definition of Done is in `11-non-functional-requirements.md` §8.** Apply it in review.
- **Design decisions belong in `plan/`, not in the code.** If you make a decision that contradicts
  a document, change the document in the same PR.
