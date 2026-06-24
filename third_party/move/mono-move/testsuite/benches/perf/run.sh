#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#
# Same-job A/B driver for the mono-move bench regression gate.
#
#   run.sh ab <base_ref> <head> [--out <file>]
#       Run the mono benches on the merge-base of <head> and <base_ref> (typically
#       origin/main), saved as criterion baseline "main", then on <head> compared
#       against it, then gate via compare.py. Exits non-zero on a regression.
#
#   run.sh calibrate-noise [--out <file>]
#       Run the benches twice on the current checkout (identical code both times)
#       and report the runner's noise floor, to pick threshold_pct in config.json.
#
# Because `ab` checks out other refs in the repo, run this script from a COPY
# placed OUTSIDE the working tree (e.g. $RUNNER_TEMP/perf), or the checkout will
# replace the script while it executes. CWD must be inside the target git repo.
#
# Limitation: if a PR changes a bench's parameters (e.g. a `const N`), the base
# and head runs measure different work and that bench's comparison is not
# meaningful -- criterion has no knowledge of those constants. Review such PRs by
# hand. The gate still protects every unchanged bench.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel)"
BENCH_DIR="$REPO_ROOT/third_party/move/mono-move/testsuite/benches"
CRIT_DIR="$REPO_ROOT/target/criterion"
PKG="mono-move-testsuite"

# List the [[bench]] names defined at the current checkout (file stems under
# benches/, excluding the perf/ tooling dir which holds no .rs files).
list_benches() {
    find "$BENCH_DIR" -maxdepth 1 -name '*.rs' -exec basename {} .rs \; | sort
}

run_bench() {
    # run_bench <name> <criterion-arg...>; returns cargo's exit code.
    local name="$1"; shift
    echo "::group::cargo bench $name $*" >&2
    local rc=0
    (cd "$REPO_ROOT" && cargo bench -p "$PKG" --bench "$name" -- "$@") || rc=$?
    echo "::endgroup::" >&2
    return $rc
}

contains() {
    # contains <needle> <haystack-words...>
    local needle="$1"; shift
    local w
    for w in "$@"; do [[ "$w" == "$needle" ]] && return 0; done
    return 1
}

clean_criterion() {
    # target/criterion persists on the self-hosted runner; wipe it so a stale
    # change/ dir from a renamed or removed bench can't produce a false verdict.
    # Guarded so a bad REPO_ROOT can't become an unbounded delete.
    if [[ -n "$REPO_ROOT" && "$CRIT_DIR" == "$REPO_ROOT/target/criterion" ]]; then
        rm -rf "$CRIT_DIR"
    fi
}

cmd_ab() {
    local base_ref="$1" head_arg="$2" out="$3"

    clean_criterion

    # Make `origin/main` resolvable; the merge-base below fails loudly if not.
    git -C "$REPO_ROOT" fetch --quiet origin main:refs/remotes/origin/main || true

    # Resolve head up front: after the base checkout, "HEAD" points at the base.
    local head_sha
    head_sha=$(git -C "$REPO_ROOT" rev-parse --verify "$head_arg^{commit}") || {
        echo "error: cannot resolve head '$head_arg'" >&2; exit 2; }

    local base_sha
    base_sha=$(git -C "$REPO_ROOT" merge-base "$head_sha" "$base_ref") || {
        echo "error: cannot compute merge-base of $head_sha and $base_ref" >&2
        exit 2; }

    echo ">> base (merge-base of head and $base_ref): $base_sha" >&2
    git -C "$REPO_ROOT" checkout --quiet --detach "$base_sha"
    local base_benches; base_benches=$(list_benches)
    local b
    for b in $base_benches; do run_bench "$b" --save-baseline main; done

    echo ">> head: $head_sha" >&2
    git -C "$REPO_ROOT" checkout --quiet --detach "$head_sha"
    local head_benches; head_benches=$(list_benches)
    for b in $head_benches; do
        if ! contains "$b" $base_benches; then
            # New bench file on this PR: no baseline exists, so just record it.
            run_bench "$b" --save-baseline new
        elif ! run_bench "$b" --baseline main; then
            # --baseline panics if a baseline is missing (a new bench function in
            # an existing file); re-record without comparison instead of failing.
            echo ">> '$b': missing baseline (new bench function?); recording without comparison" >&2
            run_bench "$b" --save-baseline new
        fi
    done

    local args=(ab --criterion-dir "$CRIT_DIR")
    [[ -n "$out" ]] && args+=(--out "$out")
    python3 "$SCRIPT_DIR/compare.py" "${args[@]}"
}

cmd_calibrate_noise() {
    clean_criterion
    local benches; benches=${MONO_MOVE_BENCHES:-$(list_benches)}
    local b
    for b in $benches; do run_bench "$b" --save-baseline main; done
    for b in $benches; do run_bench "$b" --baseline main; done
    python3 "$SCRIPT_DIR/compare.py" calibrate-noise --criterion-dir "$CRIT_DIR"
}

main() {
    local cmd="${1:-}"; shift || true
    local out=""
    local positional=()
    while [[ $# -gt 0 ]]; do
        case "$1" in
            --out) out="$2"; shift 2 ;;
            *) positional+=("$1"); shift ;;
        esac
    done

    case "$cmd" in
        ab)
            [[ ${#positional[@]} -ge 2 ]] || { echo "usage: run.sh ab <base_ref> <head_sha> [--out <file>]" >&2; exit 2; }
            cmd_ab "${positional[0]}" "${positional[1]}" "$out"
            ;;
        calibrate-noise)
            cmd_calibrate_noise
            ;;
        *)
            echo "usage: run.sh {ab|calibrate-noise} ..." >&2
            exit 2
            ;;
    esac
}

main "$@"
