//! The control plane's configuration engine.
//!
//! Holds the active configuration behind an [`ArcSwap`], so a reload is a pointer swap that
//! readers never observe as a partially-built routing table. See
//! `plan/03-architecture.md` "Configuration Reload".

use std::sync::Arc;

use arc_swap::ArcSwap;
use orchestrator_core::config::model::TenantConfig;
use orchestrator_core::config::validate::validate;
use orchestrator_core::error::ValidationErrors;
use orchestrator_core::hierarchy::FlatEndpoint;
use tracing::{error, info};

/// The active routing table.
#[derive(Debug, Clone)]
pub struct RoutingTable {
    /// The endpoints in scope, with their effective weights.
    pub endpoints: Vec<FlatEndpoint>,
}

/// The configuration engine.
#[derive(Debug)]
pub struct ConfigEngine {
    /// The active configuration.
    config: ArcSwap<TenantConfig>,

    /// The routing table derived from the active configuration.
    table: ArcSwap<RoutingTable>,
}

impl ConfigEngine {
    /// Creates an engine from an already-validated configuration.
    pub fn new(config: TenantConfig) -> Self {
        let table = RoutingTable {
            endpoints: orchestrator_core::hierarchy::flatten_region(&config.hierarchy, "*"),
        };
        Self {
            config: ArcSwap::from(Arc::new(config)),
            table: ArcSwap::from(Arc::new(table)),
        }
    }

    /// The active configuration.
    pub fn config(&self) -> Arc<TenantConfig> {
        self.config.load_full()
    }

    /// The active routing table.
    pub fn table(&self) -> Arc<RoutingTable> {
        self.table.load_full()
    }

    /// Validates a candidate configuration and, if it is valid, swaps it in.
    ///
    /// On failure the running configuration is left untouched and the errors are logged
    /// (`G9.3`). On success the swap is atomic (`G9.4`).
    pub fn reload(&self, candidate: TenantConfig) -> Result<(), ValidationErrors> {
        let errors = validate(&candidate);
        if !errors.is_empty() {
            error!(
                tenant = %candidate.tenant.name,
                "config validation failed, rejecting the change:\n{errors}"
            );
            return Err(errors);
        }

        let table = RoutingTable {
            endpoints: orchestrator_core::hierarchy::flatten_region(&candidate.hierarchy, "*"),
        };

        // Swap the table first, then the config, so a reader that sees the new table also sees
        // the config that produced it.
        self.table.store(Arc::new(table));
        self.config.store(Arc::new(candidate));

        info!("configuration reloaded successfully");
        Ok(())
    }

    /// Logs a summary of the active routing table.
    pub fn log_summary(&self) {
        let config = self.config();
        let table = self.table();
        info!(
            "routing table has {} endpoints across {} regions",
            table.endpoints.len(),
            config.hierarchy.regions.len()
        );
    }
}
