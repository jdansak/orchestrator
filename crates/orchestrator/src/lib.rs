//! The Orchestrator per-tenant process.
//!
//! Each tenant runs as its own process with its own configuration, VIP, listeners, and resource
//! limits. A crash in one tenant cannot affect any other.
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`cli`] | Command-line arguments |
//! | [`control_plane`] | Configuration, health, VIP management, rollouts |
//! | [`data_plane`] | The HTTP/HTTPS, TCP, and UDP proxies |
//! | [`runtime`] | The tokio runtime and signal handling |

#![deny(missing_docs)]

pub mod cli;
pub mod control_plane;
pub mod data_plane;
pub mod runtime;
