#!/usr/bin/env bash
#
# Verifies that every third-party dependency in the workspace appears in
# plan/09-dependencies.md, and that every crate listed there is actually used.
#
# A dependency that is not in the manifest is invisible to the licence and advisory review, so
# this check is what keeps the manifest honest.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
MANIFEST="${ROOT}/plan/09-dependencies.md"

if [[ ! -f "${MANIFEST}" ]]; then
  echo "error: ${MANIFEST} is missing" >&2
  exit 1
fi

cd "${ROOT}"

# Every direct dependency of every workspace member.
mapfile -t USED < <(
  cargo metadata --no-deps --format-version 1 \
    | jq -r '.packages[] | .dependencies[] | select(.kind == null) | .name' \
    | sort -u
)

# Every crate named in the manifest's dependency tables.
mapfile -t DECLARED < <(
  grep -oE '^\| `[a-z0-9_-]+`' "${MANIFEST}" \
    | tr -d '|` ' \
    | sort -u
)

status=0

for crate in "${USED[@]}"; do
  # Workspace-internal crates are not third-party dependencies.
  if [[ "${crate}" == orchestrator-* ]]; then
    continue
  fi
  # `printf` rather than a here-string: the array must stay one entry per line, or the
  # whole-line match never succeeds.
  if ! printf '%s\n' "${DECLARED[@]}" | grep -qx "${crate}"; then
    echo "error: the dependency '${crate}' is not listed in plan/09-dependencies.md" >&2
    status=1
  fi
done

if [[ ${status} -eq 0 ]]; then
  echo "ok: every third-party dependency is listed in plan/09-dependencies.md"
fi

exit ${status}
