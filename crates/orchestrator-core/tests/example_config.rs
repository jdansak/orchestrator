//! Validates the example configuration.
//!
//! The example is the first thing a new user copies. If it drifts away from the schema it
//! becomes a trap, so the test suite checks it.

use std::path::PathBuf;

use orchestrator_core::config::parse::load_tenant_config;
use orchestrator_core::config::validate::validate;
use orchestrator_core::hierarchy::flatten;

/// The repository root, relative to this crate.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("the repository root exists")
}

#[test]
fn the_example_tenant_config_is_valid() {
    let path = repo_root().join("examples/tenant.yaml");
    let config = load_tenant_config(&path).expect("the example config parses");

    let errors = validate(&config);
    assert!(
        errors.is_empty(),
        "the example config is invalid:\n{errors}"
    );
}

#[test]
fn the_example_tenant_config_resolves_to_the_documented_weights() {
    let path = repo_root().join("examples/tenant.yaml");
    let config = load_tenant_config(&path).expect("the example config parses");

    let endpoints = flatten(&config.hierarchy);
    let total: f64 = endpoints.iter().map(|e| e.effective_weight).sum();
    assert!(
        (total - 1.0).abs() < 1e-9,
        "the effective weights sum to {total}, not 1.0"
    );

    let by_path = |path: &str| {
        endpoints.iter().find(|e| e.path == path).map_or_else(
            || panic!("no endpoint named {path}"),
            |e| e.effective_weight,
        )
    };

    assert!((by_path("us-east/dc-1/prod/ep-a") - 0.36).abs() < 1e-9);
    assert!((by_path("us-east/dc-1/prod/ep-b") - 0.24).abs() < 1e-9);
    assert!((by_path("us-east/dc-1/prod-green/ep-green") - 0.0).abs() < 1e-9);
    assert!((by_path("eu-west/dc-2/prod/ep-c") - 0.40).abs() < 1e-9);
}

#[test]
fn the_example_tenant_config_uses_the_documented_schema_version() {
    let path = repo_root().join("examples/tenant.yaml");
    let config = load_tenant_config(&path).expect("the example config parses");
    assert_eq!(config.version, 1);
    assert_eq!(config.tenant.name, "app-frontend");
}
