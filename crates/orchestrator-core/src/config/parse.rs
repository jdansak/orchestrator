//! Parsing configuration files into the [`TenantConfig`](crate::config::model::TenantConfig) model.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::config::model::TenantConfig;
use crate::error::ConfigError;
use crate::version::{MAX_SCHEMA_VERSION, MIN_SCHEMA_VERSION};

/// Reads only the schema version from a document.
///
/// The version is checked before the rest of the document is validated, so a config written for
/// a newer schema fails with a clear message instead of a confusing field error.
#[derive(Debug, Deserialize)]
struct VersionProbe {
    /// The declared schema version, if the document has one.
    #[serde(default)]
    version: Option<u32>,
}

/// Parses a tenant configuration from a YAML string.
///
/// The schema version is checked **before** the rest of the document is validated, so a config
/// written for a newer schema fails with a clear message instead of a confusing field error.
pub fn parse_tenant_config(yaml: &str, path: &str) -> Result<TenantConfig, ConfigError> {
    let probe: VersionProbe =
        serde_saphyr::from_str(yaml).map_err(|source| ConfigError::Parse {
            path: path.to_string(),
            source: Box::new(source),
        })?;

    if let Some(version) = probe.version {
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
    }

    serde_saphyr::from_str(yaml).map_err(|source| ConfigError::Parse {
        path: path.to_string(),
        source: Box::new(source),
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

/// Parses any configuration document that implements [`DeserializeOwned`].
pub fn parse_yaml<T: DeserializeOwned>(yaml: &str, path: &str) -> Result<T, ConfigError> {
    serde_saphyr::from_str(yaml).map_err(|source| ConfigError::Parse {
        path: path.to_string(),
        source: Box::new(source),
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

    #[test]
    fn loads_a_config_from_a_file() {
        let directory = tempfile::tempdir().expect("a temporary directory");
        let path = directory.path().join("tenant.yaml");
        std::fs::write(&path, MINIMAL).expect("the config is writable");

        let config = load_tenant_config(&path).expect("the config loads");
        assert_eq!(config.tenant.name, "app-frontend");
    }

    #[test]
    fn a_missing_file_is_an_io_error() {
        let error = load_tenant_config("/nonexistent/tenant.yaml").unwrap_err();
        assert!(matches!(error, ConfigError::Io { .. }), "got {error:?}");
    }

    #[test]
    fn a_missing_file_is_not_a_parse_error() {
        let error = load_tenant_config("/nonexistent/tenant.yaml").unwrap_err();
        assert!(error.to_string().contains("failed to read config file"));
    }

    #[test]
    fn malformed_yaml_is_a_parse_error() {
        let error = parse_tenant_config("version: [unclosed", "test.yaml").unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "got {error:?}");
    }

    #[test]
    fn a_document_that_is_not_a_mapping_is_a_parse_error() {
        let error = parse_tenant_config(
            "- just
- a list\n",
            "test.yaml",
        )
        .unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "got {error:?}");
    }

    #[test]
    fn a_missing_version_is_rejected() {
        // The schema version is required. A config without one is from a tool that predates
        // the versioning, and guessing which version it meant is worse than refusing it.
        let yaml = MINIMAL.replace("version: 1\n", "");
        let error = parse_tenant_config(&yaml, "test.yaml").unwrap_err();
        assert!(matches!(error, ConfigError::Parse { .. }), "got {error:?}");
    }

    #[test]
    fn an_older_version_is_rejected() {
        let yaml = MINIMAL.replace("version: 1", "version: 0");
        let error = parse_tenant_config(&yaml, "test.yaml").unwrap_err();
        assert!(
            matches!(error, ConfigError::UnsupportedVersion { .. }),
            "got {error:?}"
        );
    }

    #[test]
    fn a_non_numeric_version_is_a_parse_error() {
        let yaml = MINIMAL.replace("version: 1", "version: \"one\"");
        assert!(parse_tenant_config(&yaml, "test.yaml").is_err());
    }

    #[test]
    fn parse_yaml_handles_any_document() {
        #[derive(Debug, serde::Deserialize, PartialEq)]
        struct Simple {
            name: String,
        }

        let parsed: Simple = parse_yaml("name: test\n", "test.yaml").expect("should parse");
        assert_eq!(
            parsed,
            Simple {
                name: "test".to_string()
            }
        );
    }

    #[test]
    fn rejects_duplicate_keys() {
        // A duplicate key is ambiguous, and a traffic manager must not guess which one the
        // operator meant.
        let yaml = format!("{MINIMAL}\nversion: 2\n");
        assert!(parse_tenant_config(&yaml, "test.yaml").is_err());
    }
}
