//! The tenant registry.
//!
//! The registry tells the manager which tenants exist on this host, what VIP each one owns,
//! which ports it binds, and what resource limits apply. The format is specified in
//! `plan/07-config-schema.md` §8.

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// The registry file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    /// The registry schema version.
    #[serde(default = "default_version")]
    pub version: u32,

    /// Defaults applied to every tenant that does not override them.
    #[serde(default)]
    pub defaults: RegistryDefaults,

    /// The tenants on this host.
    #[serde(default)]
    pub tenants: Vec<TenantEntry>,
}

/// Defaults applied to tenants that do not override them.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryDefaults {
    /// The default network interface.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub interface: Option<String>,

    /// The default resource limits.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceLimits>,

    /// The default TLS material locations.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tls: Option<RegistryTlsDefaults>,
}

/// Where TLS material lives by default.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegistryTlsDefaults {
    /// A template for the certificate directory. `{tenant}` is substituted.
    pub cert_dir: String,
}

/// The resource limits applied to a tenant's systemd unit.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceLimits {
    /// The `MemoryMax` value, e.g. `512M`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_max: Option<String>,

    /// The `CPUQuota` value, e.g. `100%`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_quota: Option<String>,
}

/// One tenant in the registry.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TenantEntry {
    /// The tenant's name. Must match `tenant.name` in the tenant's config.
    pub name: String,

    /// Whether the tenant should be running.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// The path to the tenant's `tenant.yaml`.
    pub config: PathBuf,

    /// The VIP the tenant owns.
    pub vip: String,

    /// The network interface the VIP is added to.
    pub interface: String,

    /// The tenant's workers. Two entries make an active/passive pair.
    #[serde(default)]
    pub workers: Vec<WorkerEntry>,

    /// The resource limits for this tenant.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resources: Option<ResourceLimits>,
}

/// One worker of a tenant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerEntry {
    /// The worker's stable identifier.
    pub node_id: String,

    /// The worker's host address.
    pub host: String,

    /// The ports the worker binds.
    pub ports: WorkerPorts,
}

/// The ports a worker binds.
///
/// Every tenant on a host needs its own set — a hardcoded metrics port would collide across
/// tenants. The manager validates for conflicts before starting anything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WorkerPorts {
    /// The plaintext HTTP listener.
    pub http: u16,
    /// The TLS listener.
    pub https: u16,
    /// The Prometheus exporter.
    pub metrics: u16,
    /// The gRPC sync channel.
    pub sync: u16,
    /// The DNS server.
    pub dns: u16,
}

impl WorkerPorts {
    /// Returns every port, for conflict detection.
    pub fn all(&self) -> [u16; 5] {
        [self.http, self.https, self.metrics, self.sync, self.dns]
    }
}

/// A problem found while loading or validating the registry.
#[derive(Debug, thiserror::Error)]
pub enum RegistryError {
    /// The registry file could not be read.
    #[error("failed to read the registry '{path}': {source}")]
    Io {
        /// The path that could not be read.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The registry file was not valid YAML.
    #[error("failed to parse the registry '{path}': {source}")]
    Parse {
        /// The path that failed to parse.
        path: String,
        /// The underlying YAML error.
        #[source]
        source: serde_yaml_ng::Error,
    },

    /// Two workers on the same host bind the same port.
    #[error("port {port} is claimed by both '{first}' and '{second}' on host '{host}'")]
    PortConflict {
        /// The contested port.
        port: u16,
        /// The first claimant.
        first: String,
        /// The second claimant.
        second: String,
        /// The host they share.
        host: String,
    },

    /// A tenant name appears twice.
    #[error("duplicate tenant name '{name}'")]
    DuplicateTenant {
        /// The repeated name.
        name: String,
    },
}

impl Registry {
    /// Reads and parses a registry file.
    pub fn load(path: impl AsRef<std::path::Path>) -> Result<Self, RegistryError> {
        let path = path.as_ref();
        let display = path.display().to_string();
        let contents = std::fs::read_to_string(path).map_err(|source| RegistryError::Io {
            path: display.clone(),
            source,
        })?;
        serde_yaml_ng::from_str(&contents).map_err(|source| RegistryError::Parse {
            path: display,
            source,
        })
    }

    /// Checks the registry for problems that would prevent it from being used.
    ///
    /// This is the check that catches the port collision the manager would otherwise hit at
    /// start-up, when the failure is much harder to diagnose.
    pub fn validate(&self) -> Result<(), RegistryError> {
        let mut names = HashSet::new();
        for tenant in &self.tenants {
            if !names.insert(tenant.name.clone()) {
                return Err(RegistryError::DuplicateTenant {
                    name: tenant.name.clone(),
                });
            }
        }

        // Port conflicts are per-host: two tenants on different hosts may use the same port.
        let mut claimed: std::collections::HashMap<(String, u16), String> =
            std::collections::HashMap::new();

        for tenant in &self.tenants {
            for worker in &tenant.workers {
                for port in worker.ports.all() {
                    let key = (worker.host.clone(), port);
                    if let Some(existing) = claimed.get(&key) {
                        return Err(RegistryError::PortConflict {
                            port,
                            first: existing.clone(),
                            second: format!("{}/{}", tenant.name, worker.node_id),
                            host: worker.host.clone(),
                        });
                    }
                    claimed.insert(key, format!("{}/{}", tenant.name, worker.node_id));
                }
            }
        }

        Ok(())
    }
}

fn default_version() -> u32 {
    1
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn worker(node_id: &str, host: &str, http: u16) -> WorkerEntry {
        WorkerEntry {
            node_id: node_id.to_string(),
            host: host.to_string(),
            ports: WorkerPorts {
                http,
                https: http + 1,
                metrics: http + 2,
                sync: http + 3,
                dns: http + 4,
            },
        }
    }

    fn tenant(name: &str, workers: Vec<WorkerEntry>) -> TenantEntry {
        TenantEntry {
            name: name.to_string(),
            enabled: true,
            config: PathBuf::from(format!("/etc/orchestrator/tenants/{name}/tenant.yaml")),
            vip: "10.0.0.100/32".to_string(),
            interface: "eth0".to_string(),
            workers,
            resources: None,
        }
    }

    #[test]
    fn a_valid_registry_passes() {
        let registry = Registry {
            version: 1,
            defaults: RegistryDefaults::default(),
            tenants: vec![tenant("a", vec![worker("w1", "10.0.0.1", 18080)])],
        };
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn a_port_conflict_is_detected() {
        let registry = Registry {
            version: 1,
            defaults: RegistryDefaults::default(),
            tenants: vec![
                tenant("a", vec![worker("w1", "10.0.0.1", 18080)]),
                tenant("b", vec![worker("w2", "10.0.0.1", 18080)]),
            ],
        };
        let error = registry.validate().unwrap_err();
        assert!(matches!(
            error,
            RegistryError::PortConflict { port: 18080, .. }
        ));
    }

    #[test]
    fn the_same_port_on_different_hosts_is_fine() {
        let registry = Registry {
            version: 1,
            defaults: RegistryDefaults::default(),
            tenants: vec![
                tenant("a", vec![worker("w1", "10.0.0.1", 18080)]),
                tenant("b", vec![worker("w2", "10.0.0.2", 18080)]),
            ],
        };
        assert!(registry.validate().is_ok());
    }

    #[test]
    fn duplicate_tenant_names_are_detected() {
        let registry = Registry {
            version: 1,
            defaults: RegistryDefaults::default(),
            tenants: vec![
                tenant("a", vec![worker("w1", "10.0.0.1", 18080)]),
                tenant("a", vec![worker("w2", "10.0.0.2", 18090)]),
            ],
        };
        assert!(matches!(
            registry.validate().unwrap_err(),
            RegistryError::DuplicateTenant { .. }
        ));
    }
}
