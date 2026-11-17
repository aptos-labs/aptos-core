#!/usr/bin/env bash
# Copyright © Aptos Foundation
# SPDX-License-Identifier: Apache-2.0
#
# The Move package `verex-e2e-tests/VerexE2ETests/CompiledNamespaces` verified
# over compiled namespaces, the route `scripts/verify-modules.py` drives:
# the package's namespaces are rendered and each compiled to an `.olean`, and
# `fixture::client` is verified over them, importing `base` rather than
# elaborating it. The report is compared with the package's `client.exp`, which
# this check never writes: a diff is a failure to report.
#
#   scripts/verify-compiled-namespaces.sh             # build verex-move, then verify
#   scripts/verify-compiled-namespaces.sh --no-build  # use the built executable as it is
#   scripts/verify-compiled-namespaces.sh --self-test # the script's own check of its verdict
#
# The fixture is a negative: its clause fails where `client` inlines it, so the
# run is only what the baseline records when all of this holds — a matching
# report alone is not a pass:
#
#   * the driver ends with status 1, the status its reported errors give;
#   * its result (`--json`) arrives, is judged, and names exactly
#     `fixture::client`, ended with the errors it reports, no timeout, no
#     crash, and no namespace that did not compile;
#   * no `PANIC` in anything the run wrote, before the report is reduced;
#   * the reduced report is `client.exp`.
#
# It needs the `verex-move` executable, built here unless `--no-build`, and a
# Move CLI under the `APTOS_MOVE_CLI` contract (`<workspace>/target/ci/move` by
# default; `cargo build --locked --profile ci -p aptos-move-cli --features
# binary --bin move`). The namespaces are compiled in a directory of their own,
# removed with the run, so each run compiles them from the sources as they are.
set -u
LEAN_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PACKAGE="$LEAN_ROOT/verex-e2e-tests/VerexE2ETests/CompiledNamespaces"
EXPECTED="$PACKAGE/client.exp"
MODULE=fixture::client

build=1
self_test_only=0
while [ $# -gt 0 ]; do
  case "$1" in
    --no-build) build=0 ;;
    --self-test) self_test_only=1 ;;
    *) echo "verify-compiled-namespaces: unknown argument $1" >&2; exit 2 ;;
  esac
  shift
done

for name in UB UPBL UPDATE_BASELINE; do
  if [ -n "${!name:-}" ]; then
    echo "verify-compiled-namespaces: $name is set; this check never updates a baseline" >&2
    exit 2
  fi
done

# The run's own account of what it did, which the reduced report does not
# carry: the result the driver writes with `--json`. It prints one verdict,
# `ok` or the reason it is not the expected run, and ends successfully; a
# verdict is the reader's only word, so its caller takes anything else — a
# status of its own, no python, no line — as a run it cannot judge.
result_verdict() { # result-file
  python3 - "$1" "$MODULE" <<'PY'
import json, sys


def verdict(path, module):
    try:
        with open(path) as stream:
            result = json.load(stream)
    except OSError:
        return "the driver wrote no result of its run"
    except ValueError as error:
        return f"the driver's result does not parse: {error}"
    if not isinstance(result, dict):
        return f"the driver's result is a {type(result).__name__}, not an object"
    if result.get("schema") != "verex-module-sweep":
        return f"the driver's result is not a module sweep: {result.get('schema')!r}"
    if "namespace_failures" not in result:
        return "the driver's result does not say whether the namespaces compiled"
    failures = result["namespace_failures"]
    if failures != []:
        return f"namespaces that did not compile: {failures!r}"
    modules = result.get("modules")
    if not isinstance(modules, list) or len(modules) != 1:
        count = len(modules) if isinstance(modules, list) else "no"
        return f"the result holds {count} modules, where the run verifies one"
    entry = modules[0]
    if not isinstance(entry, dict):
        return f"the result's module is a {type(entry).__name__}, not an object"
    if entry.get("module") != module:
        return f"the result is of `{entry.get('module')}`, not `{module}`"
    outcome, errors, exit_code = entry.get("outcome"), entry.get("errors"), entry.get("exit")
    if outcome != "failed":
        return f"`{module}` ended as `{outcome}`, not with the errors its baseline records"
    if not isinstance(errors, int) or errors < 1:
        return f"`{module}` reported {errors} errors, where its baseline records them"
    if exit_code != 1:
        return f"`{module}` ended with status {exit_code}, not the 1 its errors give"
    return "ok"


print(verdict(sys.argv[1], sys.argv[2]))
PY
}

# A stub driver beside a copy of this script: the copy runs its production path
# over a tree whose `verify-modules.py` is the stub, so nothing here is told it
# is being checked. Every case gives the verdict named with it.
self_test() {
  local directory
  directory=$(mktemp -d "${TMPDIR:-/tmp}/verex-compiled-namespaces-test.XXXXXX") || {
    echo "verify-compiled-namespaces: no temporary directory for the self-test" >&2
    return 2
  }
  mkdir -p "$directory/scripts" "$directory/verex-e2e-tests/VerexE2ETests/CompiledNamespaces" &&
    cp "${BASH_SOURCE[0]}" "$directory/scripts/verify-compiled-namespaces.sh" &&
    cp "$EXPECTED" "$directory/verex-e2e-tests/VerexE2ETests/CompiledNamespaces/client.exp" || {
      echo "verify-compiled-namespaces: the self-test's tree could not be made" >&2
      rm -rf "$directory"
      return 2
    }
  cat >"$directory/scripts/verify-modules.py" <<'STUB'
"""A driver that prints the expected report and then goes wrong in one way."""
import json, os, signal, sys

case = os.environ.get("VEREX_COMPILED_NAMESPACES_CASE", "normal")
arguments = sys.argv[1:]
package = arguments[0]


def option(name):
    return arguments[arguments.index(name) + 1] if name in arguments else None


result_file, logs = option("--json"), option("--logs")
with open(os.path.join(package, "client.exp")) as stream:
    report = stream.read()
sys.stdout.write("module                outcome     wall\n")
sys.stdout.write("fixture::client       failed      1.0s\n")
sys.stdout.write(report)
sys.stderr.write("verify-modules: 1 modules of CompiledNamespaces, 2 at a time\n")
os.makedirs(logs, exist_ok=True)
with open(os.path.join(logs, "fixture__client.out"), "w") as stream:
    stream.write(report)
module = {"module": "fixture::client", "outcome": "failed", "exit": 1, "errors": 2}
result = {"schema": "verex-module-sweep", "version": 1, "namespace_failures": [],
          "modules": [module]}
status = 1
if case == "status-0":
    status = 0
elif case == "status-9":
    status = 9
elif case == "timeout":
    module.update(outcome="timeout", exit=-9, errors=0)
elif case == "crashed":
    module.update(outcome="crashed", exit=2, errors=0)
elif case == "verified":
    module.update(outcome="verified", exit=0, errors=0)
elif case == "other-module":
    module["module"] = "fixture::base"
elif case == "namespace-failure":
    result["namespace_failures"] = ["0x42.base"]
elif case == "root-list":
    result = []
elif case == "entry-null":
    result["modules"] = [None]
elif case == "namespace-missing":
    del result["namespace_failures"]
elif case == "namespace-null":
    result["namespace_failures"] = None
elif case == "unjudgeable":
    # A result no reader gets through: nested past any recursion limit.
    with open(result_file, "w") as stream:
        stream.write("[" * 100000 + "]" * 100000)
    result_file = None
elif case == "no-result":
    result_file = None
elif case == "unparsable":
    with open(result_file, "w") as stream:
        stream.write("{ not a result")
    result_file = None
elif case == "panic-stderr":
    sys.stderr.write("PANIC at Lean.Meta.whnf, the verifier gave up\n")
elif case == "panic-log":
    with open(os.path.join(logs, "fixture__client.err"), "w") as stream:
        stream.write("PANIC at Lean.Meta.whnf, the verifier gave up\n")
elif case == "other-report":
    sys.stdout.write("sources/base.move:18:1: error: an error the baseline does not hold\n")
if result_file:
    with open(result_file, "w") as stream:
        stream.write(json.dumps(result))
sys.stdout.flush()
sys.stderr.flush()
if case == "signal":
    os.kill(os.getpid(), signal.SIGKILL)
sys.exit(status)
STUB
  local status=0
  # A case: its verdict, and what the verdict must say, so that a case cannot
  # pass for the wrong reason.
  verdict_case() { # case verdict reason
    local case_name=$1 wanted=$2 reason=$3 output code verdict note
    output=$(VEREX_COMPILED_NAMESPACES_CASE="$case_name" \
      "$directory/scripts/verify-compiled-namespaces.sh" --no-build 2>&1)
    code=$?
    verdict=FAIL
    [ $code = 0 ] && verdict=ok
    note="as expected"
    if [ "$verdict" != "$wanted" ]; then
      note="EXPECTED $wanted"
      status=1
    elif [ -n "$reason" ] && ! printf '%s' "$output" | grep -qF -- "$reason"; then
      note="EXPECTED the reason '$reason'"
      status=1
    fi
    printf '  %-20s %-4s %s\n' "$case_name" "$verdict" "$note"
    [ "$note" = "as expected" ] || printf '%s\n' "$output" | head -5 | sed 's/^/      /'
  }
  echo "verify-compiled-namespaces: the verdict's own cases"
  verdict_case normal            ok   ""
  verdict_case status-0          FAIL "ended with status 0"
  verdict_case status-9          FAIL "ended with status 9"
  verdict_case signal            FAIL "ended with status 137"
  verdict_case timeout           FAIL 'ended as `timeout`'
  verdict_case crashed           FAIL 'ended as `crashed`'
  verdict_case verified          FAIL 'ended as `verified`'
  verdict_case other-module      FAIL 'not `fixture::client`'
  verdict_case namespace-failure FAIL "namespaces that did not compile"
  verdict_case namespace-missing FAIL "does not say whether the namespaces compiled"
  verdict_case namespace-null    FAIL "namespaces that did not compile"
  verdict_case root-list         FAIL "is a list, not an object"
  verdict_case entry-null        FAIL "module is a NoneType, not an object"
  verdict_case no-result         FAIL "wrote no result"
  verdict_case unparsable        FAIL "does not parse"
  verdict_case unjudgeable       FAIL "could not be judged"
  verdict_case panic-stderr      FAIL "PANIC in what the run wrote"
  verdict_case panic-log         FAIL "PANIC in what the run wrote"
  verdict_case other-report      FAIL "is not verex-e2e-tests/"
  rm -rf "$directory"
  return $status
}

if [ $self_test_only = 1 ]; then
  self_test || { echo "verify-compiled-namespaces: the verdict fails its own cases" >&2; exit 2; }
  exit 0
fi

TMP=$(mktemp -d "${TMPDIR:-/tmp}/verex-compiled-namespaces.XXXXXX") || {
  echo "verify-compiled-namespaces: no temporary directory for the run" >&2
  exit 2
}
trap 'rm -rf "$TMP"' EXIT
# What the run writes stays under `run`, which the PANIC scan reads whole; what
# this script writes about the run stays beside it.
RUN="$TMP/run"
mkdir -p "$RUN" || {
  echo "verify-compiled-namespaces: cannot write in $TMP" >&2
  exit 2
}
# The driver runs as a background job in a process group of its own, so that an
# interrupted run stops exactly what it started and nothing of the caller's
# group, which this script is a member of and never signals. The group is
# interrupted rather than terminated: `verify-modules.py` turns an interrupt
# into the kill of the module processes it started in sessions of their own,
# which a signal to its group does not reach.
set -m
driver=
stop_driver() {
  [ -n "$driver" ] || return 0
  kill -INT -- "-$driver" 2>/dev/null
  # Reaped before the directory it writes in is removed.
  wait "$driver" 2>/dev/null
  driver=
}
trap 'trap "" INT TERM; stop_driver; exit 130' INT TERM

options=(--modules "$MODULE" --jobs 2 --namespaces "$RUN/namespaces" --logs "$RUN/logs"
  --json "$RUN/result.json")
[ $build = 1 ] || options+=(--no-build)
SECONDS=0
code=0
# A job turns job control off for itself, so that what it starts stays in its group.
(set +m; python3 "$LEAN_ROOT/scripts/verify-modules.py" "$PACKAGE" "${options[@]}") \
  >"$RUN/raw" 2>"$RUN/progress" &
driver=$!
wait "$driver" || code=$?
driver=
elapsed=$SECONDS

problems=()
# The fixture's errors give the driver status 1; every other status, a signal
# among them, is a run that did not do what the baseline records.
[ "$code" = 1 ] ||
  problems+=("the driver ended with status $code, not the 1 the fixture's reported errors give")
# Everything the run wrote, before the report is reduced: a panic need not
# reach the report. A scan that cannot read what the run wrote is not a scan
# that found nothing.
grep -rlI PANIC "$RUN" >"$TMP/panicking" 2>"$TMP/panicking.err"
scanned=$?
if [ $scanned -gt 1 ]; then
  problems+=("reading what the run wrote for PANIC failed (grep status $scanned): \
$(tail -1 "$TMP/panicking.err")")
elif [ $scanned = 0 ]; then
  problems+=("PANIC in what the run wrote: $(head -3 "$TMP/panicking" | tr '\n' ' ')")
fi
# The verdict on the driver's own result, which has to arrive to count.
verdict=$(result_verdict "$RUN/result.json" 2>"$TMP/verdict.err")
judged=$?
if [ $judged != 0 ] || [ -z "$verdict" ]; then
  problems+=("the driver's result could not be judged (the reader ended with status \
$judged): $(tail -1 "$TMP/verdict.err")")
elif [ "$verdict" != ok ]; then
  problems+=("$verdict")
fi
# The reduced report: the modules' rows and the driver's progress removed.
# Nothing left to print is a legitimate empty result (status 1), a failure of
# the filter itself is not.
grep -Ev '^(module |fixture::|verify-modules:)' "$RUN/raw" >"$TMP/report"
filtered=$?
if [ $filtered -gt 1 ]; then
  problems+=("reducing the driver's report failed (grep status $filtered)")
  : >"$TMP/report"
fi
# A report that differs is the fixture's own failure; a comparison that cannot
# be made at all is another.
differs=0
diff -q "$EXPECTED" "$TMP/report" >/dev/null 2>"$TMP/diff.err"
compared=$?
case $compared in
  0) ;;
  1) differs=1
     problems+=("the report of $MODULE is not ${EXPECTED#"$LEAN_ROOT"/}") ;;
  *) problems+=("comparing the report with ${EXPECTED#"$LEAN_ROOT"/} failed (diff status \
$compared): $(tail -1 "$TMP/diff.err")") ;;
esac

if [ ${#problems[@]} = 0 ]; then
  echo "compiled namespaces: ok (${elapsed}s)"
  exit 0
fi
echo "compiled namespaces: FAIL (${elapsed}s)"
printf '    %s\n' "${problems[@]}"
[ $differs = 0 ] || diff "$EXPECTED" "$TMP/report" | head -40 | sed 's/^/    /'
tail -5 "$RUN/progress" | sed 's/^/    /'
exit 1
