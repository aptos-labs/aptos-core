#!/usr/bin/env bash
#
# Reproduces the A/B benchmark for the `Validate::No` -> `Validate::Yes` change in
# `ark_de` (crates/aptos-crypto/src/arkworks/serialization.rs).
#
# That one-line change enables arkworks subgroup-membership checks on every curve point
# deserialized through serde, which is the only code path it affects: `deal`, `verify`,
# `aggregate_with` and `decrypt_own_share` never call `ark_de`. This script measures the cost
# on the chunky v1 weighted PVSS benchmarks.
#
# It runs four passes -- {Validate::No, Validate::Yes} x {1 thread, all threads} -- and prints
# criterion's per-benchmark percentage change for each thread configuration.
#
# `serialize/` and `verify/` are controls: neither goes through `ark_de` in its timed region,
# so both must come back "No change in performance detected". If either moves, the measurement
# is contaminated (thermal throttling, background load) and should be discarded.
#
# Usage:
#   crates/aptos-dkg/scripts/bench_validate_ab.sh                 # both thread configs
#   THREAD_MODES=serial crates/aptos-dkg/scripts/bench_validate_ab.sh
#   SAMPLE_SIZE=50 MEASUREMENT_TIME=10 crates/aptos-dkg/scripts/bench_validate_ab.sh
#
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

SER_FILE="crates/aptos-crypto/src/arkworks/serialization.rs"
BENCH_FILTER="${BENCH_FILTER:-serialize|verify}"
SAMPLE_SIZE="${SAMPLE_SIZE:-30}"
WARM_UP_TIME="${WARM_UP_TIME:-2}"
MEASUREMENT_TIME="${MEASUREMENT_TIME:-5}"
THREAD_MODES="${THREAD_MODES:-serial parallel}"
OUT_DIR="${OUT_DIR:-/tmp/pvss-validate-ab}"

mkdir -p "$OUT_DIR"

# Refuse to run against a dirty copy of the file we are about to rewrite, so a failure can never
# lose real work.
if ! git diff --quiet -- "$SER_FILE"; then
    echo "error: $SER_FILE has uncommitted changes; commit or stash them first." >&2
    exit 1
fi

if ! grep -q 'Compress::Yes, Validate::Yes' "$SER_FILE"; then
    echo "error: expected 'Compress::Yes, Validate::Yes' in $SER_FILE (is the fix applied?)." >&2
    exit 1
fi

# Always put the fix back, including on Ctrl-C or an aborted benchmark.
restore_fix() { git checkout -- "$SER_FILE"; }
trap restore_fix EXIT

set_validate() {
    # $1 is "No" or "Yes". Matching on the full "Compress::Yes, Validate::X" pair keeps this from
    # touching `ark_de_uncompressed_no_validate`, which legitimately uses Compress::No/Validate::No.
    restore_fix
    if [[ "$1" == "No" ]]; then
        perl -pi -e 's/Compress::Yes, Validate::Yes/Compress::Yes, Validate::No/' "$SER_FILE"
    fi
    grep -n 'Compress::Yes, Validate::' "$SER_FILE"
}

# caffeinate keeps macOS from napping mid-run; harmless to skip elsewhere.
CAFFEINATE=()
command -v caffeinate >/dev/null 2>&1 && CAFFEINATE=(caffeinate -dimsu)

run_arm() {
    local mode="$1" baseline_arg="$2" logfile="$3"
    local criterion_home="$REPO_ROOT/target/criterion-validate-ab-$mode"

    local -a env_prefix=(
        "CRITERION_HOME=$criterion_home"
        "DKG_BENCH_GROUP=chunky_v1"
    )
    [[ "$mode" == "serial" ]] && env_prefix+=("RAYON_NUM_THREADS=1")

    env "${env_prefix[@]}" "${CAFFEINATE[@]}" \
        cargo bench -p aptos-dkg --bench pvss -- \
        --sample-size "$SAMPLE_SIZE" \
        --warm-up-time "$WARM_UP_TIME" \
        --measurement-time "$MEASUREMENT_TIME" \
        $baseline_arg \
        "$BENCH_FILTER" 2>&1 | tee "$logfile"
}

for mode in $THREAD_MODES; do
    echo
    echo "=============================================================="
    echo " $mode : Validate::No  (pre-fix baseline)"
    echo "=============================================================="
    set_validate No
    run_arm "$mode" "--save-baseline validate_no" "$OUT_DIR/${mode}_validate_no.txt"

    echo
    echo "=============================================================="
    echo " $mode : Validate::Yes (the fix), compared against baseline"
    echo "=============================================================="
    set_validate Yes
    run_arm "$mode" "--baseline validate_no" "$OUT_DIR/${mode}_validate_yes.txt"

    echo
    echo "----- summary ($mode) -----"
    grep -E "^pvss/chunky_v1|time: +\[|change:|Performance has|No change" \
        "$OUT_DIR/${mode}_validate_yes.txt" || true
done

echo
echo "Logs written to $OUT_DIR"
echo "Reminder: 'serialize/' and 'verify/' are controls and must show no change."
