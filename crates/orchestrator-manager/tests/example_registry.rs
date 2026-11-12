//! Validates the example registry.
//!
//! The registry example is what an operator copies when setting up the first tenant. It must
//! parse, it must pass validation, and it must not contain a port conflict — the exact bug the
//! manager's validation exists to catch.

use std::path::PathBuf;

use orchestrator_manager::registry::Registry;

/// The repository root, relative to this crate.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .canonicalize()
        .expect("the repository root exists")
}

#[test]
fn the_example_registry_is_valid() {
    let path = repo_root().join("examples/registry.yaml");
    let registry = Registry::load(&path).expect("the example registry parses");

    registry.validate().expect("the example registry is valid");

    assert_eq!(registry.tenants.len(), 2);
    assert_eq!(registry.tenants[0].name, "app-frontend");
    assert_eq!(
        registry.tenants[0].workers.len(),
        2,
        "app-frontend is an HA pair"
    );
    assert_eq!(
        registry.tenants[1].workers.len(),
        1,
        "payments is a single worker"
    );
}

#[test]
fn the_example_registry_has_no_port_conflicts() {
    let path = repo_root().join("examples/registry.yaml");
    let registry = Registry::load(&path).expect("the example registry parses");

    for tenant in &registry.tenants {
        for worker in &tenant.workers {
            let mut ports = worker.ports.all().to_vec();
            ports.sort_unstable();
            let before = ports.len();
            ports.dedup();
            assert_eq!(
                before,
                ports.len(),
                "worker {} binds the same port twice",
                worker.node_id
            );
        }
    }
}
