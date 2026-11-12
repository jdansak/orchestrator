//! The Orchestrator manager.
//!
//! A single process per host that owns the tenant lifecycle: it starts and stops tenants,
//! allocates VIPs, watches for configuration changes, and serves the management API.
//!
//! The tenant lifecycle, the API, and the config watcher land in Phase 4. What exists today is
//! the registry model, its validation — including the port-conflict detection that prevents two
//! tenants from claiming the same port — and the process skeleton that will host them.

mod cli;
mod registry;

use std::process::ExitCode;

use clap::Parser;
use tracing::{error, info};

use crate::cli::Cli;
use crate::registry::Registry;

fn main() -> ExitCode {
    let cli = Cli::parse();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("orchestrator-manager: failed to start the tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orchestrator-manager: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The main entry point.
async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .json()
        .init();

    info!(
        version = orchestrator_core::VERSION,
        "starting orchestrator-manager"
    );

    let registry = Registry::load(&cli.registry)?;

    if let Err(error) = registry.validate() {
        error!(%error, "the registry is not valid");
        return Err(Box::new(error));
    }

    for tenant in &registry.tenants {
        info!(
            tenant = %tenant.name,
            vip = %tenant.vip,
            workers = tenant.workers.len(),
            "registered tenant"
        );
    }

    if cli.check {
        info!(tenants = registry.tenants.len(), "the registry is valid");
        return Ok(());
    }

    // The supervisor, the API, and the config watcher land in Phase 4. Until then the manager
    // idles so the process model can be exercised end to end.
    info!("the manager is idle — the tenant supervisor lands in Phase 4");
    wait_for_termination().await;

    Ok(())
}

/// Waits until the process is asked to stop.
async fn wait_for_termination() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};

        let mut terminate = match signal(SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                error!(%error, "failed to install the SIGTERM handler");
                return;
            }
        };
        let mut interrupt = match signal(SignalKind::interrupt()) {
            Ok(signal) => signal,
            Err(error) => {
                error!(%error, "failed to install the SIGINT handler");
                return;
            }
        };

        tokio::select! {
            _ = terminate.recv() => info!("received SIGTERM, shutting down"),
            _ = interrupt.recv() => info!("received SIGINT, shutting down"),
        }
    }
}
