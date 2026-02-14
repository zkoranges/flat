#!/usr/bin/env bash
# bench.sh — Quick benchmark for flat on a real codebase (bat)
#
# Usage:
#   ./scripts/bench.sh            # clone bat if needed, then benchmark
#   ./scripts/bench.sh --clean    # remove cloned repo
#
# Requires: cargo build --release has been run

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_DIR="$SCRIPT_DIR/../.bench-repos"
BAT_DIR="$REPO_DIR/bat"
FLAT="$SCRIPT_DIR/../target/release/flat"

if [[ "${1:-}" == "--clean" ]]; then
    echo "Removing $REPO_DIR"
    rm -rf "$REPO_DIR"
    exit 0
fi

# Ensure release binary exists
if [[ ! -x "$FLAT" ]]; then
    echo "Error: Release binary not found. Run: cargo build --release"
    exit 1
fi

# Clone bat if not present
if [[ ! -d "$BAT_DIR" ]]; then
    echo "Cloning bat repository..."
    mkdir -p "$REPO_DIR"
    git clone --depth 1 https://github.com/sharkdp/bat.git "$BAT_DIR"
fi

echo "=== Benchmark: flat on bat ==="
echo ""

run_bench() {
    local label="$1"
    shift
    echo "--- $label ---"
    time "$FLAT" "$BAT_DIR" "$@" --stats 2>&1 | grep -E '^(Total|Included|Compressed|Token|Excluded|Output|Skipped)'
    echo ""
}

run_bench "Baseline (no flags)"
run_bench "--compress"                  --compress
run_bench "--compress --stats"          --compress --stats
run_bench "--tokens 8000"               --tokens 8000
run_bench "--tokens 8000 --compress"    --tokens 8000 --compress
run_bench "--compress --include rs"     --compress --include rs

echo "=== Done ==="
