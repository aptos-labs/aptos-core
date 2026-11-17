#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# The rendered snapshots of prover-smoke
# (verex-ir/VerexSL/Tests/Rendered/<name>.lean) against what the frontends
# render now. prover-smoke elaborates the checked-in snapshots with the
# libraries alone and never runs this script; this is the one place that runs
# a frontend for them, and it is a check of the gate: a frontend or printer
# change that alters a rendering makes it fail until the snapshot is written
# again. It replaces no other check of the gate.
#
#   scripts/prover-smoke-renderings.sh                 # every snapshot: is it what the frontend renders?
#   scripts/prover-smoke-renderings.sh --write         # write every snapshot again
#   scripts/prover-smoke-renderings.sh --write a b     # write the snapshots of the fixtures a and b
#
# It needs the e2e driver (`lake build VerexE2ETestDriver` in verex-e2e-tests),
# the Move CLI (APTOS_MOVE_CLI, default target/ci/move) for Move fixtures, and
# the Rust exporter (VEREX_RUST_EXPORTER, default the release build under
# verex-rust/rust-exporter) for Rust fixtures. It never touches a `.exp`.
set -u
LEAN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "$LEAN_ROOT/../../.." && pwd)"
E2E="$LEAN_ROOT/verex-e2e-tests"
FIXTURES="$E2E/VerexE2ETests/SourceVerify"
SNAPSHOTS="$LEAN_ROOT/verex-ir/VerexSL/Tests/Rendered"
export APTOS_MOVE_CLI="${APTOS_MOVE_CLI:-$REPO_ROOT/target/ci/move}"
export VEREX_RUST_EXPORTER="${VEREX_RUST_EXPORTER:-$LEAN_ROOT/verex-rust/rust-exporter/target/release/verex-rust-export}"

write=0
names=()
while [ $# -gt 0 ]; do
  case "$1" in
    --write) write=1 ;;
    -*) echo "prover-smoke-renderings: unknown argument $1" >&2; exit 2 ;;
    *) names+=("$1") ;;
  esac
  shift
done
if [ ${#names[@]} = 0 ]; then
  for snapshot in "$SNAPSHOTS"/*.lean; do
    name=$(basename "$snapshot" .lean)
    [ "$name" = Support ] || names+=("$name")
  done
fi
TMP=$(mktemp -d "${TMPDIR:-/tmp}/verex-prover-smoke-renderings.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
: >"$TMP/started"

move=() rust=()
for name in "${names[@]}"; do
  if [ -f "$FIXTURES/$name.move" ]; then move+=("$name")
  elif [ -f "$FIXTURES/$name.rs" ]; then rust+=("$name")
  else echo "prover-smoke-renderings: no source fixture $name" >&2; exit 2
  fi
done
# The driver verifies a Move fixture and leaves its rendering under `.verex`;
# its verdict on the baseline is not this script's subject.
if [ ${#move[@]} -gt 0 ]; then
  (cd "$E2E" && VEREX_E2E_MANAGED=1 VEREX_E2E_SUITE=verify \
    VEREX_E2E_FILTER="$(IFS=,; echo "${move[*]}")" \
    lake env .lake/build/bin/VerexE2ETestDriver) >"$TMP/driver.log" 2>&1
fi
if [ ${#rust[@]} -gt 0 ]; then
  (cd "$E2E" && PROVER_SMOKE_RENDER_NAMES="$(IFS=,; echo "${rust[*]}")" PROVER_SMOKE_RENDER_OUT="$TMP/rust" \
    lake env lean "$LEAN_ROOT/scripts/prover-smoke-render-rust.lean") >"$TMP/rust.log" 2>&1 ||
    { cat "$TMP/rust.log" >&2; exit 1; }
fi

stale=0
for name in "${names[@]}"; do
  if [ -f "$FIXTURES/$name.move" ]; then rendering="$FIXTURES/.verex/$name.move.lean"
  else rendering="$TMP/rust/$name.rs.lean"
  fi
  # A rendering left by an earlier run is not what the frontend renders now.
  if [ ! -f "$rendering" ] || [ ! "$rendering" -nt "$TMP/started" ]; then
    echo "$name: no rendering was written"
    tail -5 "$TMP/driver.log" "$TMP/rust.log" 2>/dev/null | sed 's/^/    /'
    stale=$((stale + 1)); continue
  fi
  python3 -I "$LEAN_ROOT/scripts/prover-smoke-rendering.py" "$name" "$rendering" "$FIXTURES" \
    "$TMP/$name.lean" || { stale=$((stale + 1)); continue; }
  if [ $write = 1 ]; then
    cp "$TMP/$name.lean" "$SNAPSHOTS/$name.lean"
    echo "$name: written"
  elif diff -q "$TMP/$name.lean" "$SNAPSHOTS/$name.lean" >/dev/null 2>&1; then
    echo "$name: current"
  else
    echo "$name: STALE"
    diff "$SNAPSHOTS/$name.lean" "$TMP/$name.lean" 2>&1 | head -20 | sed 's/^/    /'
    stale=$((stale + 1))
  fi
done
echo "prover-smoke-renderings: $stale stale of ${#names[@]}"
[ $stale = 0 ]
