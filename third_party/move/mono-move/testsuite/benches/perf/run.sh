#!/usr/bin/env bash
# Copyright (c) Aptos Foundation
# Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
#
# A/B driver for the mono-move bench regression gate.
#
#   run.sh ab <base_ref> <head> [--out <file>]
#       Build the gated benches at <head> and at the merge-base of <head> and
#       <base_ref>, each in its own temporary git worktree, run them as
#       alternating separate processes (base, head, head, base, ...), then gate
#       via compare.py.
#       Exits 0 on pass, 1 on a regression, 2 on a tooling failure (bad config,
#       a side that does not build, a bench that fails or leaves incomplete
#       results). Any other non-zero code is an unexpected error, not a regression.
#       `run.sh ab <sha> <sha>` is an A/A run that measures noise.
#
# Both sides are built with every function, and every block not entered by
# fall-through, aligned to 64 bytes (the prebuilt standard library keeps its
# default alignment). Without that, unrelated code changes can swing the benches
# by large amounts just by shifting where hot code lands relative to the CPU's
# 64-byte fetch chunks.
#
# CWD must be inside the target git repo; its own checkout is never switched.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel)"
CONFIG="$SCRIPT_DIR/config.json"
PKG="mono-move-testsuite"
TESTSUITE_DIR="third_party/move/mono-move/testsuite"
# Separate from the default target dir: the alignment flags would otherwise
# invalidate regular build artifacts.
TARGET_DIR="${MONO_BENCH_TARGET_DIR:-$REPO_ROOT/target/mono-bench-gate}"
ALIGN_FLAGS='["-C", "llvm-args=-align-all-functions=6", "-C", "llvm-args=-align-all-nofallthru-blocks=6"]'

WORK=""
WORKTREES=()
# Set by load_config.
PROCESSES=""
GATED_IDS=()
GATED_BENCHES=()

cleanup() {
    local tree
    for tree in ${WORKTREES[@]+"${WORKTREES[@]}"}; do
        git -C "$REPO_ROOT" worktree remove --force "$tree" >/dev/null 2>&1 || true
    done
    [[ -n "$WORK" ]] && rm -rf "$WORK"
}

cmd_ab() {
    local base_ref="$1" head_arg="$2" out="$3"
    load_config

    # Cargo ignores all config rustflags, including the alignment flags, when
    # either variable is set, even to an empty value.
    if [[ -n "${RUSTFLAGS+set}${CARGO_ENCODED_RUSTFLAGS+set}" ]]; then
        echo "error: unset RUSTFLAGS and CARGO_ENCODED_RUSTFLAGS; they override the bench build flags" >&2
        exit 2
    fi

    if [[ "$base_ref" == origin/* ]]; then
        git -C "$REPO_ROOT" fetch --quiet origin \
            "+refs/heads/${base_ref#origin/}:refs/remotes/$base_ref" || true
    fi
    git -C "$REPO_ROOT" rev-parse --verify --quiet "$base_ref^{commit}" >/dev/null || {
        echo "error: base ref '$base_ref' does not exist (base branch deleted or retargeted?)" >&2
        exit 2; }

    local head_sha base_sha
    head_sha=$(git -C "$REPO_ROOT" rev-parse --verify "$head_arg^{commit}") || {
        echo "error: cannot resolve head '$head_arg'" >&2; exit 2; }
    base_sha=$(git -C "$REPO_ROOT" merge-base "$head_sha" "$base_ref") || {
        echo "error: cannot compute merge-base of $head_sha and $base_ref" >&2; exit 2; }
    echo ">> base (merge-base of head and $base_ref): $base_sha" >&2
    echo ">> head: $head_sha" >&2

    WORK=$(mktemp -d "${RUNNER_TEMP:-${TMPDIR:-/tmp}}/mono-bench-gate.XXXXXX")
    trap cleanup EXIT

    # Head first, so a PR that does not compile fails before the base is built.
    # Each worktree is created right before its build, so its sources are newer
    # than every artifact in the shared target dir. Cargo identifies workspace
    # crates by workspace-relative path and checks freshness by mtime, so this
    # ordering is what forces a full rebuild of each side's workspace crates.
    add_worktree head "$head_sha"
    build_side head "$TREE" || { echo "error: head ($head_sha) does not build" >&2; exit 2; }
    add_worktree base "$base_sha"
    build_side base "$TREE" || { echo "error: base ($base_sha) does not build" >&2; exit 2; }

    local pass side order
    for pass in $(seq 1 "$PROCESSES"); do
        if (( pass % 2 )); then order="base head"; else order="head base"; fi
        for side in $order; do
            run_side "$side" "$pass"
        done
    done

    local changed
    changed=$(workload_changed "$base_sha" "$head_sha") || {
        echo "error: cannot diff bench sources between $base_sha and $head_sha" >&2; exit 2; }
    local args=(ab --results-dir "$WORK/results" --base-sha "$base_sha" --head-sha "$head_sha"
        --workload-changed "$changed")
    [[ -n "$out" ]] && args+=(--out "$out")
    python3 "$SCRIPT_DIR/compare.py" "${args[@]}"
}

# Sets TREE to a new worktree of <sha>.
add_worktree() {
    local side="$1" sha="$2"
    TREE="$WORK/tree-$side"
    git -C "$REPO_ROOT" worktree add --quiet --detach "$TREE" "$sha" >&2
    WORKTREES+=("$TREE")
}

# Builds the bench binaries in <tree> and copies them to $WORK/bin/<side>/<bench>.
# The binaries locate their Move sources through the tree they were built in, so
# the tree must outlive the runs.
build_side() {
    local side="$1" tree="$2" host key bench targets=()
    mkdir -p "$WORK/bin/$side"
    for bench in "${GATED_BENCHES[@]}"; do
        [[ -f "$tree/$TESTSUITE_DIR/benches/$bench.rs" ]] && targets+=(--bench "$bench")
    done
    if (( ${#targets[@]} == 0 )); then
        echo ">> no gated bench exists at $side; nothing to build" >&2
        return 0
    fi

    host=$(cd "$tree" && rustc -vV | sed -n 's/^host: //p')
    # Rustflags from a target section replace `build.rustflags` entirely, so the
    # alignment flags go wherever the base flags for this host live. Both keys are
    # appended to, not overwritten, by `--config`.
    key="build.rustflags"
    if config_has_target_rustflags "$tree/.cargo/config.toml" "$host"; then
        key="target.$host.rustflags"
    fi

    echo "::group::cargo bench --no-run ($side)" >&2
    local rc=0 log="$WORK/build-$side.log"
    # Compiler diagnostics go to the job log, minus the long `--verbose` rustc
    # command lines, which are kept in $log for the flag check below. Only
    # artifact messages go to the JSON file.
    (cd "$tree" && CARGO_TARGET_DIR="$TARGET_DIR" \
        cargo bench -p "$PKG" --no-run --verbose --message-format=json-render-diagnostics \
            --config "$key=$ALIGN_FLAGS" "${targets[@]}" 2>&1 >"$WORK/build-$side.json") \
        | tee "$log" | { grep -v '^ *Running `' >&2 || true; } || rc=$?
    echo "::endgroup::" >&2
    (( rc == 0 )) || return "$rc"

    # A user-level cargo config or a `cfg(...)` target section can drop the flags
    # silently, so check the interpreter crate's actual rustc invocation.
    if ! grep -e '--crate-name mono_move_runtime ' "$log" \
        | grep -e 'align-all-functions=6' | grep -qe 'align-all-nofallthru-blocks=6'; then
        echo "error: the alignment flags did not reach rustc for mono_move_runtime on $side" >&2
        exit 2
    fi

    python3 - "$WORK/build-$side.json" "$WORK/bin/$side" <<'EOF'
import json
import shutil
import sys

messages, bin_dir = sys.argv[1], sys.argv[2]
for line in open(messages):
    message = json.loads(line)
    if (message.get("reason") == "compiler-artifact"
            and "bench" in message["target"]["kind"]
            and message.get("executable")):
        shutil.copy2(message["executable"], f"{bin_dir}/{message['target']['name']}")
EOF
}

# Runs every gated bench of <side> once, as one process per bench binary, with
# criterion output under $WORK/results/<side>/<pass>.
run_side() {
    local side="$1" pass="$2" bench
    for bench in "${GATED_BENCHES[@]}"; do
        # A bench binary missing on one side is reported by compare.py as new/absent.
        [[ -x "$WORK/bin/$side/$bench" ]] || continue
        echo "::group::$side pass $pass: $bench" >&2
        local rc=0
        CRITERION_HOME="$WORK/results/$side/$pass" \
            "$WORK/bin/$side/$bench" --bench --noplot "$(bench_filter "$bench")" >&2 || rc=$?
        echo "::endgroup::" >&2
        if (( rc != 0 )); then
            echo "error: bench '$bench' failed on $side (exit $rc)" >&2
            exit 2
        fi
    done
}

# Prints the comma-separated gated ids whose bench, program wrapper, or Move
# program differs between the two commits: base and head measure different work
# there, so the delta is not a regression signal.
workload_changed() {
    local base_sha="$1" head_sha="$2" id bench rc changed=()
    for id in "${GATED_IDS[@]}"; do
        bench="${id%%/*}"
        rc=0
        git -C "$REPO_ROOT" diff --quiet "$base_sha" "$head_sha" -- \
            "$TESTSUITE_DIR/benches/$bench.rs" \
            "$TESTSUITE_DIR/src/programs/$bench.rs" \
            "$TESTSUITE_DIR/tests/test_cases/differential/programs/$bench.move" || rc=$?
        case "$rc" in
            0) ;;
            1) changed+=("$id") ;;
            *) return 2 ;;
        esac
    done
    local IFS=,
    echo "${changed[*]+"${changed[*]}"}"
}

# Reads and validates config.json once, before anything is built. Sets PROCESSES,
# GATED_IDS, and GATED_BENCHES. A gated id is `<group>/<function>`, and its
# criterion group must equal the bench target (file stem) that defines it.
load_config() {
    local parsed
    parsed=$(python3 - "$CONFIG" <<'PY'
import json
import re
import sys

cfg = json.load(open(sys.argv[1]))
processes = cfg["processes_per_side"]
if not isinstance(processes, int) or processes < 1:
    sys.exit("processes_per_side must be a positive integer")
for key in ("threshold_percent", "notable_percent"):
    if not isinstance(cfg[key], (int, float)):
        sys.exit(f"{key} must be a number")
ids = [entry["id"] for entry in cfg["mono_benches"]]
if not ids:
    sys.exit("mono_benches is empty")
for bench_id in ids:
    if not re.fullmatch(r"[A-Za-z0-9_]+/[A-Za-z0-9_]+", bench_id):
        sys.exit(f"invalid bench id {bench_id!r}; expected <group>/<function>")
print(processes)
print(" ".join(ids))
print(" ".join(dict.fromkeys(bench_id.split("/")[0] for bench_id in ids)))
PY
    ) || { echo "error: invalid $CONFIG" >&2; exit 2; }
    { read -r PROCESSES; read -r -a GATED_IDS; read -r -a GATED_BENCHES; } <<< "$parsed"
}

# Whether <config> sets `rustflags` in its `[target.<host>]` section.
config_has_target_rustflags() {
    local config="$1" host="$2"
    [[ -f "$config" ]] || return 1
    awk -v header="[target.$host]" '
        {
            line = $0
            sub(/[[:space:]]*#.*$/, "", line)
            sub(/^[[:space:]]+/, "", line)
        }
        line ~ /^\[/ { in_section = (line == header); next }
        in_section && line ~ /^rustflags[[:space:]]*=/ { found = 1 }
        END { exit !found }
    ' "$config"
}

# Criterion regex selecting exactly the gated ids of <bench>.
bench_filter() {
    local bench="$1" id alternatives=""
    for id in "${GATED_IDS[@]}"; do
        [[ "$id" == "$bench/"* ]] && alternatives+="${alternatives:+|}$id"
    done
    echo "^($alternatives)\$"
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
            [[ ${#positional[@]} -ge 2 ]] || { echo "usage: run.sh ab <base_ref> <head> [--out <file>]" >&2; exit 2; }
            cmd_ab "${positional[0]}" "${positional[1]}" "$out"
            ;;
        *)
            echo "usage: run.sh ab <base_ref> <head> [--out <file>]" >&2
            exit 2
            ;;
    esac
}

main "$@"
