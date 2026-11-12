//! Command-line arguments for the per-tenant binary.

use std::path::PathBuf;

use clap::Parser;

/// The per-tenant Orchestrator process.
///
/// Each tenant runs as its own process with its own configuration, VIP, listeners, and resource
/// limits. See `plan/03-architecture.md` for the process model.
#[derive(Debug, Clone, Parser)]
#[command(name = "orchestrator", version, about, long_about = None)]
pub struct Cli {
    /// The tenant configuration file to load.
    #[arg(long, value_name = "PATH", env = "ORCHESTRATOR_CONFIG")]
    pub config: PathBuf,

    /// This worker's stable identifier. Distinguishes the two workers of a pair.
    #[arg(long, value_name = "ID", env = "ORCHESTRATOR_NODE_ID")]
    pub node_id: Option<String>,

    /// Override the plaintext HTTP listener address from the config.
    #[arg(long, value_name = "ADDR", env = "ORCHESTRATOR_HTTP_BIND")]
    pub http_bind: Option<String>,

    /// Override the TLS listener address from the config.
    #[arg(long, value_name = "ADDR", env = "ORCHESTRATOR_HTTPS_BIND")]
    pub https_bind: Option<String>,

    /// Override the metrics listener address from the config.
    #[arg(long, value_name = "ADDR", env = "ORCHESTRATOR_METRICS_BIND")]
    pub metrics_bind: Option<String>,

    /// Override the gRPC sync listener address from the config.
    #[arg(long, value_name = "ADDR", env = "ORCHESTRATOR_SYNC_BIND")]
    pub sync_bind: Option<String>,

    /// Override the DNS listener address from the config.
    #[arg(long, value_name = "ADDR", env = "ORCHESTRATOR_DNS_BIND")]
    pub dns_bind: Option<String>,

    /// The sync peer's address. The worker fails over when this peer stops heartbeating.
    #[arg(long, value_name = "ADDR", env = "ORCHESTRATOR_SYNC_PEER")]
    pub sync_peer: Option<String>,

    /// Validate the configuration and exit without serving traffic.
    #[arg(long)]
    pub check: bool,

    /// Print the resolved routing table and exit.
    #[arg(long)]
    pub print_routes: bool,

    /// The log level. Overrides the config.
    #[arg(long, value_name = "LEVEL", env = "ORCHESTRATOR_LOG_LEVEL")]
    pub log_level: Option<String>,
}
