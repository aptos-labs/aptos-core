#!/usr/bin/env bash
# Benchmark source-verification proof cost across the whole test suite.
#
# Every `verify` proof is wrapped in `move_bench`, which is inert unless the
# environment variable MOVE_PROOF_BENCH is set.  With it set, each proof logs
#   ‖MOVE_BENCH‖  <name>  <heartbeats>  <elapsed-ms>
# Heartbeats are deterministic (independent of machine, load, and the aptos
# CLI the suite otherwise spends its wall time in), so their sum is a stable
# benchmark of proof work.  Wall-ms is reported too, but only as a hint.
#
# Usage:  scripts/bench-proofs.sh [N]      # N = top slowest to list (default 15)
set -euo pipefail
cd "$(dirname "$0")/../move"
TOP="${1:-15}"
RAW="$(mktemp)"
trap 'rm -f "$RAW"' EXIT

# Force re-elaboration of every test file (oleans are cached otherwise).
rm -rf .lake/build/lib/lean/Move/Tests
echo "building suite with proof benchmarking on ..." >&2
MOVE_PROOF_BENCH=1 lake test > "$RAW" 2>&1 || true

grep -aF '‖MOVE_BENCH‖' "$RAW" \
  | sed 's/.*‖MOVE_BENCH‖//' \
  | awk -F'\t' 'NF>=4 && $2!="" {print $2"\t"$3"\t"$4}' \
  | sort -u > "$RAW.parsed"

awk -F'\t' '
  { hb[$1]+=$2; ms[$1]+=$3; n++ }
  END {
    th=0; tm=0
    for (k in hb) { th+=hb[k]; tm+=ms[k] }
    printf "\n=== proof benchmark: %d verified functions ===\n", n
    printf "total heartbeats: %d\n", th
    printf "total wall (ms):  %d   (noisy; heartbeats are the stable metric)\n", tm
  }
' "$RAW.parsed"

echo ""
echo "top ${TOP} by heartbeats:"
sort -t$'\t' -k2 -rn "$RAW.parsed" | head -n "$TOP" \
  | awk -F'\t' '{printf "  %10d hb  %6d ms  %s\n", $2, $3, $1}'

if ! grep -aqF '‖MOVE_BENCH‖' "$RAW"; then
  echo "no benchmark lines found — did the build fail?" >&2
  tail -5 "$RAW" >&2
fi
rm -f "$RAW.parsed"
