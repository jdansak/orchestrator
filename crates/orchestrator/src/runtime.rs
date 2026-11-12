//! Tokio runtime setup and signal handling.

use tracing_subscriber::EnvFilter;

/// Builds the tracing subscriber from the log level and format.
///
/// The format is `json` (the default for production) or `compact` (easier to read in a
/// terminal).
pub fn init_tracing(level: &str, format: &str) {
    let filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new(level))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let builder = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(true);

    if format.eq_ignore_ascii_case("json") {
        builder.json().init();
    } else {
        builder.compact().init();
    }
}
