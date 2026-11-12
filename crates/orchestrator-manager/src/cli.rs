//! Command-line arguments for the manager.

use std::path::PathBuf;

use clap::Parser;

/// The host-level manager for Orchestrator tenants.
#[derive(Debug, Parser)]
#[command(name = "orchestrator-manager", version, about, long_about = None)]
pub struct Cli {
    /// The tenant registry to read.
    #[arg(
        long,
        value_name = "PATH",
        default_value = "/etc/orchestrator/registry.yaml",
        env = "ORCHESTRATOR_REGISTRY"
    )]
    pub registry: PathBuf,

    /// The address the management API binds to.
    #[arg(
        long,
        value_name = "ADDR",
        default_value = "0.0.0.0:8081",
        env = "ORCHESTRATOR_MANAGER_BIND"
    )]
    pub bind: String,

    /// Validate the registry and exit.
    #[arg(long)]
    pub check: bool,
}
