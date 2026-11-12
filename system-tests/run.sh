#!/usr/bin/env bash
#
# Runs the privileged Linux system tests.
#
# These tests need CAP_NET_ADMIN to create network namespaces and to move a VIP between
# namespaces. They are the only way to prove the failover requirements (G3) without two
# physical hosts.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "${ROOT}/system-tests"

if [[ "${EUID}" -ne 0 ]]; then
  echo "error: the system tests need root" >&2
  echo "       run them with: sudo ./system-tests/run.sh" >&2
  exit 1
fi

if ! command -v ip >/dev/null 2>&1; then
  echo "error: iproute2 is not installed" >&2
  echo "       debian/ubuntu: sudo apt-get install -y iproute2" >&2
  echo "       fedora/rhel:   sudo dnf install -y iproute" >&2
  exit 1
fi

if ! command -v unshare >/dev/null 2>&1; then
  echo "error: 'unshare' is not available — install util-linux" >&2
  exit 1
fi

echo "==> Running the system tests"
cargo test -- --test-threads=1 "$@"
