#!/usr/bin/env bash
#
# Regenerates plan/DESIGN_SPEC.md from the numbered design documents.
#
# The numbered documents in plan/ are the source of truth. DESIGN_SPEC.md is their
# concatenation, so that there is one file to read or share. It is generated, never edited by
# hand — editing it directly means the next regeneration silently discards the change.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PLAN="${ROOT}/plan"
OUTPUT="${PLAN}/DESIGN_SPEC.md"

if [[ ! -d "${PLAN}" ]]; then
  echo "error: ${PLAN} is missing" >&2
  exit 1
fi

DOCUMENTS=(
  01-project-overview.md
  02-goals-and-acceptance-criteria.md
  03-architecture.md
  04-code-standards.md
  05-implementation-plan.md
  06-tracker.md
  07-config-schema.md
  08-api-reference.md
  09-dependencies.md
  10-test-strategy.md
  11-non-functional-requirements.md
  12-repo-layout-and-tooling.md
)

for document in "${DOCUMENTS[@]}"; do
  if [[ ! -f "${PLAN}/${document}" ]]; then
    echo "error: ${PLAN}/${document} is missing" >&2
    exit 1
  fi
done

TEMP="$(mktemp "${TMPDIR:-/tmp}/design-spec.XXXXXX")"
trap 'rm -f "${TEMP}"' EXIT

cat >"${TEMP}" <<'EOF'
# Orchestrator — Complete Design Specification

> **This file is generated. Do not edit it.**
>
> It is the concatenation of the numbered design documents in [`plan/`](.), which are the
> source of truth. To change the design, edit the numbered document and run
> `make design-spec`.
>
> | Document | Subject |
> |----------|---------|
EOF

for document in "${DOCUMENTS[@]}"; do
  title="$(head -1 "${PLAN}/${document}" | sed 's/^# //')"
  printf '> | [`%s`](%s) | %s |\n' "${document}" "${document}" "${title#Orchestrator — }" >>"${TEMP}"
done

cat >>"${TEMP}" <<'EOF'

---

EOF

for document in "${DOCUMENTS[@]}"; do
  # Strip the top-level heading so the document's own title does not break the outline.
  tail -n +2 "${PLAN}/${document}" >>"${TEMP}"
  printf '\n\n---\n\n' >>"${TEMP}"
done

mv "${TEMP}" "${OUTPUT}"
trap - EXIT

echo "wrote ${OUTPUT} from ${#DOCUMENTS[@]} documents"
