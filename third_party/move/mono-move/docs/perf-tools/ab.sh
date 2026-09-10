#!/bin/bash
# Interleaved A/B harness. Usage: ab.sh <reps> <labelA>=<binA> [<labelB>=<binB> ...]
#
# The machine is noisy one-sidedly: interference only ever makes a run slower.
# The workload is a deterministic replay, so the right estimator is the max over
# reps, not the median. Arms are interleaved rep by rep so any drift hits all of
# them equally.
set -euo pipefail
cd "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
W=${MONOPROF_WORK:?set MONOPROF_WORK to a scratch dir}
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
REPS=${1:?reps required}; shift
OUT=$W/ab.tsv
[ -f "$OUT" ] || printf 'label\tworkload\tmode\trep\toverall_tps\tinner_tps\texec_frac\ttxns\n' > "$OUT"

run_one() {
  local BIN=$1 LABEL=$2 WL=$3 MODE=$4 FLIP=$5 REP=$6
  local LOG=$W/ab-$LABEL-$WL-$MODE-$REP.log
  trash "$W/$WL-cp" 2>/dev/null || true
  "$BIN" \
    --block-executor-type aptos-vm-with-block-stm \
    --execution-threads 1 --generate-then-execute \
    --num-generator-workers 1 --block-size 500 \
    run-executor --data-dir "$W/$WL-recorded-db" --checkpoint-dir "$W/$WL-cp" \
    --replay-blocks "$W/$WL.blocks" $FLIP ENABLE_MONO_MOVE \
    > "$LOG" 2>&1 || { echo "FAILED $LABEL $WL $MODE rep$REP"; tail -5 "$LOG"; return 1; }
  python3 "$HERE/parse.py" "$LOG" "$LABEL" "$WL" "$MODE" "$REP" >> "$OUT"
}

for WL in bench-clob bench-aave; do
  for R in $(seq 1 "$REPS"); do
    for ARM in "$@"; do
      LABEL=${ARM%%=*}; BIN=${ARM#*=}
      run_one "$BIN" "$LABEL" "$WL" legacy --disable-feature-after-init "$R"
      run_one "$BIN" "$LABEL" "$WL" mono   --enable-feature-after-init  "$R"
    done
  done
done

exec "$HERE/report.py" "$OUT" "$@"
