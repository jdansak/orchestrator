#!/usr/bin/env bash
#
# Sets up a development environment and verifies the build.
#
# This is the one command a new contributor needs.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}"

info() { printf '\033[36m==>\033[0m %s\n' "$*"; }
fail() { printf '\033[31merror:\033[0m %s\n' "$*" >&2; exit 1; }

info "Checking the toolchain"

command -v cargo >/dev/null 2>&1 || fail "cargo is not installed — see https://rustup.rs"
info "rustc $(rustc --version)"

if command -v node >/dev/null 2>&1; then
  info "node $(node --version)"
else
  info "node is not installed — skipping the GUI setup"
fi

info "Fetching the Rust dependencies"
cargo fetch

info "Building the workspace"
cargo build --workspace

info "Running the tests"
cargo test --workspace

info "Checking the formatting and the lints"
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings

if command -v node >/dev/null 2>&1; then
  info "Setting up the GUI"
  npm --prefix orchestrator-gui ci --no-audit --no-fund
  npm --prefix orchestrator-gui run build
fi

cat <<'EOF'

Ready.

  Validate the example configuration:
    cargo run -p orchestrator -- --config examples/tenant.yaml --check

  Print the resolved routing table:
    cargo run -p orchestrator -- --config examples/tenant.yaml --print-routes

  Start the GUI:
    make gui

  Run everything CI runs:
    make check

EOF
