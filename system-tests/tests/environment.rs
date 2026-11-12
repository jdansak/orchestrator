//! The privileged Linux system tests.
//!
//! These tests prove the requirements that cannot be proven without real privileges:
//!
//! - **G3** — VIP acquisition, release, and failover
//! - **G1** — tenant process isolation and resource limits
//!
//! They need `CAP_NET_ADMIN` to create network namespaces and move a VIP between them. Run
//! them with `sudo ./system-tests/run.sh`.
//!
//! The `TestCluster` fixture lands in Phase 2 task 2.8. What exists today is the environment
//! check, so a failure here is a missing capability rather than a missing test.

use std::process::Command;

/// Whether the process has the privileges the system tests need.
fn has_net_admin() -> bool {
    let Ok(status) = Command::new("ip").arg("addr").arg("add").arg("help").output() else {
        return false;
    };
    // `ip addr add help` exits non-zero but only after the kernel has parsed the request, so a
    // permissions error means the capability is missing.
    let stderr = String::from_utf8_lossy(&status.stderr);
    !stderr.contains("Operation not permitted") && !stderr.contains("Permission denied")
}

/// Whether `ip` is installed.
fn has_iproute2() -> bool {
    Command::new("ip").arg("-V").output().is_ok()
}

/// Whether network namespaces can be created.
fn can_create_netns() -> bool {
    Command::new("unshare")
        .arg("--net")
        .arg("true")
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

#[test]
fn the_environment_supports_the_system_tests() {
    assert!(has_iproute2(), "iproute2 is not installed — install it first");

    assert!(
        has_net_admin(),
        "the process lacks CAP_NET_ADMIN — run the system tests with sudo"
    );

    assert!(
        can_create_netns(),
        "network namespaces cannot be created — unshare may be unavailable"
    );
}

#[test]
fn a_vip_can_be_added_and_removed_on_the_loopback_interface() {
    // The VIP mechanism is a plain address add. This proves the kernel path works in this
    // environment before the failover tests depend on it.
    let address = "192.0.2.99/32";

    let add = Command::new("ip")
        .args(["addr", "add", address, "dev", "lo"])
        .status()
        .expect("failed to run ip addr add");
    assert!(add.success(), "failed to add the VIP to the loopback interface");

    let listed = Command::new("ip")
        .args(["addr", "show", "dev", "lo"])
        .output()
        .expect("failed to run ip addr show");
    let listed = String::from_utf8_lossy(&listed.stdout);
    assert!(
        listed.contains("192.0.2.99"),
        "the VIP is not listed on the loopback interface:\n{listed}"
    );

    let remove = Command::new("ip")
        .args(["addr", "del", address, "dev", "lo"])
        .status()
        .expect("failed to run ip addr del");
    assert!(remove.success(), "failed to remove the VIP from the loopback interface");
}
