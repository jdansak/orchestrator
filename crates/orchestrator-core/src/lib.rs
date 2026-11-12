//! Shared types, configuration model, and validation for Orchestrator.
//!
//! This crate is the common vocabulary of the project. It depends on no other Orchestrator crate
//! and holds no I/O — it is pure data and pure logic, which is what makes the routing rules
//! testable without spinning up a proxy.
//!
//! # Layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`config`] | The configuration model, its parser, and its validator |
//! | [`hierarchy`] | The four-level hierarchy and weight composition |
//! | [`weights`] | Weighted random selection |
//! | [`routing`] | Route matching |
//! | [`affinity`] | Session affinity key derivation |
//! | [`rollout`] | The rolling-update state machine |
//! | [`error`] | Error types |
//! | [`version`] | Schema and product version constants |
//!
//! # Example
//!
//! ```
//! use orchestrator_core::config::{parse_tenant_config, validate};
//!
//! let yaml = r#"
//! version: 1
//! tenant:
//!   name: app-frontend
//!   zone: app-frontend.internal
//! vip:
//!   address: 10.0.0.100/32
//!   interface: eth0
//! hierarchy:
//!   regions:
//!     - name: us-east
//!       weight: 100
//!       datacenters:
//!         - name: dc-1
//!           weight: 100
//!           clusters:
//!             - name: prod
//!               weight: 100
//!               endpoints:
//!                 - name: ep-a
//!                   weight: 100
//!                   address: 10.1.1.5:8080
//! "#;
//!
//! let config = parse_tenant_config(yaml, "example.yaml").expect("valid config");
//! let errors = validate(&config);
//! assert!(errors.is_empty(), "unexpected errors: {errors}");
//! ```

#![deny(missing_docs)]

pub mod affinity;
pub mod config;
pub mod error;
pub mod hierarchy;
pub mod rollout;
pub mod routing;
pub mod version;
pub mod weights;

pub use config::{parse_tenant_config, TenantConfig};
pub use error::{ConfigError, RolloutError, ValidationErrors, ValidationIssue, VipError};
pub use hierarchy::flatten;
pub use rollout::{Rollout, RolloutState};
pub use routing::match_route;
pub use version::{GIT_SHA, SCHEMA_VERSION, VERSION};
