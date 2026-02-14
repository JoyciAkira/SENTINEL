#!/bin/bash
# Sentinel Benchmark Harness - Wave 3.7
#
# Performance validation suite for Sentinel core components.
# Tests alignment computation, memory compaction, and quality evaluation.

set -euo pipefail

# Colors for output
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly BLUE='\033[0;34m'
readonly NC='\033[0m' # No Color

# Benchmark configuration
readonly SENTINEL_CLI="${SENTINEL_CLI:-./target/release/sentinel}"
readonly SCENARIOS_DIR="$(dirname "$0")/scenarios"
readonly RESULTS_DIR="$(dirname "$0")/results"
readonly ITERATIONS="${ITERATIONS:-5}"

# Ensure results directory exists
mkdir -p "$RESULTS_DIR"

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[PASS]${NC} $1"
}

log_error() {
    echo -e "${RED}[FAIL]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

# Check if sentinel-cli is built
check_cli() {
    if [[ ! -f "$SENTINEL_CLI" ]]; then
        log_error "sentinel-cli not found at $SENTINEL_CLI"
        log_info "Build with: cargo build --release"
        return 1
    fi
    log_success "Found sentinel-cli at $SENTINEL_CLI"
}

# Run a single benchmark scenario
run_scenario() {
    local scenario="$1"
    local scenario_file="$SCENARIOS_DIR/$scenario.json"

    if [[ ! -f "$scenario_file" ]]; then
        log_error "Scenario file not found: $scenario_file"
        return 1
    fi

    log_info "Running scenario: $scenario"

    # Initialize sentinel with the scenario
    local intent
    intent=$(jq -r '.intent' "$scenario_file")

    # Clean up any existing sentinel.json
    rm -f /tmp/benchmark-sentinel.json

    # Initialize manifold
    "$SENTINEL_CLI" init "$intent" --manifold /tmp/benchmark-sentinel.json > /dev/null 2>&1

    # Benchmark iterations
    local total_time=0
    for ((i = 1; i <= ITERATIONS; i++)); do
        local start end duration
        start=$(date +%s%N)

        # Run status command to test alignment computation
        "$SENTINEL_CLI" status --json --manifold /tmp/benchmark-sentinel.json > /dev/null 2>&1

        end=$(date +%s%N)
        duration=$(( (end - start) / 1000000 )) # Convert to milliseconds
        total_time=$((total_time + duration))
        echo "  Iteration $i: ${duration}ms"
    done

    local avg_time=$((total_time / ITERATIONS))
    log_success "Average time: ${avg_time}ms"

    # Write result to JSON
    local result_file="$RESULTS_DIR/${scenario}.json"
    cat > "$result_file" <<EOF
{
  "scenario": "$scenario",
  "iterations": $ITERATIONS,
  "total_time_ms": $total_time,
  "average_time_ms": $avg_time,
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
}
EOF

    # Check against performance threshold
    local threshold
    threshold=$(jq -r '.threshold_ms // 1000' "$scenario_file")

    if (( avg_time <= threshold )); then
        log_success "Within threshold (${threshold}ms)"
        return 0
    else
        log_error "Exceeds threshold (${threshold}ms)"
        return 1
    fi
}

# Run all benchmarks
run_all() {
    log_info "Starting Sentinel Benchmark Suite"
    log_info "CLI: $SENTINEL_CLI"
    log_info "Iterations: $ITERATIONS"
    echo ""

    local passed=0
    local failed=0
    local scenarios=()

    # Find all scenario files
    for scenario_file in "$SCENARIOS_DIR"/*.json; do
        if [[ -f "$scenario_file" ]]; then
            local scenario
            scenario=$(basename "$scenario_file" .json)
            scenarios+=("$scenario")
        fi
    done

    if [[ ${#scenarios[@]} -eq 0 ]]; then
        log_warning "No benchmark scenarios found"
        return 1
    fi

    log_info "Found ${#scenarios[@]} scenario(s)"
    echo ""

    for scenario in "${scenarios[@]}"; do
        if run_scenario "$scenario"; then
            ((passed++))
        else
            ((failed++))
        fi
        echo ""
    done

    # Summary
    log_info "Benchmark Summary:"
    log_info "  Passed: $passed"
    log_info "  Failed: $failed"

    # Generate overall report
    cat > "$RESULTS_DIR/summary.json" <<EOF
{
  "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "iterations": $ITERATIONS,
  "passed": $passed,
  "failed": $failed,
  "total_scenarios": $((passed + failed))
}
EOF

    return $failed
}

# Main entry point
main() {
    check_cli || exit 1
    run_all
}

# Run main if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi
