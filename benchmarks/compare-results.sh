#!/bin/bash
# Sentinel Benchmark Comparison Tool
#
# Compares benchmark results between two runs to detect regressions.

set -euo pipefail

readonly RESULTS_DIR="$(dirname "$0")/results"
readonly REGRESSION_THRESHOLD="${REGRESSION_THRESHOLD:-10}" # percent

# Colors
readonly RED='\033[0;31m'
readonly GREEN='\033[0;32m'
readonly YELLOW='\033[1;33m'
readonly NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[REGRESSION]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

# Find the two most recent benchmark runs
find_recent_runs() {
    local runs
    runs=$(ls -t "$RESULTS_DIR"/*.json 2>/dev/null | grep -v summary.json || true)

    if [[ -z "$runs" ]]; then
        echo "Error: No benchmark results found"
        return 1
    fi

    local count
    count=$(echo "$runs" | wc -l | xargs)

    if [[ $count -lt 2 ]]; then
        echo "Error: Need at least 2 benchmark runs to compare"
        return 1
    fi

    # Get two most recent summary files
    local baseline current
    baseline=$(ls -t "$RESULTS_DIR"/summary*.json 2>/dev/null | head -2 | tail -1)
    current=$(ls -t "$RESULTS_DIR"/summary*.json 2>/dev/null | head -1)

    if [[ -z "$baseline" || -z "$current" ]]; then
        # Fallback: use scenario files
        local scenarios
        scenarios=$(ls "$RESULTS_DIR"/*.json 2>/dev/null | grep -v summary | sort -r | head -1 | xargs -I{} dirname {})
        baseline=$(ls "$RESULTS_DIR"/*.json 2>/dev/null | grep -v summary | sort | tail -1)
        current=$(ls "$RESULTS_DIR"/*.json 2>/dev/null | grep -v summary | sort -r | head -1)
    fi

    echo "$baseline|$current"
}

# Compare two benchmark results
compare_results() {
    local baseline_file="$1"
    local current_file="$2"
    local scenario_name
    scenario_name=$(basename "$current_file" .json)

    local baseline_time current_time percent_diff
    baseline_time=$(jq -r '.average_time_ms // .time_ms // 0' "$baseline_file")
    current_time=$(jq -r '.average_time_ms // .time_ms // 0' "$current_file")

    if [[ $baseline_time -eq 0 ]]; then
        log_warning "Skipping $scenario_name (invalid baseline: $baseline_time)"
        return 0
    fi

    # Calculate percentage difference
    local diff
    diff=$((current_time - baseline_time))
    percent_diff=$((diff * 100 / baseline_time))

    printf "%-30s %10s %10s %8s\n" \
        "$scenario_name" \
        "${baseline_time}ms" \
        "${current_time}ms" \
        "${percent_diff}%"

    # Check for regression
    if [[ $percent_diff -gt $REGRESSION_THRESHOLD ]]; then
        log_error "$scenario_name: +${percent_diff}% (exceeds ${REGRESSION_THRESHOLD}% threshold)"
        return 1
    elif [[ $percent_diff -lt -$REGRESSION_THRESHOLD ]]; then
        log_info "$scenario_name: ${percent_diff}% (improvement!)"
    fi

    return 0
}

# Main comparison function
main() {
    log_info "Sentinel Benchmark Comparison"
    log_info "Regression threshold: ${REGRESSION_THRESHOLD}%"
    echo ""

    # Get all unique scenario names from results
    local scenarios
    scenarios=$(ls "$RESULTS_DIR"/*.json 2>/dev/null | grep -v summary | xargs -I{} basename {} .json | sort -u || true)

    if [[ -z "$scenarios" ]]; then
        echo "Error: No benchmark results found in $RESULTS_DIR"
        return 1
    fi

    # For each scenario, compare runs over time
    printf "\n%-30s %10s %10s %8s\n" "Scenario" "Baseline" "Latest" "Change"
    printf "%-30s %10s %10s %8s\n" "--------" "--------" "------" "-----"

    local regressions=0

    while IFS= read -r scenario; do
        local files
        files=$(ls -t "$RESULTS_DIR"/${scenario}.json 2>/dev/null || true)

        if [[ -z "$files" ]]; then
            continue
        fi

        local file_count
        file_count=$(echo "$files" | wc -l | xargs)

        if [[ $file_count -lt 2 ]]; then
            continue
        fi

        # Compare oldest vs latest
        local baseline current
        baseline=$(echo "$files" | tail -1)
        current=$(echo "$files" | head -1)

        if ! compare_results "$baseline" "$current"; then
            ((regressions++))
        fi
    done <<< "$scenarios"

    echo ""

    if [[ $regressions -gt 0 ]]; then
        log_error "Found $regressions regression(s)"
        return 1
    else
        log_info "No regressions detected"
        return 0
    fi
}

main "$@"
