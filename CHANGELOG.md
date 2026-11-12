# Changelog

All notable changes to this project are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project
adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- The full design record: 12 documents covering the goals, architecture, code standards,
  implementation plan, configuration schema, API reference, dependency manifest, test strategy,
  non-functional requirements, and repository layout.
- `orchestrator-core` — the configuration model, the validator, the hierarchy and weight
  composition, weighted random selection, route matching, session affinity key derivation, and
  the rolling-update state machine.
- `orchestrator` — the per-tenant binary: configuration loading with atomic hot reload, the
  HTTP proxy data plane, and the CLI.
- `orchestrator-manager` — the host-level manager: the tenant registry model and its
  validation, including the per-host port-conflict detection.
- `orchestrator-proto` — the gRPC definitions for the worker sync channel.
- `orchestrator-testkit` — the shared test fixtures: `MockBackend`, `LoadGenerator`,
  `TestConfigBuilder`, and `FakeClock`.
- A working Rust workspace with CI, `cargo deny` policy, a GUI scaffold, systemd templates, and
  RPM/DEB packaging scaffolding.

### Changed

- The YAML parser is now `serde-saphyr` — pure Rust, with no `unsafe` and no unmaintained
  transitive dependency. `serde_yaml` is deprecated upstream and `serde_yaml_ng` pulls in the
  archived `unsafe-libyaml`. The switch also means duplicate keys are rejected rather than
  silently resolved.
- The workspace path dependencies now declare a version. Without one, `cargo deny` treats them
  as wildcard dependencies.

### Added

- End-to-end tests for the HTTP data plane: the proxy really does forward to the backend, pass
  backend errors through, return `404` for an unmatched path, and return `502` for an
  unreachable one.
- A coverage gate in CI. `orchestrator-core` is at 95.6%, above its 95% target.

### Fixed

- Weight composition now treats a lone child as taking its parent's whole share. Under the
  original rule, a hierarchy whose only datacenter was declared at weight 70 silently dropped
  30% of its region's traffic.
- The duration parser saturated on overflow instead of reporting it. `18446744073709551615h`
  silently became a duration of 584 billion years; it is now rejected.
- The DNS records were only validated when the DNS server was enabled, so a typo in a record
  went unnoticed until the operator turned DNS on.
- The coverage script matched crate names as a path prefix, so it found no data at all.
- The ESLint configuration was migrated to the flat format that ESLint 9 requires.
- The DNS server is now opt-in (`dns.enabled` defaults to `false`), so a configuration without
  an upstream resolver is not rejected by default.
- The GUI's ESLint configuration was migrated to the flat format that ESLint 9 requires.

## [0.1.0] — unreleased

Initial design and foundation. Not yet released.
