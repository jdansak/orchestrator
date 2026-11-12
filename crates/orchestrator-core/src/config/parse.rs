//! Parsing configuration files into the [`TenantConfig`](crate::config::model::TenantConfig) model.

use std::path::Path;

use serde::de::DeserializeOwned;

use crate::config::model::TenantConfig;
use crate::error::ConfigError;
use crate::version::{MAX_SCHEMA_VERSION, MIN_SCHEMA_VERSION};

/// Parses a tenant configuration from a YAML string.
///
/// The schema version is checked **before** the rest of the document is validated, so a config
/// written for a newer schema fails with a clear message instead of a confusing field error.
pub fn parse_tenant_config(yaml: &str, path: &str) -> Result<TenantConfig, ConfigError> {
    let value: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(yaml).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source,
        })?;

    check_schema_version(&value)?;

    serde_yaml_ng::from_value(value).map_err(|source| ConfigError::Parse {
        path: path.to_string(),
        source,
    })
}

/// Reads and parses a tenant configuration from a file.
pub fn load_tenant_config(path: impl AsRef<Path>) -> Result<TenantConfig, ConfigError> {
    let path = path.as_ref();
    let display = path.display().to_string();
    let contents = std::fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: display.clone(),
        source,
    })?;
    parse_tenant_config(&contents, &display)
}

/// Checks that the schema version in the document is one this build supports.
fn check_schema_version(value: &serde_yaml_ng::Value) -> Result<(), ConfigError> {
    let Some(mapping) = value.as_mapping() else {
        return Ok(());
    };
    let Some(version) = mapping.get(serde_yaml_ng::Value::from("version")) else {
        return Ok(());
    };
    let Some(version) = version.as_u64() else {
        return Ok(());
    };

    let version = u32::try_from(version).unwrap_or(u32::MAX);
    if version > MAX_SCHEMA_VERSION {
        return Err(ConfigError::UnsupportedVersion {
            found: version,
            supported: MAX_SCHEMA_VERSION,
        });
    }
    if version < MIN_SCHEMA_VERSION {
        return Err(ConfigError::UnsupportedVersion {
            found: version,
            supported: MIN_SCHEMA_VERSION,
        });
    }
    Ok(())
}

/// Parses any configuration document that implements [`DeserializeOwned`].
pub fn parse_yaml<T: DeserializeOwned>(yaml: &str, path: &str) -> Result<T, ConfigError> {
    serde_yaml_ng::from_str(yaml).map_err(|source| ConfigError::Parse {
        path: path.to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MINIMAL: &str = r"
version: 1
tenant:
  name: app-frontend
  zone: app-frontend.internal
vip:
  address: 10.0.0.100/32
  interface: eth0
hierarchy:
  regions:
    - name: us-east
      weight: 100
      datacenters:
        - name: dc-1
          weight: 100
          clusters:
            - name: prod
              weight: 100
              endpoints:
                - name: ep-a
                  weight: 100
                  address: 10.1.1.5:8080
";

    #[test]
    fn parses_a_minimal_config() {
        let config = parse_tenant_config(MINIMAL, "test.yaml").expect("should parse");
        assert_eq!(config.version, 1);
        assert_eq!(config.tenant.name, "app-frontend");
        assert_eq!(config.hierarchy.regions.len(), 1);
    }

    #[test]
    fn applies_defaults() {
        let config = parse_tenant_config(MINIMAL, "test.yaml").expect("should parse");
        assert_eq!(config.upstream.retries, 1);
        assert_eq!(config.upstream.health_check.healthy_threshold, 2);
        assert_eq!(config.dns.ttl, 60);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn rejects_unknown_fields() {
        let yaml = format!("{MINIMAL}\nnot_a_real_field: 1\n");
        let error = parse_tenant_config(&yaml, "test.yaml").unwrap_err();
        assert!(
            matches!(error, ConfigError::Parse { .. }),
            "expected a parse error, got {error:?}"
        );
    }

    #[test]
    fn rejects_a_newer_schema_version() {
        let yaml = MINIMAL.replace("version: 1", "version: 999");
        let error = parse_tenant_config(&yaml, "test.yaml").unwrap_err();
        assert!(
            matches!(
                error,
                ConfigError::UnsupportedVersion {
                    found: 999,
                    supported: MAX_SCHEMA_VERSION
                }
            ),
            "expected an unsupported-version error, got {error:?}"
        );
    }

    #[test]
    fn parses_durations_from_strings() {
        let yaml = format!("{MINIMAL}\nupstream:\n  connect_timeout: 3s\n");
        let config = parse_tenant_config(&yaml, "test.yaml").expect("should parse");
        assert_eq!(config.upstream.connect_timeout.as_duration().as_secs(), 3);
    }

    #[test]
    fn rejects_bare_integer_durations() {
        let yaml = format!("{MINIMAL}\nupstream:\n  connect_timeout: 3\n");
        assert!(parse_tenant_config(&yaml, "test.yaml").is_err());
    }
}
