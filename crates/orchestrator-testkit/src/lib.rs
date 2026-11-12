//! Shared test fixtures for Orchestrator.
//!
//! This crate is a **dev-dependency** only. It never ships in the production binaries.
//!
//! | Fixture | Purpose |
//! |---------|---------|
//! | [`mock_backend`] | A real HTTP server that records every request and can simulate latency, failures, and death |
//! | [`load_generator`] | Concurrent traffic generation and statistically sound distribution assertions |
//! | [`test_config`] | A builder that produces valid configurations without hand-writing YAML |
//! | [`fake_clock`] | A controllable clock, so no test sleeps for real time |
//!
//! The Linux-only network-namespace cluster fixture lands in Phase 2 task 2.8.

#![deny(missing_docs)]

pub mod fake_clock;
pub mod load_generator;
pub mod mock_backend;
pub mod test_config;

pub use fake_clock::FakeClock;
pub use load_generator::{assert_binomial, assert_within, LoadGenerator, LoadResult};
pub use mock_backend::{MockBackend, RecordedRequest, ResponseSpec};
pub use test_config::TestConfigBuilder;
