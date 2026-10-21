#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# The core smoke suite of the Verex Lean stack: the few fixtures that catch
# the regressions this stack produces, in minutes, between the checkpoint
# gates (the four package suites and the Move Prover's lean gate). It never
# updates a baseline: a diff is a failure to report.
#
#   scripts/smoke.sh                 # build in place, then run the fixtures
#   scripts/smoke.sh --no-mvp        # skip the Move Prover tests (no cargo)
#   scripts/smoke.sh --list          # print the fixtures
set -u
LEAN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
REPO_ROOT="$(cd "$LEAN_ROOT/../../.." && pwd)"
MVP_DIR="$REPO_ROOT/third_party/move/move-prover"

# Check fixtures of verex-e2e-tests (VerexE2ETests/Check/<name>.lean), each
# elaborated as the Check driver elaborates it.
CHECKS=(
  Storage/TableReads
  Storage/TableAllocation
  Storage/TableGlobalWrites
  Storage/TableStoredInvariants
  Storage/WrappedTables
  Specifications/TableSnapshots
  Specifications/InsertionMap
  Specifications/OptionalMapResults
  Specifications/AbstractOwners
  Closures/InvariantBehavior
)
# Test modules of verex-ir, elaborated on their own; their output is empty.
IR_TESTS=(
  VerexSL/Tests/StoredTableInvariantErrors.lean
  VerexSL/Tests/LiteralMaps.lean
  VerexSL/Tests/TypedMaps.lean
  VerexSL/Tests/SharedPreparation.lean
  VerexSL/Tests/CallRangeResults.lean
)
# Move sources of verex-e2e-tests verified with `verex-move verify`, whose
# reports are their `.exp` (the hypothesis list of a typing theorem shows
# in behavior_manual_facts).
SOURCE_VERIFY=(
  VerexE2ETests/SourceVerify/behavior_manual_facts.move
)
# Move Prover unit tests run together with the lean feature and one snapshot.
MVP_TESTS=(
  closures/inline/folds_of_map_intrinsic.move
  regression/fun_value_equality.move
  intrinsic_map_insertion_order.move
  simple_vector_client.move
  verify_table.move
  data_invariant_in_map.move
)
# The caps the Check driver elaborates under (VerexE2ETests/Check.lean).
CHECK_HEARTBEATS=180000

run_mvp=1
for argument in "$@"; do
  case "$argument" in
    --no-mvp) run_mvp=0 ;;
    --list)
      printf 'check %s\n' "${CHECKS[@]}"; printf 'ir %s\n' "${IR_TESTS[@]}"
      printf 'verify %s\n' "${SOURCE_VERIFY[@]}"; printf 'mvp %s\n' "${MVP_TESTS[@]}"
      exit 0 ;;
    *) echo "smoke.sh: unknown argument $argument" >&2; exit 2 ;;
  esac
done

for name in UB UPBL UPDATE_BASELINE; do
  if [ -n "${!name:-}" ]; then
    echo "smoke.sh: $name is set; the smoke suite never updates baselines" >&2
    exit 2
  fi
done

failures=0
started=$(date +%s)
report() { # name status seconds
  printf '%-58s %-4s %6ss\n' "$1" "$2" "$3"
  [ "$2" = ok ] || failures=$((failures + 1))
}
elapsed() { echo $(( $(date +%s) - $1 )); }

# Compare an output with a fixture's expectation: its `.exp` when one exists,
# otherwise nothing.
compare() { # name output-file exp-file start
  local expected=/dev/null
  [ -f "$3" ] && expected="$3"
  if diff -q "$expected" "$2" >/dev/null; then
    report "$1" ok "$(elapsed "$4")"
  else
    report "$1" FAIL "$(elapsed "$4")"
    diff "$expected" "$2" | head -40 | sed 's/^/    /'
  fi
}

build() { # package targets...
  local package=$1; shift
  local start; start=$(date +%s)
  if (cd "$LEAN_ROOT/$package" && lake build "$@" >"$LEAN_ROOT/.smoke-build.log" 2>&1); then
    report "build $package" ok "$(elapsed "$start")"
  else
    report "build $package" FAIL "$(elapsed "$start")"
    tail -30 "$LEAN_ROOT/.smoke-build.log" | sed 's/^/    /'
    rm -f "$LEAN_ROOT/.smoke-build.log"
    exit 1
  fi
  rm -f "$LEAN_ROOT/.smoke-build.log"
}

echo "smoke: building"
build verex-ir
build verex-e2e-tests VerexE2ETests

cd "$LEAN_ROOT/verex-ir"
for test in "${IR_TESTS[@]}"; do
  start=$(date +%s)
  lake env lean "$test" >"$LEAN_ROOT/.smoke-out" 2>&1
  compare "verex-ir $test" "$LEAN_ROOT/.smoke-out" /dev/null "$start"
done

cd "$LEAN_ROOT/verex-e2e-tests"
for check in "${CHECKS[@]}"; do
  start=$(date +%s)
  source="VerexE2ETests/Check/$check.lean"
  lake env lean "-DmaxHeartbeats=$CHECK_HEARTBEATS" \
    "-Dweak.verex.verifyHeartbeats=$CHECK_HEARTBEATS" "$source" >"$LEAN_ROOT/.smoke-out" 2>&1
  compare "check $check" "$LEAN_ROOT/.smoke-out" "${source%.lean}.exp" "$start"
done

# `verex-move verify` of a single Move file exports it with the standalone
# Move CLI the e2e driver builds (CLAUDE.md, "Toolchains and setup").
if [ ${#SOURCE_VERIFY[@]} -gt 0 ]; then
  if [ -z "${APTOS_MOVE_CLI:-}" ]; then
    start=$(date +%s)
    if (cd "$REPO_ROOT" && cargo build --locked --profile ci -p aptos-move-cli --features binary \
        --bin move >"$LEAN_ROOT/.smoke-build.log" 2>&1); then
      report "build move CLI (cargo, ci profile)" ok "$(elapsed "$start")"
    else
      report "build move CLI (cargo, ci profile)" FAIL "$(elapsed "$start")"
      tail -20 "$LEAN_ROOT/.smoke-build.log" | sed 's/^/    /'
    fi
    rm -f "$LEAN_ROOT/.smoke-build.log"
    export APTOS_MOVE_CLI="$REPO_ROOT/target/ci/move"
  fi
  build verex-move verex-move
  for source in "${SOURCE_VERIFY[@]}"; do
    start=$(date +%s)
    lake env verex-move verify "$source" >"$LEAN_ROOT/.smoke-out" 2>/dev/null
    compare "verify $source" "$LEAN_ROOT/.smoke-out" "${source%.move}.exp" "$start"
  done
fi
rm -f "$LEAN_ROOT/.smoke-out"

if [ $run_mvp = 1 ]; then
  start=$(date +%s)
  mvp_filters=$(IFS=,; echo "${MVP_TESTS[*]}")
  # One process shares its verifier snapshot across all selected fixtures.
  if (cd "$MVP_DIR" && MVP_TEST_FEATURE=lean MVP_TEST_FILTERS="$mvp_filters" \
      cargo test -p move-prover --test testsuite >"$LEAN_ROOT/.smoke-mvp.log" 2>&1) \
      && grep -Eq "${#MVP_TESTS[@]} passed; 0 failed" "$LEAN_ROOT/.smoke-mvp.log" \
      && ! grep -q 'will be skipped' "$LEAN_ROOT/.smoke-mvp.log"; then
    report "mvp (${#MVP_TESTS[@]} fixtures, shared snapshot)" ok "$(elapsed "$start")"
  else
    report "mvp (${#MVP_TESTS[@]} fixtures, shared snapshot)" FAIL "$(elapsed "$start")"
    tail -30 "$LEAN_ROOT/.smoke-mvp.log" | sed 's/^/    /'
  fi
  rm -f "$LEAN_ROOT/.smoke-mvp.log"
fi

echo "smoke: $failures failure(s), $(elapsed "$started")s"
[ $failures = 0 ]
