# Contributing to Orchestrator

Thanks for your interest. This document is the short version of what you need to know.

---

## Before you start

1. **Read the design.** The `plan/` directory is the source of truth for what the project is and
   how it works. In particular:
   - `plan/04-code-standards.md` — how to write the code
   - `plan/07-config-schema.md` — the configuration schema
   - `plan/11-non-functional-requirements.md` §8 — the Definition of Done
2. **Open an issue** before starting anything larger than a bug fix. It saves everyone the work
   of a pull request that solves the wrong problem.

---

## Setting up

```bash
git clone https://github.com/Jdansak/orchestrator.git
cd orchestrator
./scripts/bootstrap.sh
```

The bootstrap script installs the toolchain, fetches the dependencies, and verifies the build.

| Tool | Version | Check with |
|------|---------|------------|
| Rust | 1.99 stable | `rustc --version` |
| Node.js | 22 LTS | `node --version` |
| Linux | kernel 5.10+ | only needed for the VIP and the system tests |

---

## The workflow

```bash
make check        # lint + test + build — exactly what CI runs
```

| Command | What it does |
|---------|--------------|
| `make build` | Build the workspace |
| `make test` | Run the unit and integration tests |
| `make lint` | `rustfmt`, `clippy -D warnings`, `cargo deny`, and the GUI linters |
| `make fmt` | Format everything |
| `make docs` | Build the Rust documentation |
| `make check` | Everything CI runs |

---

## Ground rules

These are not negotiable. They are what keeps the project reviewable.

### A requirement with no task is a gap

Every `G-x.y` requirement in `plan/02-goals-and-acceptance-criteria.md` maps to a task in
`plan/05-implementation-plan.md`. If you add a requirement, add the task in the same pull
request. If you change behaviour, update the requirement.

### A criterion with no test is not done

Every `TC-xxx` criterion has an automated test. The map lives in `plan/10-test-strategy.md`. CI
fails if any criterion has no test.

**No test sleeps for real time** to satisfy a timing requirement. Use `FakeClock` or
`tokio::time::pause()`. The only real sleeps permitted are in the system tests that measure
genuine wall-clock VIP failover.

### No new dependency without a manifest entry

Every third-party dependency is listed in `plan/09-dependencies.md` with a version and a
rationale. `scripts/check-manifest.sh` enforces this in CI.

### The Definition of Done

`plan/11-non-functional-requirements.md` §8 is the checklist applied in review. It covers code,
tests, documentation, and operational safety.

### Design decisions belong in `plan/`

If you make a decision that contradicts a document, change the document in the same pull request.
The documents are the design record, and a code change that quietly diverges from them is worse
than no change at all.

---

## Code style

The rules are in `plan/04-code-standards.md`. The short version:

- Rust 2021 edition, `rustfmt` with the repository's `rustfmt.toml`
- `clippy` with `-D warnings`. The enforced lint set is in `Cargo.toml` under `[workspace.lints]`
- `thiserror` for library errors, `anyhow` where application-level context matters
- `tracing` for logging. Never `println!` outside `main()` and tests
- Never `unwrap()`, `expect()`, or `panic!()` outside tests and `main()`
- Never block the async runtime. Use `tokio::fs`, `tokio::net`, and `spawn_blocking`
- No `unsafe` — the workspace forbids it

---

## Commits and pull requests

We use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(rollout): support manual step advancement
fix(vip): release the VIP before exiting on SIGTERM
docs(api): document the rollout endpoints
```

A pull request should:

- Link to an issue or a tracker task
- Include tests for the new behaviour
- Keep `make check` green
- Update `plan/06-tracker.md`
- Add any new dependency to `plan/09-dependencies.md`

---

## Reporting bugs

Open an issue with:

- What you did, what you expected, and what happened
- The Orchestrator version (`orchestrator --version`)
- The relevant logs, with secrets removed
- Your platform and configuration, if it is relevant

If the bug is a security issue, do **not** open a public issue. See `SECURITY.md`.

---

## Licence

By contributing, you agree that your contributions are licensed under the Apache-2.0 licence that
covers the project.
