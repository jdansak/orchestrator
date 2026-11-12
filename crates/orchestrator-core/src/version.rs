//! Version constants for the configuration schema and the product.

/// The configuration schema version this build understands.
///
/// A config declaring a `version` greater than this is refused rather than partially applied —
/// see `plan/07-config-schema.md` §1.
pub const SCHEMA_VERSION: u32 = 1;

/// The newest schema version this build understands. Equivalent to [`SCHEMA_VERSION`].
pub const MAX_SCHEMA_VERSION: u32 = SCHEMA_VERSION;

/// The oldest schema version this build can still read.
pub const MIN_SCHEMA_VERSION: u32 = 1;

/// The product version, taken from `Cargo.toml`.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The git commit this binary was built from, when the build system provided one.
pub const GIT_SHA: &str = match option_env!("ORCHESTRATOR_GIT_SHA") {
    Some(sha) => sha,
    None => "unknown",
};
