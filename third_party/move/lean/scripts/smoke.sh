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
#   scripts/smoke.sh --no-exe        # Lean fixtures only: no executable, no Move CLI
#   scripts/smoke.sh --jobs N        # Lean processes at once (default below)
#   scripts/smoke.sh --list          # print the fixtures
#
# The Lean fixtures need only the libraries. They run as parallel processes,
# longest first by their times in the previous run, and are reported in the
# order listed here. The `verex-move` executable, which `verex-move verify`
# and the Move Prover tests need, is built beside them; those two run after
# the fixtures, beside each other.
set -u
if [ "${BASH_VERSINFO[0]}" -lt 5 ] || { [ "${BASH_VERSINFO[0]}" = 5 ] && [ "${BASH_VERSINFO[1]}" -lt 1 ]; }; then
  echo "smoke.sh: needs bash 5.1 or later (wait -n -p), not $BASH_VERSION" >&2
  exit 2
fi
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
  Calls/TrustedGeneric
)
# Shared replay of generic family conversion and independent stored invariants.
SHARED_CHECKS=(
  Scalars/BitVectors
  Specifications/AbstractOwners
  Calls/TrustedGeneric
  Closures/Frames
  References/ReturnedMutRefs
  Storage/CrossInv
  Storage/GlobalInv
  Storage/ModuleInvariantChecks
  Storage/TableGlobalWrites
)
# Test modules of verex-ir, elaborated on their own; their output is empty.
IR_TESTS=(
  VerexIR/Tests/SharedContinuations.lean
  VerexIR/Tests/BitLift.lean
  VerexIR/Tests/SharedBudgets.lean
  VerexIR/Tests/PreparationCache.lean
  VerexIR/Tests/FamilyTransport.lean
  VerexSL/Tests/StoredTableInvariantErrors.lean
  VerexSL/Tests/LiteralMaps.lean
  VerexSL/Tests/TypedMaps.lean
  VerexSL/Tests/ArithmeticContext.lean
  VerexSL/Tests/FrameWrites.lean
  VerexSL/Tests/StatedFacts.lean
  VerexSL/Tests/SharedPreparation.lean
  VerexSL/Tests/ContinuationJoins.lean
  VerexSL/Tests/DiagnosticOrder.lean
  VerexSL/Tests/SharedStateLabels.lean
  VerexSL/Tests/SharedReferenceJoins.lean
  VerexSL/Tests/SharedBehaviorJoins.lean
  VerexSL/Tests/SharedNativeJoins.lean
  VerexSL/Tests/SharedCases.lean
  VerexSL/Tests/SharedPruning.lean
  VerexSL/Tests/CallRangeResults.lean
)
# Existing source fixtures replayed with the development join scheduler.
SHARED_IR_TESTS=(
  VerexSL/Tests/StateLabelWitnesses.lean
  VerexSL/Tests/StateLabelCallObservations.lean
  VerexSL/Tests/StateLabelBooleanObservations.lean
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
# The Lean fixtures' times in the previous run, longest scheduled first.
DURATIONS="$LEAN_ROOT/verex-ir/.lake/smoke-durations"
# Memory a Lean fixture process is planned with (the largest peak near 2 GB).
PROCESS_GB=3

# Half the hardware threads, at most one process per PROCESS_GB of available
# memory, at least one.
default_jobs() {
  local jobs available
  jobs=$(( $(nproc 2>/dev/null || echo 2) / 2 ))
  available=$(awk '/^MemAvailable:/ { print int($2 / 1048576) }' /proc/meminfo 2>/dev/null)
  if [ -n "$available" ] && [ $(( available / PROCESS_GB )) -lt "$jobs" ]; then
    jobs=$(( available / PROCESS_GB ))
  fi
  [ "$jobs" -ge 1 ] || jobs=1
  echo "$jobs"
}

run_mvp=1
run_exe=1
jobs=${SMOKE_JOBS:-}
while [ $# -gt 0 ]; do
  case "$1" in
    --no-mvp) run_mvp=0 ;;
    --no-exe) run_exe=0; run_mvp=0 ;;
    --jobs|-j)
      [ $# -ge 2 ] || { echo "smoke.sh: $1 needs a number" >&2; exit 2; }
      jobs=$2; shift ;;
    --list)
      printf 'check %s\n' "${CHECKS[@]}"; printf 'ir %s\n' "${IR_TESTS[@]}"
      printf 'shared ir %s\n' "${SHARED_IR_TESTS[@]}"
      printf 'shared check %s\n' "${SHARED_CHECKS[@]}"
      printf 'verify %s\n' "${SOURCE_VERIFY[@]}"; printf 'mvp %s\n' "${MVP_TESTS[@]}"
      exit 0 ;;
    *) echo "smoke.sh: unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done
[ -n "$jobs" ] || jobs=$(default_jobs)
case "$jobs" in
  ''|*[!0-9]*|0) echo "smoke.sh: --jobs expects a positive number, not '$jobs'" >&2; exit 2 ;;
esac

for name in UB UPBL UPDATE_BASELINE; do
  if [ -n "${!name:-}" ]; then
    echo "smoke.sh: $name is set; the smoke suite never updates baselines" >&2
    exit 2
  fi
done

TMP=$(mktemp -d "${TMPDIR:-/tmp}/verex-smoke.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
# An interrupted run stops everything it started, then cleans up.
trap 'trap "" INT TERM; kill 0 2>/dev/null; exit 130' INT TERM

failures=0
now_ms() { echo $(( ${EPOCHREALTIME/[.,]/} / 1000 )); }
# A result file appears complete or not at all: the pool polls for them.
publish() { # file text
  echo "$2" >"$1.partial" && mv "$1.partial" "$1"
}
started=$(now_ms)
seconds() { # milliseconds
  printf '%d.%d' $(( $1 / 1000 )) $(( $1 % 1000 / 100 ))
}
report() { # name status milliseconds
  printf '%-58s %-4s %6ss\n' "$1" "$2" "$(seconds "$3")"
  [ "$2" = ok ] || failures=$((failures + 1))
}

# Compare an output with a fixture's expectation: its `.exp` when one exists,
# otherwise nothing.
compare() { # name output-file exp-file milliseconds
  local expected=/dev/null
  [ -f "$3" ] && expected="$3"
  if diff -q "$expected" "$2" >/dev/null; then
    report "$1" ok "$4"
  else
    report "$1" FAIL "$4"
    diff "$expected" "$2" | head -40 | sed 's/^/    /'
  fi
}

build() { # package targets...
  local package=$1; shift
  local start; start=$(now_ms)
  if (cd "$LEAN_ROOT/$package" && lake build "$@" >"$TMP/build.log" 2>&1); then
    report "build $package" ok $(( $(now_ms) - start ))
  else
    report "build $package" FAIL $(( $(now_ms) - start ))
    tail -30 "$TMP/build.log" | sed 's/^/    /'
    exit 1
  fi
}

# A background step: its output, status and time in files named by `key`.
step() { # key command...
  local key=$1; shift
  local start; start=$(now_ms)
  "$@" >"$TMP/$key.log" 2>&1
  publish "$TMP/$key.status" $?
  publish "$TMP/$key.ms" $(( $(now_ms) - start ))
}
step_report() { # key name lines-of-log-on-failure
  if [ "$(cat "$TMP/$1.status")" = 0 ]; then
    report "$2" ok "$(cat "$TMP/$1.ms")"
  else
    report "$2" FAIL "$(cat "$TMP/$1.ms")"
    tail -"$3" "$TMP/$1.log" | sed 's/^/    /'
  fi
}

echo "smoke: building"
build verex-ir
build verex-e2e-tests VerexE2ETests

# `verex-move verify` of a single Move file exports it with the standalone
# Move CLI the e2e driver builds (CLAUDE.md, "Toolchains and setup"). The
# Move CLI and the executable build beside the Lean fixtures.
build_move_cli() (
  cd "$REPO_ROOT" && cargo build --locked --profile ci -p aptos-move-cli --features binary \
    --bin move
)
build_executable() (
  cd "$LEAN_ROOT/verex-move" && lake build verex-move
)
cli_built=0
if [ $run_exe = 1 ]; then
  if [ ${#SOURCE_VERIFY[@]} -gt 0 ] && [ -z "${APTOS_MOVE_CLI:-}" ]; then
    cli_built=1
    export APTOS_MOVE_CLI="$REPO_ROOT/target/ci/move"
  fi
  (
    [ $cli_built = 0 ] || step cli build_move_cli
    step exe build_executable
  ) &
  executable_pid=$!
fi

# The Lean fixtures: parallel arrays of the label, how to run, the source, and
# the expected output.
LABELS=() KINDS=() SOURCES=() EXPECTED=()
fixture() { LABELS+=("$1"); KINDS+=("$2"); SOURCES+=("$3"); EXPECTED+=("$4"); }
for test in "${IR_TESTS[@]}"; do fixture "verex-ir $test" ir "$test" /dev/null; done
for test in "${SHARED_IR_TESTS[@]}"; do fixture "shared $test" shared-ir "$test" /dev/null; done
for check in "${CHECKS[@]}"; do
  source="VerexE2ETests/Check/$check.lean"
  fixture "check $check" check "$source" "$LEAN_ROOT/verex-e2e-tests/${source%.lean}.exp"
done
for check in "${SHARED_CHECKS[@]}"; do
  source="VerexE2ETests/Check/$check.lean"
  fixture "shared check $check" shared-check "$source" \
    "$LEAN_ROOT/verex-e2e-tests/${source%.lean}.exp"
done

run_fixture() { # index
  local index=$1 start; start=$(now_ms)
  local caps=("-DmaxHeartbeats=$CHECK_HEARTBEATS" "-Dweak.verex.verifyHeartbeats=$CHECK_HEARTBEATS")
  case ${KINDS[$index]} in
    ir) cd "$LEAN_ROOT/verex-ir" && lake env lean "${SOURCES[$index]}" ;;
    shared-ir) cd "$LEAN_ROOT/verex-ir" &&
      lake env lean -Dverex.sharedContinuations=true "${SOURCES[$index]}" ;;
    check) cd "$LEAN_ROOT/verex-e2e-tests" && lake env lean "${caps[@]}" "${SOURCES[$index]}" ;;
    shared-check) cd "$LEAN_ROOT/verex-e2e-tests" &&
      lake env lean "${caps[@]}" -Dverex.sharedContinuations=true "${SOURCES[$index]}" ;;
  esac >"$TMP/$index.out" 2>&1
  publish "$TMP/$index.ms" $(( $(now_ms) - start ))
}

declare -A previous=()
if [ -f "$DURATIONS" ]; then
  while read -r ms label; do previous[$label]=$ms; done <"$DURATIONS"
fi
# Longest first; a fixture without a previous time counts as the longest.
order=()
while read -r _ index; do order+=("$index"); done < <(
  for index in "${!LABELS[@]}"; do
    echo "${previous[${LABELS[$index]}]:-999999999} $index"
  done | sort -rn)

echo "smoke: ${#LABELS[@]} Lean fixtures, $jobs at a time"
# Report the fixtures in their listed order as soon as each one and all
# before it have finished.
printed=0
report_finished() {
  while [ $printed -lt ${#LABELS[@]} ] && [ -f "$TMP/$printed.ms" ]; do
    compare "${LABELS[$printed]}" "$TMP/$printed.out" "${EXPECTED[$printed]}" \
      "$(cat "$TMP/$printed.ms")"
    printed=$((printed + 1))
  done
}
declare -A running=()
for index in "${order[@]}"; do
  while [ ${#running[@]} -ge "$jobs" ]; do
    wait -n -p finished "${!running[@]}"
    unset "running[$finished]"
    report_finished
  done
  run_fixture "$index" &
  running[$!]=$index
done
while [ ${#running[@]} -gt 0 ]; do
  wait -n -p finished "${!running[@]}"
  unset "running[$finished]"
  report_finished
done
report_finished
for index in "${!LABELS[@]}"; do
  echo "$(cat "$TMP/$index.ms") ${LABELS[$index]}"
done >"$DURATIONS" 2>/dev/null

if [ $run_exe = 0 ]; then
  echo "smoke: --no-exe: skipped verex-move verify and the Move Prover tests"
else
  wait "$executable_pid"
  [ $cli_built = 0 ] || step_report cli "build move CLI (cargo, ci profile)" 20
  step_report exe "build verex-move" 30
  if [ "$(cat "$TMP/exe.status")" = 0 ]; then
    # `verex-move verify` and the Move Prover tests run beside each other.
    verify_sources() {
      local index=0
      for source in "${SOURCE_VERIFY[@]}"; do
        local start; start=$(now_ms)
        (cd "$LEAN_ROOT/verex-e2e-tests" && lake env verex-move verify "$source") \
          >"$TMP/verify-$index.out" 2>/dev/null
        publish "$TMP/verify-$index.ms" $(( $(now_ms) - start ))
        index=$((index + 1))
      done
    }
    verify_sources &
    verify_pid=$!
    if [ $run_mvp = 1 ]; then
      # One process shares its verifier snapshot across all selected fixtures.
      mvp_tests() (
        cd "$MVP_DIR" && MVP_TEST_FEATURE=lean MVP_TEST_FILTERS="$(IFS=,; echo "${MVP_TESTS[*]}")" \
          cargo test -p move-prover --test testsuite
      )
      step mvp mvp_tests
    fi
    wait "$verify_pid"
    index=0
    for source in "${SOURCE_VERIFY[@]}"; do
      compare "verify $source" "$TMP/verify-$index.out" \
        "$LEAN_ROOT/verex-e2e-tests/${source%.move}.exp" "$(cat "$TMP/verify-$index.ms")"
      index=$((index + 1))
    done
    if [ $run_mvp = 1 ]; then
      if [ "$(cat "$TMP/mvp.status")" = 0 ] \
          && grep -Eq "${#MVP_TESTS[@]} passed; 0 failed" "$TMP/mvp.log" \
          && ! grep -q 'will be skipped' "$TMP/mvp.log"; then
        report "mvp (${#MVP_TESTS[@]} fixtures, shared snapshot)" ok "$(cat "$TMP/mvp.ms")"
      else
        report "mvp (${#MVP_TESTS[@]} fixtures, shared snapshot)" FAIL "$(cat "$TMP/mvp.ms")"
        tail -30 "$TMP/mvp.log" | sed 's/^/    /'
      fi
    fi
  fi
fi

echo "smoke: $failures failure(s), $(seconds $(( $(now_ms) - started )))s"
[ $failures = 0 ]
