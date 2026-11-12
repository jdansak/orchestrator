#!/usr/bin/env bash
#
# Enforces the coverage thresholds from plan/11-non-functional-requirements.md §5.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
LCOV="${1:-lcov.info}"

if [[ ! -f "${LCOV}" ]]; then
  echo "error: ${LCOV} is missing" >&2
  exit 1
fi

# crate:minimum-line-coverage
THRESHOLDS=(
  "orchestrator-core:95"
  "orchestrator:80"
  "orchestrator-manager:70"
  "orchestrator-testkit:70"
)

status=0

for entry in "${THRESHOLDS[@]}"; do
  crate="${entry%%:*}"
  minimum="${entry##*:}"

  # Sum the covered and total line counts for the crate's files.
  covered="$(awk -v crate="${crate}" '
    /^SF:/ { in_crate = ($0 ~ ("/" crate "-")); next }
    /^LH:/ { if (in_crate) { hits += $2 } }
    /^LF:/ { if (in_crate) { total += $2 } }
    END { print hits + 0 }
  ' "${LCOV}")"

  total="$(awk -v crate="${crate}" '
    /^SF:/ { in_crate = ($0 ~ ("/" crate "-")); next }
    /^LF:/ { if (in_crate) { total += $2 } }
    END { print total + 0 }
  ' "${LCOV}")"

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
