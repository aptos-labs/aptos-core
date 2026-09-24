#!/usr/bin/env bash
# SCRATCH-BRANCH EXPERIMENT, DO NOT MERGE.
#
# Runner confirmation for the aligned + interleaved wall-clock gate.
#
#   layout_experiment.sh layout-v0|layout-v1|layout-v2
#       R1: time the gated programs (and native controls) across code layouts of
#       identical source. Layouts come from a padding blob linked first
#       (LAYOUT_PAD, shifts all downstream code) and from nops injected at the
#       top of the interpreter dispatch loop (LAYOUT_NOPS, shifts its internals).
#       v0 = default flags, v1 = -align-all-functions=6, v2 = v1 plus
#       -align-all-nofallthru-blocks=5. 3 interleaved processes per layout.
#
#   layout_experiment.sh noise
#       R2 + R4: one v1 binary run as many separate processes under four process
#       settings (plain, setarch -R, taskset pin, both), interleaved, at work
#       scale 1.0 and 1.25 (synthetic ~25% regression).
#
# Results: $OUT_DIR/results.jsonl (one line per program per process) and
# $OUT_DIR/env.txt. SMOKE=1 shrinks everything for a local dry run.

set -euo pipefail

MODE="${1:?usage: layout_experiment.sh layout-v0|layout-v1|layout-v2|noise}"
REPO_ROOT="$(git rev-parse --show-toplevel)"
OUT_DIR="${OUT_DIR:-$REPO_ROOT/target/layout-exp-out/$MODE}"
TARGET_DIR="${LAYOUT_TARGET_DIR:-$REPO_ROOT/target/layout-exp-$MODE}"
BIN_DIR="$OUT_DIR/bin"
RESULTS="$OUT_DIR/results.jsonl"
mkdir -p "$BIN_DIR"
: > "$RESULTS"
cd "$REPO_ROOT"

# RUSTFLAGS replaces every config-file rustflags list, so repeat the target's.
if [[ "$(uname -s)-$(uname -m)" == "Linux-x86_64" ]]; then
    BASE_FLAGS="--cfg tokio_unstable -C link-arg=-fuse-ld=lld -C force-frame-pointers=yes -C force-unwind-tables=yes -C target-cpu=x86-64-v3"
else
    BASE_FLAGS="--cfg tokio_unstable -C force-frame-pointers=yes -C force-unwind-tables=yes"
fi
ALIGN_FUNCTIONS="-C llvm-args=-align-all-functions=6"
ALIGN_BLOCKS="-C llvm-args=-align-all-nofallthru-blocks=5"

if [[ -n "${SMOKE:-}" ]]; then
    PADS="0 16"
    NOPS=""
    LAYOUT_REPS=1
    NOISE_REPS=2
    SCALED_REPS=1
else
    PADS="0 16 32 48 64 192 448 960 1984 4032"
    NOPS="4 8 12 16 20 24 28 32 36 40 44 48 52 56 60"
    LAYOUT_REPS=3
    NOISE_REPS=20
    SCALED_REPS=10
fi

env_info() {
    echo "mode: $MODE"
    echo "commit: $(git rev-parse HEAD)"
    uname -a
    command -v lscpu >/dev/null && lscpu
    echo "nproc: $(nproc 2>/dev/null || getconf _NPROCESSORS_ONLN)"
    for file in /sys/kernel/mm/transparent_hugepage/enabled \
        /sys/kernel/mm/transparent_hugepage/defrag \
        /proc/sys/kernel/randomize_va_space \
        /sys/devices/system/cpu/cpu0/cpufreq/scaling_governor \
        /sys/fs/cgroup/cpu.max /sys/fs/cgroup/cpuset.cpus.effective; do
        [[ -r "$file" ]] && echo "$file: $(cat "$file")"
    done
    command -v taskset >/dev/null && taskset -pc $$
    true
}

# build <name> <rustflags> <pad> <nops>
build() {
    local name="$1" flags="$2" pad="$3" nops="$4"
    echo "::group::build $name" >&2
    LAYOUT_PAD="$pad" LAYOUT_NOPS="$nops" RUSTFLAGS="$flags" CARGO_TARGET_DIR="$TARGET_DIR" \
        cargo build --profile bench -p mono-move-testsuite --example layout_exp >&2
    echo "::endgroup::" >&2
    cp "$TARGET_DIR/release/examples/layout_exp" "$BIN_DIR/$name"
    # Proves the layout actually moved: these addresses should differ per pad/nops.
    nm "$BIN_DIR/$name" | grep -E '10native_fib$|18InterpreterContext3run$|layout_exp_pad$' \
        | sed "s/^/[$name] /" | tee -a "$OUT_DIR/symbols.txt" >&2 || true
}

# run_one <meta-json> <cmd...>
run_one() {
    local meta="$1"; shift
    "$@" | python3 -c '
import json, sys
meta = json.loads(sys.argv[1])
for line in sys.stdin:
    print(json.dumps({**json.loads(line), **meta}))
' "$meta" >> "$RESULTS"
}

shuffled() {
    # shuffled <seed> <words...>
    local seed="$1"; shift
    python3 -c 'import random, sys; words = sys.argv[2:]; random.Random(int(sys.argv[1])).shuffle(words); print(" ".join(words))' "$seed" "$@"
}

cmd_layout() {
    local variant="${MODE#layout-}" flags
    case "$variant" in
        v0) flags="$BASE_FLAGS" ;;
        v1) flags="$BASE_FLAGS $ALIGN_FUNCTIONS" ;;
        v2) flags="$BASE_FLAGS $ALIGN_FUNCTIONS $ALIGN_BLOCKS" ;;
        *) echo "unknown variant $variant" >&2; exit 2 ;;
    esac
    echo "RUSTFLAGS: $flags" | tee -a "$OUT_DIR/env.txt"

    local names=() pad nops
    for pad in $PADS; do
        build "pad${pad}_nops0" "$flags" "$pad" 0
        names+=("pad${pad}_nops0")
    done
    for nops in $NOPS; do
        build "pad0_nops${nops}" "$flags" 0 "$nops"
        names+=("pad0_nops${nops}")
    done

    local rep name
    for rep in $(seq 1 "$LAYOUT_REPS"); do
        for name in $(shuffled "$rep" "${names[@]}"); do
            pad="${name#pad}"; pad="${pad%%_*}"; nops="${name##*nops}"
            run_one "{\"variant\": \"$variant\", \"pad\": $pad, \"nops\": $nops, \"rep\": $rep}" \
                "$BIN_DIR/$name"
        done
        echo "layout rep $rep done" >&2
    done
}

cmd_noise() {
    local flags="$BASE_FLAGS $ALIGN_FUNCTIONS"
    echo "RUSTFLAGS: $flags" | tee -a "$OUT_DIR/env.txt"
    build "v1" "$flags" 0 0

    local settings="plain"
    local setarch_cmd=(setarch "$(uname -m)" -R)
    local cpu=""
    if "${setarch_cmd[@]}" true 2>/dev/null; then
        settings="$settings setarch"
    else
        echo "setarch -R unavailable; skipping" | tee -a "$OUT_DIR/env.txt"
    fi
    if command -v taskset >/dev/null; then
        # Third CPU of this process's allowed set, away from CPU 0 housekeeping.
        cpu=$(taskset -pc $$ | sed 's/.*: //' | python3 -c '
import sys
cpus = []
for part in sys.stdin.read().strip().split(","):
    lo, _, hi = part.partition("-")
    cpus += range(int(lo), int(hi or lo) + 1)
print(cpus[min(2, len(cpus) - 1)])
')
        echo "taskset cpu: $cpu" | tee -a "$OUT_DIR/env.txt"
        settings="$settings taskset"
        [[ "$settings" == *setarch* ]] && settings="$settings both"
    else
        echo "taskset unavailable; skipping" | tee -a "$OUT_DIR/env.txt"
    fi

    local rep setting scale prefix
    for rep in $(seq 1 "$NOISE_REPS"); do
        for setting in $(shuffled "$rep" $settings); do
            for scale in 1.0 1.25; do
                [[ "$scale" == "1.25" && "$rep" -gt "$SCALED_REPS" ]] && continue
                case "$setting" in
                    plain) prefix=() ;;
                    setarch) prefix=("${setarch_cmd[@]}") ;;
                    taskset) prefix=(taskset -c "$cpu") ;;
                    both) prefix=(taskset -c "$cpu" "${setarch_cmd[@]}") ;;
                esac
                run_one "{\"variant\": \"v1\", \"setting\": \"$setting\", \"scale\": $scale, \"rep\": $rep}" \
                    env LAYOUT_ONLY_MONO=1 LAYOUT_WORK_SCALE="$scale" ${prefix[@]+"${prefix[@]}"} "$BIN_DIR/v1"
            done
        done
        echo "noise rep $rep done" >&2
    done
}

env_info | tee "$OUT_DIR/env.txt"
case "$MODE" in
    layout-*) cmd_layout ;;
    noise) cmd_noise ;;
    *) echo "unknown mode $MODE" >&2; exit 2 ;;
esac
echo "results: $(wc -l < "$RESULTS") lines in $RESULTS"
