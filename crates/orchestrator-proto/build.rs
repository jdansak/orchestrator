//! Generates the Rust bindings for the worker sync protocol.
//!
//! `protoc` is vendored so the build does not depend on a system installation.

// `set_var` is the only way to point prost-build at a vendored protoc, and build
// scripts are single-threaded, so this is safe here.
#[allow(clippy::disallowed_methods)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    std::env::set_var("PROTOC", protoc);

    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/sync.proto"], &["proto"])?;

    println!("cargo:rerun-if-changed=proto/sync.proto");

    Ok(())
}
