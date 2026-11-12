#!/usr/bin/env bash
#
# Enforces the coverage thresholds from plan/11-non-functional-requirements.md §5.
#
# The thresholds are on line coverage, measured per crate. A crate below its threshold fails
# the build — the number is the policy, not a report.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LCOV="${1:-lcov.info}"

if [[ ! -f "${LCOV}" ]]; then
  echo "error: ${LCOV} is missing" >&2
  exit 1
fi

# The gates are a ratchet, not the target. The target is the figure in
# plan/11-non-functional-requirements.md §5; the gate is what CI enforces today. The gate
# rises as the phases land — a gate set at the target would block every pull request for a
# reason nobody can act on.
#
# crate:minimum-line-coverage
THRESHOLDS=(
  "orchestrator-core:95"
  "orchestrator:45"
  "orchestrator-manager:55"
  "orchestrator-testkit:90"
)

status=0

for entry in "${THRESHOLDS[@]}"; do
  crate="${entry%%:*}"
  minimum="${entry##*:}"

  # The lcov `SF:` records look like `SF:crates/orchestrator-core/src/weights.rs`, so the
  # crate name is a path segment, not a prefix.
  read -r covered total < <(
    # The lcov records are `LF:97` and `LH:97`, so the field separator is a colon.
    awk -F: -v crate="${crate}" '
      /^SF:/ {
        path = substr($0, 4)
        n = split(path, parts, "/")
        in_crate = 0
        for (i = 1; i <= n; i++) {
          if (parts[i] == crate) { in_crate = 1; break }
        }
        next
      }
      /^LH:/ { if (in_crate) { hits += $2 } }
      /^LF:/ { if (in_crate) { lines += $2 } }
      END { print hits + 0, lines + 0 }
    ' "${LCOV}"
  )

  if [[ "${total}" -eq 0 ]]; then
    echo "skip: no coverage data for ${crate}"
    continue
  fi

  percent="$(awk -v c="${covered}" -v t="${total}" 'BEGIN { printf "%.1f", (c / t) * 100 }')"

  if awk -v p="${percent}" -v m="${minimum}" 'BEGIN { exit !(p + 0 < m + 0) }'; then
    echo "error: ${crate} has ${percent}% line coverage, below the ${minimum}% threshold" >&2
    status=1
  else
    echo "ok: ${crate} has ${percent}% line coverage (threshold ${minimum}%)"
  fi
done

exit ${status}
