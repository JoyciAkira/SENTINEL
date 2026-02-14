#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

OUT_DIR="$ROOT_DIR/.sentinel/quality"
mkdir -p "$OUT_DIR"

TS="$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
RUN_ID="$(date -u +"%Y%m%dT%H%M%SZ")"
REPORT_JSON="$OUT_DIR/harness-${RUN_ID}.json"
REPORT_LOG="$OUT_DIR/harness-${RUN_ID}.log"

PACKAGES=("sentinel-core" "sentinel-agent-native" "sentinel-cli")
TOTAL_PASSED=0
TOTAL_FAILED=0
TOTAL_IGNORED=0
TEST_COMMANDS=0
FAILED_PACKAGES=()
START_EPOCH="$(date +%s)"

echo "[harness] run_id=$RUN_ID started_at=$TS" | tee "$REPORT_LOG"

run_package_tests() {
  local pkg="$1"
  local tmp_log
  tmp_log="$(mktemp)"
  echo "[harness] testing package '$pkg'" | tee -a "$REPORT_LOG"
  if cargo test -p "$pkg" >"$tmp_log" 2>&1; then
    :
  else
    FAILED_PACKAGES+=("$pkg")
  fi
  cat "$tmp_log" >>"$REPORT_LOG"

  local parsed
  parsed="$(awk '
    /test result:/ {
      passed += $4;
      failed += $6;
      ignored += $8;
      commands += 1;
    }
    END {
      if (commands == 0) {
        print "0 0 0 0";
      } else {
        print passed " " failed " " ignored " " commands;
      }
    }' "$tmp_log")"
  rm -f "$tmp_log"

  local passed failed ignored commands
  read -r passed failed ignored commands <<<"$parsed"
  TOTAL_PASSED=$((TOTAL_PASSED + passed))
  TOTAL_FAILED=$((TOTAL_FAILED + failed))
  TOTAL_IGNORED=$((TOTAL_IGNORED + ignored))
  TEST_COMMANDS=$((TEST_COMMANDS + commands))
}

for pkg in "${PACKAGES[@]}"; do
  run_package_tests "$pkg"
done

echo "[harness] running cargo check for MCP runtime stack" | tee -a "$REPORT_LOG"
CARGO_CHECK_OK=true
if ! cargo check -p sentinel-agent-native -p sentinel-cli >>"$REPORT_LOG" 2>&1; then
  CARGO_CHECK_OK=false
fi

END_EPOCH="$(date +%s)"
DURATION_SEC=$((END_EPOCH - START_EPOCH))
TOTAL_TESTS=$((TOTAL_PASSED + TOTAL_FAILED))
PASS_RATE="0.00"
if [[ "$TOTAL_TESTS" -gt 0 ]]; then
  PASS_RATE="$(awk -v p="$TOTAL_PASSED" -v t="$TOTAL_TESTS" 'BEGIN { printf "%.4f", p / t }')"
fi

OVERALL_OK=true
if [[ "$TOTAL_FAILED" -gt 0 || "$CARGO_CHECK_OK" != "true" ]]; then
  OVERALL_OK=false
fi

{
  echo "{"
  echo "  \"run_id\": \"${RUN_ID}\","
  echo "  \"started_at_utc\": \"${TS}\","
  echo "  \"duration_sec\": ${DURATION_SEC},"
  echo "  \"overall_ok\": ${OVERALL_OK},"
  echo "  \"cargo_check_ok\": ${CARGO_CHECK_OK},"
  echo "  \"kpi\": {"
  echo "    \"total_tests\": ${TOTAL_TESTS},"
  echo "    \"passed\": ${TOTAL_PASSED},"
  echo "    \"failed\": ${TOTAL_FAILED},"
  echo "    \"ignored\": ${TOTAL_IGNORED},"
  echo "    \"pass_rate\": ${PASS_RATE},"
  echo "    \"test_result_sections\": ${TEST_COMMANDS}"
  echo "  },"
  echo "  \"packages\": ["
  for i in "${!PACKAGES[@]}"; do
    comma=","
    if [[ "$i" -eq $((${#PACKAGES[@]} - 1)) ]]; then
      comma=""
    fi
    echo "    \"${PACKAGES[$i]}\"${comma}"
  done
  echo "  ],"
  echo "  \"failed_packages\": ["
  for i in "${!FAILED_PACKAGES[@]}"; do
    comma=","
    if [[ "$i" -eq $((${#FAILED_PACKAGES[@]} - 1)) ]]; then
      comma=""
    fi
    echo "    \"${FAILED_PACKAGES[$i]}\"${comma}"
  done
  echo "  ],"
  echo "  \"log_path\": \"${REPORT_LOG}\""
  echo "}"
} >"$REPORT_JSON"

echo "[harness] report_json=$REPORT_JSON" | tee -a "$REPORT_LOG"
echo "[harness] report_log=$REPORT_LOG" | tee -a "$REPORT_LOG"
echo "[harness] overall_ok=$OVERALL_OK pass_rate=$PASS_RATE total_tests=$TOTAL_TESTS" | tee -a "$REPORT_LOG"

if [[ "$OVERALL_OK" != "true" ]]; then
  exit 1
fi
