//! Error types for the Orchestrator configuration model.
//!
//! Every error carries enough context to diagnose the problem from the log line alone, which is
//! the rule from `plan/04-code-standards.md`.

use std::fmt;

/// A single problem found while validating a configuration.
///
/// The validator collects every problem rather than stopping at the first, so an operator fixing
/// a config sees the whole list at once.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// The path to the offending value, e.g. `hierarchy.regions[0].weight`.
    pub field: String,
    /// A human-readable explanation of what is wrong.
    pub message: String,
}

impl ValidationIssue {
    /// Creates a new issue.
    pub fn new(field: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// The result of validating a configuration.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationErrors {
    issues: Vec<ValidationIssue>,
}

impl ValidationErrors {
    /// Creates an empty error set.
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    /// Creates an error set from a list of issues.
    pub fn from_issues(issues: Vec<ValidationIssue>) -> Self {
        Self { issues }
    }

    /// Records an issue.
    pub fn push(&mut self, field: impl Into<String>, message: impl Into<String>) {
        self.issues.push(ValidationIssue::new(field, message));
    }

    /// Returns `true` if no issues were recorded.
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Returns the number of issues.
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Returns the recorded issues.
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Merges another error set into this one.
    pub fn extend(&mut self, other: Self) {
        self.issues.extend(other.issues);
    }
}

impl fmt::Display for ValidationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "{} validation error{}:",
            self.issues.len(),
            if self.issues.len() == 1 { "" } else { "s" }
        )?;
        for issue in &self.issues {
            writeln!(f, "  - {issue}")?;
        }
        Ok(())
    }
}

impl std::error::Error for ValidationErrors {}

/// Errors produced while loading, parsing, or validating a configuration.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// The file could not be read.
    #[error("failed to read config file '{path}': {source}")]
    Io {
        /// The path that could not be read.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// The file was not valid YAML, or did not match the schema shape.
    #[error("failed to parse config file '{path}': {source}")]
    Parse {
        /// The path that failed to parse.
        path: String,
        /// The underlying YAML error. Boxed — the error type is large.
        #[source]
        source: Box<serde_saphyr::Error>,
    },

    /// The config parsed but failed validation.
    #[error("config validation failed:\n{0}")]
    Validation(#[from] ValidationErrors),

    /// The config declares a schema version this build does not understand.
    #[error("config schema version {found} is newer than the supported version {supported}")]
    UnsupportedVersion {
        /// The version found in the config.
        found: u32,
        /// The highest version this build understands.
        supported: u32,
    },

    /// The config exceeds a documented limit.
    #[error("config exceeds the limit for {what}: {count} > {max}")]
    LimitExceeded {
        /// What was exceeded, e.g. `routes`.
        what: String,
        /// The observed count.
        count: usize,
        /// The maximum allowed.
        max: usize,
    },
}

/// Errors from the rolling-update engine.
#[derive(Debug, thiserror::Error)]
pub enum RolloutError {
    /// A rollout is already running for this route.
    #[error("a rollout is already in progress for route '{route}'")]
    AlreadyRunning {
        /// The route that already has a rollout.
        route: String,
    },

    /// No rollout exists for the requested route.
    #[error("no rollout exists for route '{route}'")]
    NotFound {
        /// The route that has no rollout.
        route: String,
    },

    /// The requested transition is not legal from the current state.
    #[error("cannot {action} a rollout in state '{state}'")]
    InvalidTransition {
        /// The action that was attempted.
        action: String,
        /// The state the rollout is in.
        state: String,
    },

    /// The rollout configuration is not valid.
    #[error("invalid rollout configuration: {0}")]
    InvalidConfig(String),
}

/// Errors from the VIP manager.
#[derive(Debug, thiserror::Error)]
pub enum VipError {
    /// The platform does not support VIP manipulation.
    #[error("VIP management is only supported on Linux")]
    UnsupportedPlatform,

    /// The interface named in the config does not exist.
    #[error("network interface '{interface}' does not exist")]
    InterfaceNotFound {
        /// The missing interface.
        interface: String,
    },

    /// The address is already present on the interface.
    #[error("VIP {address} is already present on interface '{interface}'")]
    AlreadyPresent {
        /// The address that is already present.
        address: String,
        /// The interface it is present on.
        interface: String,
    },

    /// The address could not be added.
    #[error("failed to add VIP {address} to interface '{interface}': {source}")]
    AddFailed {
        /// The address being added.
        address: String,
        /// The interface being used.
        interface: String,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },

    /// The address could not be removed.
    #[error("failed to release VIP {address} from interface '{interface}': {source}")]
    RemoveFailed {
        /// The address being removed.
        address: String,
        /// The interface being used.
        interface: String,
        /// The underlying error.
        #[source]
        source: std::io::Error,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_errors_render_all_issues() {
        let mut errors = ValidationErrors::new();
        errors.push("routes[0].name", "must not be empty");
        errors.push("hierarchy.regions[0].weight", "must sum to 100");

        let rendered = errors.to_string();
        assert!(rendered.contains("2 validation errors"));
        assert!(rendered.contains("routes[0].name"));
        assert!(rendered.contains("hierarchy.regions[0].weight"));
    }

    #[test]
    fn validation_errors_empty_is_empty() {
        assert!(ValidationErrors::new().is_empty());
        assert_eq!(ValidationErrors::new().len(), 0);
    }

    #[test]
    fn validation_errors_extend_merges() {
        let mut a = ValidationErrors::new();
        a.push("a", "first");
        let mut b = ValidationErrors::new();
        b.push("b", "second");
        a.extend(b);
        assert_eq!(a.len(), 2);
    }

    #[test]
    fn a_single_issue_is_rendered_in_the_singular() {
        let mut errors = ValidationErrors::new();
        errors.push("a", "first");
        assert!(errors.to_string().contains("1 validation error:"));
    }

    #[test]
    fn the_other_errors_render() {
        let io = ConfigError::Io {
            path: "/tmp/tenant.yaml".to_string(),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        assert!(io.to_string().contains("/tmp/tenant.yaml"));

        let version = ConfigError::UnsupportedVersion {
            found: 9,
            supported: 1,
        };
        assert!(version.to_string().contains('9'));
        assert!(version.to_string().contains("newer"));

        let limit = ConfigError::LimitExceeded {
            what: "routes".to_string(),
            count: 2_000,
            max: 1_000,
        };
        assert!(limit.to_string().contains("routes"));

        let rollout = RolloutError::AlreadyRunning {
            route: "api".to_string(),
        };
        assert!(rollout.to_string().contains("api"));

        let rollout = RolloutError::NotFound {
            route: "api".to_string(),
        };
        assert!(rollout.to_string().contains("api"));

        let rollout = RolloutError::InvalidTransition {
            action: "advance".to_string(),
            state: "completed".to_string(),
        };
        assert!(rollout.to_string().contains("advance"));

        let rollout = RolloutError::InvalidConfig("bad steps".to_string());
        assert!(rollout.to_string().contains("bad steps"));

        let vip = VipError::UnsupportedPlatform;
        assert!(vip.to_string().contains("Linux"));

        let vip = VipError::InterfaceNotFound {
            interface: "eth9".to_string(),
        };
        assert!(vip.to_string().contains("eth9"));

        let vip = VipError::AlreadyPresent {
            address: "10.0.0.100/32".to_string(),
            interface: "eth0".to_string(),
        };
        assert!(vip.to_string().contains("10.0.0.100/32"));
    }
}
