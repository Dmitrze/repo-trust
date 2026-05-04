#!/usr/bin/env bash
#
# run-benchmarks.sh — Repo Trust real-API benchmark sweep.
#
# Reads examples/benchmark-set.csv, runs `repo-trust scan` against each
# reference repository using the live GitHub / Scorecard / deps.dev / OSV
# APIs, and emits a GitHub-flavored markdown summary table at
# ./benchmark-output/SUMMARY.md.
#
# Usage:
#   export GITHUB_TOKEN=ghp_...
#   ./scripts/run-benchmarks.sh
#   open ./benchmark-output/SUMMARY.md
#
# Prerequisites:
#   - GITHUB_TOKEN env var set (passed through to the binary).
#   - `cargo` on PATH; release build will be triggered by `cargo run --release`.
#   - `jq` on PATH for JSON extraction. (Install: `brew install jq` /
#     `apt-get install jq`.)
#
# Output:
#   ./benchmark-output/<owner>_<name>.json   per-repo Trust Report
#   ./benchmark-output/SUMMARY.md            aggregated comparison table
#
# Exit code:
#   0  all scans succeeded
#   1  one or more scans failed (the script keeps going so partial results
#      are still written, then exits non-zero at the end)

set -euo pipefail

# ─── Resolve repo root ───────────────────────────────────────────────────────
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
cd "${REPO_ROOT}"

CSV="examples/benchmark-set.csv"
OUT_DIR="./benchmark-output"
SUMMARY="${OUT_DIR}/SUMMARY.md"

# ─── Preflight ───────────────────────────────────────────────────────────────
if [[ ! -f "${CSV}" ]]; then
  echo "error: ${CSV} not found (run from repo root)" >&2
  exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
  echo "error: jq is required but not on PATH" >&2
  echo "  brew install jq    # macOS" >&2
  echo "  apt-get install jq # Debian/Ubuntu" >&2
  exit 2
fi

if [[ -z "${GITHUB_TOKEN:-}" ]]; then
  echo "warning: GITHUB_TOKEN is empty; you will hit the 60 req/h unauthenticated rate limit." >&2
  echo "         export GITHUB_TOKEN=ghp_... and re-run for a full sweep." >&2
fi

mkdir -p "${OUT_DIR}"

# ─── Build once up-front so per-repo runs aren't blocked on compile ──────────
echo "==> Building release binary…"
cargo build --release --quiet

# ─── Sweep ───────────────────────────────────────────────────────────────────
declare -a ROWS
FAIL_COUNT=0
TOTAL=0

# Skip the header line; trim CR if the file was edited on Windows.
while IFS=, read -r FULL_NAME EXPECTED NOTES; do
  # Strip a trailing \r if present.
  FULL_NAME="${FULL_NAME%$'\r'}"
  EXPECTED="${EXPECTED%$'\r'}"
  NOTES="${NOTES%$'\r'}"

  # Skip blank lines and comments.
  [[ -z "${FULL_NAME}" ]] && continue
  [[ "${FULL_NAME}" =~ ^# ]] && continue

  TOTAL=$((TOTAL + 1))
  SAFE="${FULL_NAME//\//_}"
  JSON_PATH="${OUT_DIR}/${SAFE}.json"

  echo "==> [${TOTAL}] scanning ${FULL_NAME} (expected: ${EXPECTED})"

  if cargo run --release --quiet -- \
        scan "${FULL_NAME}" \
        --mode standard \
        --format json \
        --output "${OUT_DIR}" \
        --quiet; then

    if [[ ! -f "${JSON_PATH}" ]]; then
      echo "    ! scan succeeded but ${JSON_PATH} not found; skipping row" >&2
      FAIL_COUNT=$((FAIL_COUNT + 1))
      ROWS+=("| ${FULL_NAME} | ${EXPECTED} | ERROR | - | - | - | - | - | - | - | - |")
      continue
    fi

    # Extract fields with jq. Module scores are looked up by module name so
    # ordering in the JSON does not matter. // "n/a" guards missing modules.
    OVERALL=$(jq -r '.overall_score'      "${JSON_PATH}")
    CATEGORY=$(jq -r '.category'          "${JSON_PATH}")
    CONFIDENCE=$(jq -r '.overall_confidence' "${JSON_PATH}")
    RUNTIME=$(jq -r '.runtime_seconds'    "${JSON_PATH}")
    STARS=$(jq -r '.modules[]      | select(.name=="stars")       | .score' "${JSON_PATH}" 2>/dev/null || echo "n/a")
    ACTIVITY=$(jq -r '.modules[]   | select(.name=="activity")    | .score' "${JSON_PATH}" 2>/dev/null || echo "n/a")
    MAINT=$(jq -r '.modules[]      | select(.name=="maintainers") | .score' "${JSON_PATH}" 2>/dev/null || echo "n/a")
    ADOPTION=$(jq -r '.modules[]   | select(.name=="adoption")    | .score' "${JSON_PATH}" 2>/dev/null || echo "n/a")
    SECURITY=$(jq -r '.modules[]   | select(.name=="security")    | .score' "${JSON_PATH}" 2>/dev/null || echo "n/a")

    # Empty fields -> "n/a" so the table never has trailing | | gaps.
    STARS="${STARS:-n/a}"; ACTIVITY="${ACTIVITY:-n/a}"; MAINT="${MAINT:-n/a}"
    ADOPTION="${ADOPTION:-n/a}"; SECURITY="${SECURITY:-n/a}"

    if [[ "${CATEGORY}" == "${EXPECTED}" ]]; then
      MATCH="yes"
    else
      MATCH="NO"
    fi

    ROWS+=("| ${FULL_NAME} | ${EXPECTED} | ${CATEGORY} | ${MATCH} | ${OVERALL} | ${CONFIDENCE} | ${STARS} | ${ACTIVITY} | ${MAINT} | ${ADOPTION} | ${SECURITY} |")
    echo "    -> score=${OVERALL} category=${CATEGORY} confidence=${CONFIDENCE} runtime=${RUNTIME}s"
  else
    echo "    ! scan failed for ${FULL_NAME}" >&2
    FAIL_COUNT=$((FAIL_COUNT + 1))
    ROWS+=("| ${FULL_NAME} | ${EXPECTED} | ERROR | - | - | - | - | - | - | - | - |")
  fi
done < <(tail -n +2 "${CSV}")

# ─── Write summary ───────────────────────────────────────────────────────────
{
  echo "# Repo Trust benchmark sweep — SUMMARY"
  echo
  echo "- Total repos: ${TOTAL}"
  echo "- Failures: ${FAIL_COUNT}"
  echo "- Methodology version: see \`SCORING_VERSION\` in \`src/lib.rs\`"
  echo "- Generated: $(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  echo
  echo "| Repo | Expected | Actual | Match | Overall | Confidence | Stars | Activity | Maintainers | Adoption | Security |"
  echo "|------|----------|--------|-------|---------|------------|-------|----------|-------------|----------|----------|"
  for row in "${ROWS[@]}"; do
    echo "${row}"
  done
} > "${SUMMARY}"

echo
echo "Done — open ${SUMMARY}"

if (( FAIL_COUNT > 0 )); then
  echo "warning: ${FAIL_COUNT} of ${TOTAL} scans failed (see logs above)" >&2
  exit 1
fi
exit 0
