#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# prover-smoke: the inner loop for changes to the prover of the Verex Lean
# stack (kernel laws, solver, specification layer). In minutes it elaborates
# small VerexSL fixtures with the built Lean libraries alone, each as its own
# Lean process: no `verex-move` or `verex-rust` executable, no Move CLI, no
# cargo, no e2e driver. It never updates a baseline or a cost pin: a diff is
# a failure to report.
#
# It is not evidence for a frontend change. A rendered fixture below covers
# what the prover does with VerexSL a frontend once wrote, not the frontend
# that writes it; source verification through the executables, the Move
# Prover's lean tests, the Rust renderings and the `verex-rust` suite are
# checked by the gate (the four package suites, the e2e sweeps and the Move
# Prover's lean gate), which every commit still needs. The package over
# compiled namespaces (verex-e2e-tests/VerexE2ETests/CompiledNamespaces) is in
# no suite either: the gate runs it as `scripts/verify-compiled-namespaces.sh`.
#
#   scripts/prover-smoke.sh            # build the libraries in place, run the fixtures
#   scripts/prover-smoke.sh --jobs N   # Lean processes at once (or PROVER_SMOKE_JOBS; default below)
#   scripts/prover-smoke.sh --costs    # also print every pinned cost as measured
#   scripts/prover-smoke.sh --list     # print the fixtures
#   scripts/prover-smoke.sh --self-test # check the script's reading of costs, of process failures, and its cancellation
#
# The fixtures run several at a time, longest first by their times in the
# previous run, and are reported in the order listed here, one line each,
# with wall and CPU seconds. A fixture fails when its process does not return
# what its expectation gives (a signal, a silent failure), when its output is
# not its expectation or holds a PANIC, or when a pinned cost is missed or its
# target was not verified. `scripts/prover-smoke-coverage.md` says
# which fixture stands for which mechanism, regression class and cost, and
# what is not covered.
set -u
if [ "${BASH_VERSINFO[0]}" -lt 5 ] || { [ "${BASH_VERSINFO[0]}" = 5 ] && [ "${BASH_VERSINFO[1]}" -lt 1 ]; }; then
  echo "prover-smoke: needs bash 5.1 or later (wait -n -p), not $BASH_VERSION" >&2
  exit 2
fi
LEAN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

# The shape matrix (verex-ir/VerexSL/Tests/Shapes/<name>.lean): one module per
# mechanism, one small function per type shape, negatives pinned; a cell that
# does not verify stays under `expectOpen` (Shapes/README.md lists them).
SHAPES=(
  SupportControls
  Construct
  Mutate
  CallArguments
  CallResults
  Generics
  OpaqueGenerics
  Storage
  StorageBorrows
  Vectors
  Tables
  TableRows
  TableTakes
  Enums
  Closures
  KnownClosures
  MethodRejections
  Control
)
# Further test modules of verex-ir, elaborated on their own; their output is
# empty. The controls of the join scheduler, of preparation and its caches,
# of maps, frames and stated facts, of diagnostics, and of state labels.
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
  VerexSL/Tests/SharedCallPremises.lean
  VerexSL/Tests/CallRangeResults.lean
  VerexSL/Tests/StateLabelWitnesses.lean
  VerexSL/Tests/StateLabelCallObservations.lean
  VerexSL/Tests/StateLabelBooleanObservations.lean
)
# Controls of boundaries that are no type shape: evidence that arrives before
# and after the read it belongs to, facts of the same carrier at another type,
# frame or owner (and a raw projection beside its application), an invalid
# value inside a chain of writes, a declaration that is imported beside one
# that is local, the lifetime of the denotation cache, a `verify` command
# beside its module's own verification, a residual and the callee's row it
# belongs to, a dictionary's contract that is missing or another's, the
# generated bridges of own invariants, the domain a call binds its result
# at, guarded woven behaviors, a nested read under a pinned budget, the
# element a successful read of a typed literal lookup yields, a range goal
# stated on a named result, a read of a written vector at a position the
# write leaves alone, and the conditions of two kinds one site owes.
BOUNDARY_TESTS=(
  VerexIR/Tests/VectorElementReads.lean
  VerexSL/Tests/ExecutableVectorReads.lean
  VerexSL/Tests/LeafOpenedFacts.lean
  VerexSL/Tests/NominalFrameObservations.lean
  VerexIR/Tests/StoredBehaviorFrames.lean
  VerexSL/Tests/WrittenContainerHolds.lean
  VerexIR/Tests/OwnBridgeDeclared.lean
  VerexSL/Tests/DenoteCacheImport.lean
  VerexSL/Tests/DenoteCache.lean
  VerexSL/Tests/GivenOwnSelection.lean
  VerexIR/Tests/CallArgumentResidual.lean
  VerexIR/Tests/BehaviorDispatchResidual.lean
  VerexSL/Tests/ForwardedMethodEvidence.lean
  VerexSL/Tests/OwnBridgeGenerated.lean
  VerexSL/Tests/OwnBridgeLeaves.lean
  VerexSL/Tests/CallResultDomains.lean
  VerexSL/Tests/CallResultHolds.lean
  VerexSL/Tests/GuardedWovenReadings.lean
  VerexSL/Tests/CertifiedFieldProjections.lean
  VerexSL/Tests/MiniAmmPositives.lean
  VerexSL/Tests/MapDefaultPositives.lean
  VerexSL/Tests/MapSizeUpdatePositives.lean
  VerexSL/Tests/ConditionKindPositives.lean
  VerexSL/Tests/ClauseSingletonConcat.lean
  VerexSL/Tests/LiteralVectorPush.lean
  VerexSL/Tests/CanonicalMapValidity.lean
  VerexSL/Tests/WrittenBackRoundTrip.lean
  VerexSL/Tests/LiteralVectorReads.lean
  VerexSL/Tests/RangeNamedResults.lean
  VerexIR/Tests/VectorWriteSizes.lean
)
# Renderings of source fixtures as a frontend writes them
# (verex-ir/VerexSL/Tests/Rendered/<name>.lean), checked in with the fixture's
# diagnostics; `scripts/prover-smoke-renderings.sh`, a tool of the gate,
# writes them again and tells a stale one. Each stands for a shape only a
# frontend produces.
RENDERED=(
  static_step
  static_fnonce
  fnonce_forward
  library_slice_get_mut_errors
  library_option_map
  table_stored_resource
  generic_function_literals_model
  mini_amm
  map_key_default
  map_value_default
  map_size_update
  condition_kinds
)
# Check fixtures of verex-e2e-tests (VerexE2ETests/Check/<name>.lean), each
# elaborated as the Check driver elaborates it. These print nothing.
CHECKS=(
  Storage/TableReads
  Storage/TableAllocation
  Storage/TableGlobalWrites
  Storage/WrappedTables
  Storage/TableStoredInvariants
  Specifications/TableSnapshots
  Specifications/InsertionMap
  Specifications/OptionalMapResults
  Specifications/AbstractOwners
  Closures/InvariantBehavior
  Calls/TrustedGeneric
  References/ReturnedMutRefs
  Storage/CrossInv
  Storage/GlobalInv
  Generics/Generics
)
# Check fixtures whose expectation is their `.exp`: the diagnostics of their
# intended negatives.
CHECKS_EXPECTING=(
  Specifications/ImpureCalls
  Closures/TupleTargets
  Closures/Frames
  Scalars/BitVectors
  Storage/ModuleInvariantChecks
)
# Check fixtures that belong here and are not run, each with the reason and
# what stands for it meanwhile. Every run and `--list` print them. The list
# is empty.
DEFERRED_CHECKS=()
# The performance tier: larger targets whose cost grows faster than their
# size, each with pinned heartbeat costs in `scripts/prover-smoke-cost.pins`
# (which may also pin targets of the fixtures above). Test modules of
# verex-ir, check fixtures, and benchmarks of `bench/lean`, each elaborated
# as it is.
COST_IR_TESTS=(
  VerexSL/Tests/Cost/MapScaling.lean
)
COST_CHECKS=(
  Closures/KnownSiteCost
  Examples/Quicksort
)
COST_BENCH=(
  OrderedMap
)
# The pinned costs, and the percentage a measured cost or ratio may differ
# from its pin in either direction.
PINS="$LEAN_ROOT/scripts/prover-smoke-cost.pins"
COST_MARGIN=5
# The caps the Check driver elaborates under (VerexE2ETests/Check.lean).
CHECK_HEARTBEATS=180000
# The fixtures' times in the previous run, longest scheduled first.
DURATIONS="$LEAN_ROOT/verex-ir/.lake/prover-smoke-durations"
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

show_costs=0
list_only=0
check_reading=0
jobs=${PROVER_SMOKE_JOBS:-}
while [ $# -gt 0 ]; do
  case "$1" in
    --costs) show_costs=1 ;;
    --jobs|-j)
      [ $# -ge 2 ] || { echo "prover-smoke: $1 needs a number" >&2; exit 2; }
      jobs=$2; shift ;;
    --list) list_only=1 ;;
    --self-test) check_reading=1 ;;
    *) echo "prover-smoke: unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done

# The fixtures: parallel arrays of the label, how to run, the source, the
# expected output, and whether the fixture's costs are pinned.
LABELS=() KINDS=() SOURCES=() EXPECTED=() PINNED=()
fixture() { # label kind source expected [pinned]
  local pinned=${5:-0}
  # Any fixture with a line in the pins file has its costs compared.
  if [ "$pinned" = 0 ] && grep -Eq "^(pin|ratio) +$3 " "$PINS" 2>/dev/null; then pinned=1; fi
  LABELS+=("$1"); KINDS+=("$2"); SOURCES+=("$3"); EXPECTED+=("$4"); PINNED+=("$pinned")
}
# A check fixture is listed with its expectation (`<name>.exp` beside it,
# which must be there) or as printing nothing (and then has no `.exp`).
check_fixture() { # name pinned expecting
  local source="VerexE2ETests/Check/$1.lean" expected=/dev/null
  [ "$3" = 0 ] || expected="$LEAN_ROOT/verex-e2e-tests/${source%.lean}.exp"
  fixture "check $1" check "$source" "$expected" "$2"
}
for shape in "${SHAPES[@]}"; do
  fixture "shapes $shape" ir "VerexSL/Tests/Shapes/$shape.lean" /dev/null
done
for test in "${IR_TESTS[@]}"; do fixture "verex-ir $test" ir "$test" /dev/null; done
for test in "${BOUNDARY_TESTS[@]}"; do fixture "boundary $test" ir "$test" /dev/null; done
for name in "${RENDERED[@]}"; do
  fixture "rendered $name" ir "VerexSL/Tests/Rendered/$name.lean" /dev/null
done
for check in "${CHECKS[@]}"; do check_fixture "$check" 0 0; done
for check in "${CHECKS_EXPECTING[@]}"; do check_fixture "$check" 0 1; done
for test in "${COST_IR_TESTS[@]}"; do fixture "cost verex-ir $test" ir "$test" /dev/null 1; done
for check in "${COST_CHECKS[@]}"; do check_fixture "$check" 1 0; done
for bench in "${COST_BENCH[@]}"; do
  fixture "cost bench $bench" bench "../bench/lean/$bench.lean" /dev/null 1
done

if [ $list_only = 1 ]; then
  printf '%s\n' "${LABELS[@]}"
  [ ${#DEFERRED_CHECKS[@]} = 0 ] || printf 'deferred check %s\n' "${DEFERRED_CHECKS[@]}"
  exit 0
fi
[ -n "$jobs" ] || jobs=$(default_jobs)
case "$jobs" in
  ''|*[!0-9]*|0) echo "prover-smoke: --jobs expects a positive number, not '$jobs'" >&2; exit 2 ;;
esac

for name in UB UPBL UPDATE_BASELINE; do
  if [ -n "${!name:-}" ]; then
    echo "prover-smoke: $name is set; the suite never updates baselines" >&2
    exit 2
  fi
done

TMP=$(mktemp -d "${TMPDIR:-/tmp}/verex-prover-smoke.XXXXXX")
trap 'rm -rf "$TMP"' EXIT
# Each job this run starts in the background (a build, a fixture's worker) is
# a process group of its own, recorded here, so that an interrupted run can
# stop exactly what it started: the worker, the `lake` below it and the Lean
# process below that, whichever of them is still alive. The script's own
# process group is its caller's where no job control gave it one, and is
# never signalled. A job turns job control off for itself, so that what it
# starts stays in its group.
set -m
declare -A running=()
stop_jobs() {
  local job
  [ ${#running[@]} -gt 0 ] || return 0
  for job in "${!running[@]}"; do kill -TERM -- "-$job" 2>/dev/null; done
  # Reaped before anything of theirs is removed.
  wait "${!running[@]}" 2>/dev/null
  running=()
}
trap 'trap "" INT TERM; stop_jobs; exit 130' INT TERM

failures=0
failed=()
cpu_total=0
now_ms() { echo $(( ${EPOCHREALTIME/[.,]/} / 1000 )); }
# A result file appears complete or not at all: the pool polls for them.
publish() { # file text
  echo "$2" >"$1.partial" && mv "$1.partial" "$1"
}
started=$(now_ms)
seconds() { # milliseconds
  printf '%d.%d' $(( $1 / 1000 )) $(( $1 % 1000 / 100 ))
}
report() { # name status milliseconds [cpu-milliseconds]
  if [ -n "${4:-}" ]; then
    printf '%-58s %-4s %6ss %7ss cpu\n' "$1" "$2" "$(seconds "$3")" "$(seconds "$4")"
    cpu_total=$(( cpu_total + $4 ))
  else
    printf '%-58s %-4s %6ss\n' "$1" "$2" "$(seconds "$3")"
  fi
  [ "$2" = ok ] || failures=$((failures + 1))
}

# Run a command as a fixture's process: its output, its exit status and its
# user and system seconds, in files named by `key`. What the shell says of a
# process a signal ended goes with the seconds, not with the output.
in_directory() { # directory command...
  cd "$1" && shift && "$@"
}
capture() { # key command...
  local key=$1; shift
  local TIMEFORMAT='%U %S'
  { time {
    "$@" >"$TMP/$key.out" 2>&1
    echo $? >"$TMP/$key.status"
  } ; } 2>"$TMP/$key.cpu"
}
# What is wrong with a fixture's process, a line each. Lean returns 1 where
# it reported an error and 0 otherwise, so a fixture whose expectation holds
# error diagnostics returns 1 and every other returns 0: a test module of
# verex-ir, a benchmark and a check fixture without an `.exp` print nothing
# and return 0. Any other status fails, whatever the output: a process a
# signal ended or one that failed without a word has an empty output, which
# is what such a fixture expects. A PANIC fails whatever the status is.
process_lines() { # status-file output expected
  local status wanted=0
  status=$(cat "$1" 2>/dev/null)
  ! grep -q ': error' "$3" 2>/dev/null || wanted=1
  case "$status" in
    '') echo "process: no exit status was recorded" ;;
    *[!0-9]*) echo "process: the recorded exit status is not a number: $status" ;;
    *)
      if [ "$status" -gt 128 ]; then
        echo "process: ended by signal $(( status - 128 )), where its fixture returns $wanted"
      elif [ "$status" != "$wanted" ]; then
        echo "process: returned $status, where its fixture returns $wanted"
      fi ;;
  esac
  if grep -aq 'PANIC' "$2"; then
    echo "panic: the fixture's output holds $(grep -ac 'PANIC' "$2") PANIC line(s):"
    grep -a 'PANIC' "$2" | sort | uniq -c | head -3
  fi
}

# The pinned costs of a fixture against its stage log, which records the
# heartbeats of each verified target under its module. A line per pin that
# the measured cost misses by more than the margin, in either direction, and
# per pinned target that was not verified; with mode `all`, a line per pin.
# A ratio pin compares the quotient of two targets' costs. A cost under its
# pin fails too, so that a pin stays where the cost is; it says nothing of
# whether the target still proves what it proved.
cost_lines() { # source stage-log mode [pins-file]
  awk -v key="$1" -v margin="$COST_MARGIN" -v mode="$3" '
    function percent(measured, pinned) { return (measured - pinned) * 100.0 / pinned }
    function verdict(what, measured, pinned, format, below,    moved) {
      moved = percent(measured, pinned)
      if (moved > margin)
        printf "cost: %s measured " format " against the pin " format ": %+.1f%%, over the margin of %d%%\n", what, measured, pinned, moved, margin
      else if (moved < -margin)
        printf "cost: %s measured " format " against the pin " format ": %+.1f%%, under the margin of %d%%: %s\n", what, measured, pinned, moved, margin, below
      else if (mode == "all")
        printf "cost: %s measured " format " against the pin " format ": %+.1f%%\n", what, measured, pinned, moved
    }
    FNR == NR {
      # A target may hold spaces; the pinned figure is the last field.
      if ($1 == "pin" && $2 == key) {
        target = $0; sub(/^pin +[^ ]+ +/, "", target); sub(/ +[^ ]+$/, "", target)
        targets[++pins] = target; pin[target] = $NF
      }
      else if ($1 == "ratio" && $2 == key) { ratios++; over[ratios] = $3; under[ratios] = $4; ratio[ratios] = $5 }
      next
    }
    # A line of the log is its clock, then the event. A module that is
    # verified where it is elaborated brackets its targets; a `verify`
    # command logs its target alone, under no module.
    { event = $0; sub(/^[0-9]+ +/, "", event) }
    event ~ /^module .*: verification start$/ {
      module = event; sub(/^module /, "", module); sub(/: verification start$/, "", module); next
    }
    event ~ /^module .*: verification done$/ { module = ""; next }
    (at = index(event, ": theorem done, ")) && event ~ /, verified, / {
      target = module "::" substr(event, 1, at - 1)
      split(substr(event, at + length(": theorem done, ")), figures, " ")
      if (target in cost) twice[target] = 1
      if (figures[1] !~ /^[0-9]+$/) malformed[target] = 1
      cost[target] = figures[1]
    }
    END {
      if (pins + ratios == 0) printf "cost: no pin for %s in the pins file\n", key
      for (index_ = 1; index_ <= pins; index_++) {
        target = targets[index_]
        if (target in malformed) printf "cost: the logged figure of the pinned target %s is not a number\n", target
        else if (target in twice) printf "cost: two verified targets are logged as %s: a pin needs a name of its own\n", target
        else if (!(target in cost)) printf "cost: the pinned target %s was not verified; pin %.0f\n", target, pin[target]
        else verdict(target " heartbeats", cost[target], pin[target], "%.0f", "neither a defect nor a reason to re-pin by itself: check that the target proves what it proved, then lower the pin")
      }
      for (index_ = 1; index_ <= ratios; index_++) {
        if ((over[index_] in malformed) || (under[index_] in malformed))
          printf "cost: the logged figure of a target of the pinned ratio %s / %s is not a number\n", over[index_], under[index_]
        else if ((over[index_] in twice) || (under[index_] in twice))
          printf "cost: two verified targets are logged under a name of the pinned ratio %s / %s\n", over[index_], under[index_]
        else if (!(over[index_] in cost) || !(under[index_] in cost))
          printf "cost: a target of the pinned ratio %s / %s was not verified\n", over[index_], under[index_]
        else if (cost[under[index_]] + 0 == 0)
          printf "cost: the pinned ratio %s / %s has a denominator of no heartbeats\n", over[index_], under[index_]
        else
          verdict("the ratio " over[index_] " / " under[index_], cost[over[index_]] / cost[under[index_]], ratio[index_], "%.3f", "the larger target got cheaper against the smaller: check both, then update the ratio pin")
      }
    }' "${4:-$PINS}" "$2"
}

# The script's own checks, run before the fixtures. First the reading of a
# stage log against canned pins: a module verified where it is elaborated, a `verify` command after it, a name with a space, a target
# that failed, one that is absent, a name logged twice, and a figure that is
# not a number.
self_test() {
  local directory; directory=$(mktemp -d "${TMPDIR:-/tmp}/verex-prover-smoke-test.XXXXXX")
  cat >"$directory/stage" <<'LOG'
100  0x1 ::m : unit
101 module «0x1».m: verification start
102 f: theorem start
103 f: theorem done, 1000 heartbeats, verified, automatic
104 impl Step for Counter: theorem done, 2000 heartbeats, verified, authored
105 g: theorem done, 500 heartbeats, failed, automatic
106 module «0x1».m: verification done
107 f: theorem done, 4000 heartbeats, verified, automatic
108 twin: theorem done, 10 heartbeats, verified, automatic
109 twin: theorem done, 10 heartbeats, verified, automatic
110 odd: theorem done, many heartbeats, verified, automatic
LOG
  cat >"$directory/pins" <<'PINS'
pin   source «0x1».m::f 1000
pin   source ::f 3000
pin   source «0x1».m::impl Step for Counter 2000
pin   source «0x1».m::g 500
pin   source ::twin 10
pin   source ::odd 10
pin   source ::absent 10
ratio source ::f «0x1».m::f 4.000
ratio source ::f «0x1».m::f 5.000
PINS
  cat >"$directory/expected" <<'EXPECTED'
cost: «0x1».m::f heartbeats measured 1000 against the pin 1000: +0.0%
cost: ::f heartbeats measured 4000 against the pin 3000: +33.3%, over the margin of 5%
cost: «0x1».m::impl Step for Counter heartbeats measured 2000 against the pin 2000: +0.0%
cost: the pinned target «0x1».m::g was not verified; pin 500
cost: two verified targets are logged as ::twin: a pin needs a name of its own
cost: the logged figure of the pinned target ::odd is not a number
cost: the pinned target ::absent was not verified; pin 10
cost: the ratio ::f / «0x1».m::f measured 4.000 against the pin 4.000: +0.0%
cost: the ratio ::f / «0x1».m::f measured 4.000 against the pin 5.000: -20.0%, under the margin of 5%: the larger target got cheaper against the smaller: check both, then update the ratio pin
EXPECTED
  cost_lines source "$directory/stage" all "$directory/pins" >"$directory/actual"
  local status=0
  diff "$directory/expected" "$directory/actual" || status=1
  # The verdict on a fixture's process, of processes run as a fixture's is: one
  # that prints nothing and returns 0, one that fails without a word, one a
  # signal ends, one that returns 0 over a PANIC, one that reports the error
  # its expectation holds, and that expectation with a process a signal ended.
  local TMP=$directory
  : >"$directory/nothing"
  echo "a.lean:1:1: error: expected" >"$directory/errors"
  process_case() { # name expectation expected-lines command...
    local name=$1 expectation=$2 wanted=$3; shift 3
    capture "$name" "$@" 2>/dev/null
    process_lines "$directory/$name.status" "$directory/$name.out" "$expectation" |
      head -1 >"$directory/$name.lines"
    if [ "$(cat "$directory/$name.lines")" != "$wanted" ]; then
      echo "process case $name: expected '$wanted', found '$(cat "$directory/$name.lines")'"
      status=1
    fi
  }
  process_case clean "$directory/nothing" "" true
  process_case silent "$directory/nothing" "process: returned 3, where its fixture returns 0" \
    sh -c 'exit 3'
  process_case signalled "$directory/nothing" \
    "process: ended by signal 9, where its fixture returns 0" sh -c 'kill -KILL $$'
  process_case panicking "$directory/nothing" \
    "panic: the fixture's output holds 1 PANIC line(s):" sh -c 'echo "PANIC at a b:1:2: c"'
  process_case negative "$directory/errors" "" \
    sh -c 'echo "a.lean:1:1: error: expected"; exit 1'
  process_case negative-signalled "$directory/errors" \
    "process: ended by signal 15, where its fixture returns 1" sh -c 'kill -TERM $$'
  process_case negative-clean "$directory/errors" \
    "process: returned 0, where its fixture returns 1" true
  # A cancelled run, with stubs: two jobs, each with a child and a grandchild,
  # are stopped with everything below them, and a process of the script's own
  # process group that is no job of the run is left alone.
  set +m; sleep 60 & local bystander=$!; set -m
  local job tries
  for job in 1 2; do
    (set +m; sh -c 'sleep 60 & echo $! >"$1"; wait' stub "$directory/below-$job") &
    running[$!]=stub
  done
  for tries in 1 2 3 4 5 6 7 8 9 10; do
    [ -s "$directory/below-1" ] && [ -s "$directory/below-2" ] && break
    sleep 0.2
  done
  local below; below=$(cat "$directory"/below-* 2>/dev/null)
  stop_jobs
  for tries in 1 2 3 4 5 6 7 8 9 10; do
    kill -0 $below 2>/dev/null || break
    sleep 0.2
  done
  if [ -z "$below" ] || kill -0 $below 2>/dev/null; then
    echo "cancellation: what a stopped job started is still alive, or was never started"
    status=1
  fi
  if ! kill -0 "$bystander" 2>/dev/null; then
    echo "cancellation: a process that is no job of the run was stopped"
    status=1
  fi
  kill "$bystander" 2>/dev/null; wait "$bystander" 2>/dev/null
  rm -rf "$directory"
  return $status
}
# Every run checks itself first: a verdict is only as good as its reading.
if ! self_test; then
  echo "prover-smoke: the self-test of the cost reading, the process verdicts and the cancellation fails" >&2
  exit 2
fi
if [ $check_reading = 1 ]; then
  echo "prover-smoke: the cost reading, the process verdicts and the cancellation pass their self-test"
  exit 0
fi

# The raw output of the builds and of every failed fixture stays here after a
# run with a failure.
KEPT="$LEAN_ROOT/verex-ir/.lake/prover-smoke-failed"
build() { # package targets...
  local package=$1; shift
  local start; start=$(now_ms)
  local status=0
  (set +m; cd "$LEAN_ROOT/$package" && lake build "$@") >"$TMP/build-$package.log" 2>&1 &
  local job=$!
  running[$job]=build
  wait "$job" || status=1
  unset "running[$job]"
  # A panic while a library builds is logged and the build goes on.
  if grep -aq 'PANIC' "$TMP/build-$package.log"; then status=2; fi
  if [ $status = 0 ]; then
    report "build $package" ok $(( $(now_ms) - start ))
  else
    report "build $package" FAIL $(( $(now_ms) - start ))
    if [ $status = 2 ]; then
      echo "    panic: the build's output holds a PANIC:"
      grep -a 'PANIC' "$TMP/build-$package.log" | head -5 | sed 's/^/    /'
    else
      tail -30 "$TMP/build-$package.log" | sed 's/^/    /'
    fi
    rm -rf "$KEPT"; mkdir -p "$KEPT" && cp "$TMP"/build-*.log "$KEPT"/
    echo "prover-smoke: the build failed; its output is in $KEPT"
    exit 1
  fi
}

# Every fixture elaborates against libraries built from the sources as they
# are: the libraries, the test modules the listed verex-ir fixtures import,
# and the one module of interpreter assertions a check fixture imports.
echo "prover-smoke: building"
supports=()
for index in "${!LABELS[@]}"; do
  [ "${KINDS[$index]}" = ir ] || continue
  while read -r module; do
    case " ${supports[*]:-} " in *" +$module "*) ;; *) supports+=("+$module") ;; esac
  done < <(sed -n 's/^import \(Verex[A-Z]*\.Tests\.[A-Za-z0-9_.]*\).*/\1/p' \
    "$LEAN_ROOT/verex-ir/${SOURCES[$index]}")
done
build verex-ir VerexIR VerexSL "${supports[@]}"
build verex-e2e-tests +VerexE2ETests.CheckSupport

run_fixture() { # index
  set +m
  local index=$1 start; start=$(now_ms)
  local caps=("-DmaxHeartbeats=$CHECK_HEARTBEATS" "-Dweak.verex.verifyHeartbeats=$CHECK_HEARTBEATS")
  local stage=()
  [ "${PINNED[$index]}" = 0 ] || stage=("-Dweak.verex.stageLog=$TMP/$index.stage")
  case ${KINDS[$index]} in
    ir) capture "$index" in_directory "$LEAN_ROOT/verex-ir" \
      lake env lean "${stage[@]}" "${SOURCES[$index]}" ;;
    check) capture "$index" in_directory "$LEAN_ROOT/verex-e2e-tests" \
      lake env lean "${caps[@]}" "${stage[@]}" "${SOURCES[$index]}" ;;
    bench) capture "$index" in_directory "$LEAN_ROOT/verex-e2e-tests" \
      lake env lean "${stage[@]}" "${SOURCES[$index]}" ;;
  esac
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

echo "prover-smoke: ${#LABELS[@]} fixtures, $jobs at a time"
# The user and system seconds of a fixture's process, in milliseconds.
cpu_ms() { # index
  tail -1 "$TMP/$1.cpu" 2>/dev/null | awk '{ printf "%d", ($1 + $2) * 1000 }'
}
# A fixture passes when its process returned what its expectation gives, its
# output is its expectation (its `.exp` when one exists, otherwise nothing)
# and holds no PANIC, and, where its costs are pinned, every pinned target
# was verified within its pin.
judge() { # index
  local expected=${EXPECTED[$1]} beside
  : >"$TMP/$1.verdict"
  # An expectation that is listed is there to be read; a fixture listed as
  # printing nothing has none beside it.
  if [ "$expected" != /dev/null ] && [ ! -r "$expected" ]; then
    echo "expectation: $expected is listed and cannot be read" >>"$TMP/$1.verdict"
    expected=/dev/null
  fi
  beside="$LEAN_ROOT/verex-e2e-tests/${SOURCES[$1]%.lean}.exp"
  if [ "${KINDS[$1]}" = check ] && [ "$expected" = /dev/null ] && [ -e "$beside" ]; then
    echo "expectation: the fixture has an .exp and is listed as printing nothing" >>"$TMP/$1.verdict"
  fi
  # The process next: its status and a panic are named before the
  # difference they make to the output.
  process_lines "$TMP/$1.status" "$TMP/$1.out" "$expected" >>"$TMP/$1.verdict"
  diff "$expected" "$TMP/$1.out" >"$TMP/$1.diff"
  [ $? -le 1 ] || echo "expectation: the comparison with $expected failed" >>"$TMP/$1.verdict"
  head -40 "$TMP/$1.diff" >>"$TMP/$1.verdict"
  if [ "${PINNED[$1]}" != 0 ]; then
    if [ -f "$TMP/$1.stage" ]; then
      cost_lines "${SOURCES[$1]}" "$TMP/$1.stage" fail >>"$TMP/$1.verdict" ||
        echo "cost: the reading of the stage log failed" >>"$TMP/$1.verdict"
    else
      echo "cost: no stage log was written for ${SOURCES[$1]}" >>"$TMP/$1.verdict"
    fi
  fi
  local status=ok
  [ ! -s "$TMP/$1.verdict" ] || status=FAIL
  report "${LABELS[$1]}" "$status" "$(cat "$TMP/$1.ms")" "$(cpu_ms "$1")"
  [ $status = ok ] || failed+=("$1")
  sed 's/^/    /' "$TMP/$1.verdict"
  if [ $show_costs = 1 ] && [ "${PINNED[$1]}" != 0 ] && [ -f "$TMP/$1.stage" ]; then
    cost_lines "${SOURCES[$1]}" "$TMP/$1.stage" all | sed 's/^/    /'
  fi
}
# Report the fixtures in their listed order as soon as each one and all
# before it have finished.
printed=0
report_finished() {
  while [ $printed -lt ${#LABELS[@]} ] && [ -f "$TMP/$printed.ms" ]; do
    judge "$printed"
    printed=$((printed + 1))
  done
}
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
# A worker that left no record of completion has no status either, and its
# fixture fails for that.
for index in "${!LABELS[@]}"; do
  [ -f "$TMP/$index.ms" ] || publish "$TMP/$index.ms" 0
  [ -f "$TMP/$index.out" ] || : >"$TMP/$index.out"
done
report_finished
for index in "${!LABELS[@]}"; do
  echo "$(cat "$TMP/$index.ms") ${LABELS[$index]}"
done >"$DURATIONS" 2>/dev/null

rm -rf "$KEPT"
if [ ${#failed[@]} -gt 0 ]; then
  mkdir -p "$KEPT" && cp "$TMP"/build-*.log "$KEPT"/
  for index in "${failed[@]}"; do
    name=$(echo "${LABELS[$index]}" | tr ' /' '__')
    cp "$TMP/$index.out" "$KEPT/$name.out"
    [ ! -f "$TMP/$index.stage" ] || cp "$TMP/$index.stage" "$KEPT/$name.stage"
  done
  echo "prover-smoke: the output of the failed fixtures is in $KEPT"
fi
[ ${#DEFERRED_CHECKS[@]} = 0 ] ||
  echo "prover-smoke: not run, deferred: ${DEFERRED_CHECKS[*]}"
echo "prover-smoke: $failures failure(s), $(seconds $(( $(now_ms) - started )))s, $(seconds $cpu_total)s cpu in the fixtures, $jobs at a time"
[ $failures = 0 ]
