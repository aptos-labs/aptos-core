#!/usr/bin/env bash
# Benchmark the Rust source round-trip or Rust-to-VerexSL e2e pipeline.
#
# Usage:
#   scripts/bench-rust-pipeline.sh source   # default: exporter/source/VIR phases
#   scripts/bench-rust-pipeline.sh e2e      # exporter vs VerexSL print/re-import
set -euo pipefail

LEAN_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
MODE="${1:-source}"

quiet_build() {
  if ! lake build "$@" >/dev/null 2>&1; then
    lake build "$@"
  fi
}

case "$MODE" in
  source)
    cd "$LEAN_ROOT/verex-rust"
    quiet_build VerexRust.Driver
    RAW="$(mktemp "$LEAN_ROOT/verex-rust/.lake/build/rust-pipeline-bench.XXXXXX")"
    trap 'rm -f "$RAW"' EXIT
    VEREX_RUST_BENCHMARK=1 VEREX_RUST_BENCHMARK_OUTPUT="$RAW" \
      lake env lean VerexRust/Tests/Source.lean
    ;;
  e2e)
    cd "$LEAN_ROOT/verex-e2e-tests"
    quiet_build VerexE2ETestDriver
    RAW="$(mktemp "$LEAN_ROOT/verex-e2e-tests/.lake/build/rust-pipeline-bench.XXXXXX")"
    trap 'rm -f "$RAW"' EXIT
    VEREX_E2E_SUITE=rust VEREX_RUST_BENCHMARK=1 \
      VEREX_RUST_BENCHMARK_OUTPUT="$RAW" lake test
    ;;
  *)
    echo "usage: $0 [source|e2e]" >&2
    exit 2
    ;;
esac

if ! grep -q '^VEREX_RUST_BENCH|' "$RAW"; then
  echo "benchmark produced no phase timings" >&2
  exit 1
fi

echo ""
echo "=== Rust pipeline benchmark: $MODE ==="
awk -F'|' '
  /^VEREX_RUST_BENCH\|/ {
    total[$2] += $3
    count[$2] += 1
    if ($3 > maximum[$2]) maximum[$2] = $3
  }
  END {
    for (phase in total) {
      printf "%-30s %9.3f s  %4d calls  %8.3f ms/call  %8.3f ms max\n",
        phase, total[phase] / 1000000000, count[phase],
        total[phase] / count[phase] / 1000000, maximum[phase] / 1000000
    }
  }
' "$RAW" | sort -k2,2nr
