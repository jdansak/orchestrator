//! The Orchestrator manager's library.
//!
//! The manager is a single process per host that owns the tenant lifecycle: it starts and stops
//! tenants, allocates VIPs, watches for configuration changes, and serves the management API.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`registry`] | The tenant registry model and its validation |

#![deny(missing_docs)]

pub mod registry;

pub use registry::{Registry, RegistryError, TenantEntry, WorkerEntry, WorkerPorts};
