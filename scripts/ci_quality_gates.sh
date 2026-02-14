#!/usr/bin/env bash
set -euo pipefail

echo "[quality] running sentinel quality gates"
echo "[quality] rust version: $(rustc --version)"

SLO_MIN_TOTAL_TESTS="${SLO_MIN_TOTAL_TESTS:-200}"
SLO_MIN_PASS_RATE="${SLO_MIN_PASS_RATE:-1.0}"
SLO_MAX_ROLLBACK_RATE="${SLO_MAX_ROLLBACK_RATE:-0.05}"

TOTAL_TESTS=0
TOTAL_PASSED=0
TOTAL_FAILED=0

run_tests_and_collect() {
  local package="$1"
  local output

  if ! output="$(cargo test -q -p "$package" 2>&1)"; then
    echo "$output"
    echo "[quality] FAIL: tests failed for package '$package'"
    exit 1
  fi

  echo "$output"

  local pkg_passed pkg_failed
  pkg_passed="$(echo "$output" | awk '/test result: ok\./{for (i=1; i<=NF; i++) if ($i=="passed;") sum += $(i-1)} END{print sum+0}')"
  pkg_failed="$(echo "$output" | awk '/test result: ok\./{for (i=1; i<=NF; i++) if ($i=="failed;") sum += $(i-1)} END{print sum+0}')"

  TOTAL_PASSED=$((TOTAL_PASSED + pkg_passed))
  TOTAL_FAILED=$((TOTAL_FAILED + pkg_failed))
  TOTAL_TESTS=$((TOTAL_TESTS + pkg_passed + pkg_failed))
}

# Keep gates deterministic and aligned with the current repository baseline.
run_tests_and_collect sentinel-core
run_tests_and_collect sentinel-agent-native
run_tests_and_collect sentinel-cli
run_tests_and_collect sentinel-sandbox

echo "[quality] running strict clippy warning gates"
cargo clippy -q -p sentinel-core -- -D warnings
cargo clippy -q -p sentinel-agent-native -- -D warnings
cargo clippy -q -p sentinel-cli -- -D warnings

PASS_RATE="$(awk -v p="$TOTAL_PASSED" -v t="$TOTAL_TESTS" 'BEGIN { if (t == 0) print 0; else printf "%.6f", p / t }')"
ROLLBACK_PROXY="$(awk -v pr="$PASS_RATE" 'BEGIN { printf "%.6f", 1.0 - pr }')"

echo "[quality] KPI summary: total_tests=$TOTAL_TESTS passed=$TOTAL_PASSED failed=$TOTAL_FAILED pass_rate=$PASS_RATE rollback_proxy=$ROLLBACK_PROXY"

if [ "$TOTAL_TESTS" -lt "$SLO_MIN_TOTAL_TESTS" ]; then
  echo "[quality] FAIL: SLO_MIN_TOTAL_TESTS violated ($TOTAL_TESTS < $SLO_MIN_TOTAL_TESTS)"
  exit 1
fi

if awk -v pr="$PASS_RATE" -v min="$SLO_MIN_PASS_RATE" 'BEGIN { exit !(pr < min) }'; then
  echo "[quality] FAIL: SLO_MIN_PASS_RATE violated ($PASS_RATE < $SLO_MIN_PASS_RATE)"
  exit 1
fi

if awk -v rb="$ROLLBACK_PROXY" -v max="$SLO_MAX_ROLLBACK_RATE" 'BEGIN { exit !(rb > max) }'; then
  echo "[quality] FAIL: SLO_MAX_ROLLBACK_RATE violated ($ROLLBACK_PROXY > $SLO_MAX_ROLLBACK_RATE)"
  exit 1
fi

echo "[quality] all gates passed"
