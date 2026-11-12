//! The Orchestrator per-tenant process.
//!
//! Each tenant runs as its own process with its own configuration, VIP, listeners, and resource
//! limits. A crash in one tenant cannot affect any other.

mod cli;
mod control_plane;
mod data_plane;
mod runtime;

use std::net::SocketAddr;
use std::process::ExitCode;
use std::sync::Arc;

use clap::Parser;
use orchestrator_core::config::model::TenantConfig;
use orchestrator_core::config::validate::validate;
use tracing::{debug, error, info, warn};

use crate::cli::Cli;
use crate::control_plane::engine::ConfigEngine;

/// The address the HTTP listener binds to when the config does not specify one.
const DEFAULT_HTTP_BIND: &str = "0.0.0.0:8080";

fn main() -> ExitCode {
    let cli = Cli::parse();

    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(error) => {
            eprintln!("orchestrator: failed to start the tokio runtime: {error}");
            return ExitCode::FAILURE;
        }
    };

    match runtime.block_on(run(cli)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("orchestrator: {error}");
            ExitCode::FAILURE
        }
    }
}

/// The main asynchronous entry point.
async fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    // Load the config first so the log level from the config applies to the startup logs.
    let config = orchestrator_core::config::parse::load_tenant_config(&cli.config)?;
    let level = cli
        .log_level
        .clone()
        .unwrap_or_else(|| config.logging.level.clone());
    runtime::init_tracing(&level, &config.logging.format);

    info!(
        tenant = %config.tenant.name,
        version = orchestrator_core::VERSION,
        "starting orchestrator"
    );

    if cli.check {
        let errors = validate(&config);
        if errors.is_empty() {
            info!("configuration is valid");
            return Ok(());
        }
        error!("configuration is invalid:\n{errors}");
        return Err(format!("configuration is invalid:\n{errors}").into());
    }

    // Printing the routing table is a diagnostic, so it validates first — showing a table
    // built from an invalid config would be misleading.
    let errors = validate(&config);
    if !errors.is_empty() {
        error!("configuration is invalid:\n{errors}");
        return Err(format!("configuration is invalid:\n{errors}").into());
    }

    let engine = Arc::new(ConfigEngine::new(config.clone()));

    if cli.print_routes {
        print_routes(&engine);
        return Ok(());
    }

    engine.log_summary();

    let bind = resolve_bind(&config, cli.http_bind.as_deref());
    let bind: SocketAddr = bind
        .parse()
        .map_err(|error| format!("invalid HTTP bind address '{bind}': {error}"))?;

    info!(%bind, "http listener starting");
    serve(bind, engine.clone(), cli.clone()).await?;

    Ok(())
}

/// Prints the resolved routing table.
fn print_routes(engine: &ConfigEngine) {
    let table = engine.table();
    println!("{:<40} {:<24} {:>10}", "ENDPOINT", "ADDRESS", "WEIGHT");
    for endpoint in &table.endpoints {
        println!(
            "{:<40} {:<24} {:>9.4}%",
            endpoint.path,
            endpoint.endpoint.address.to_string(),
            endpoint.effective_weight * 100.0
        );
    }
}

/// Resolves the HTTP bind address, preferring the CLI override.
fn resolve_bind(config: &TenantConfig, override_bind: Option<&str>) -> String {
    if let Some(bind) = override_bind {
        return bind.to_string();
    }
    config.listeners.http.as_ref().map_or_else(
        || DEFAULT_HTTP_BIND.to_string(),
        |listener| listener.bind.to_string(),
    )
}

/// Runs the HTTP server until the process is signalled.
async fn serve(
    bind: SocketAddr,
    engine: Arc<ConfigEngine>,
    cli: Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    let listener = tokio::net::TcpListener::bind(bind).await?;

    // The reload watcher and the accept loop run concurrently; whichever finishes first ends
    // the process.
    tokio::select! {
        result = accept_loop(listener, engine.clone()) => {
            result?;
        }
        result = watch_for_reload(engine, cli) => {
            result?;
        }
    }

    Ok(())
}

/// Accepts connections and serves them.
async fn accept_loop(
    listener: tokio::net::TcpListener,
    engine: Arc<ConfigEngine>,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = data_plane::http::ProxyService::new(engine.config());

    loop {
        let (stream, peer) = listener.accept().await?;
        let service = service.clone();

        tokio::spawn(async move {
            let io = hyper_util::rt::TokioIo::new(stream);
            let result = hyper::server::conn::http1::Builder::new()
                .serve_connection(
                    io,
                    hyper::service::service_fn(move |request| {
                        let service = service.clone();
                        async move { service.handle(request).await }
                    }),
                )
                .await;

            if let Err(error) = result {
                debug!(%peer, %error, "connection closed with an error");
            }
        });
    }
}

/// Watches the config file for changes and reloads it when it does.
///
/// The manager normally signals `SIGHUP`; this watcher is the fallback for a tenant running
/// without a manager. Both paths end in the same atomic swap.
async fn watch_for_reload(
    engine: Arc<ConfigEngine>,
    cli: Cli,
) -> Result<(), Box<dyn std::error::Error>> {
    use tokio::signal::unix::{signal, SignalKind};

    let mut hangup = signal(SignalKind::hangup())?;
    let path = cli.config.clone();

    loop {
        hangup.recv().await;

        info!(path = %path.display(), "reload requested");
        match orchestrator_core::config::parse::load_tenant_config(&path) {
            Ok(config) => {
                if let Err(errors) = engine.reload(config) {
                    warn!("reload rejected, keeping the running configuration");
                    let _ = errors;
                }
            }
            Err(error) => {
                error!(%error, "failed to read the config, keeping the running configuration");
            }
        }
    }
}
