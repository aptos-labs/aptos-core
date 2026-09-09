#!/bin/bash
set -euo pipefail
cd "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
W=${MONOPROF_WORK:?set MONOPROF_WORK to a scratch dir}
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BIN=target/release/aptos-executor-benchmark

run() {
  local WL=$1 MODE=$2 FLIP=$3
  echo "=== $WL $MODE ==="
  samply record --save-only --unstable-presymbolicate --rate 999 -o "$W/$WL-$MODE.json.gz" -- \
    $BIN \
    --block-executor-type aptos-vm-with-block-stm \
    --execution-threads 1 --generate-then-execute \
    --num-generator-workers 1 --block-size 500 \
    run-executor --data-dir "$W/$WL-recorded-db" --checkpoint-dir "$W/$WL-cp" \
    --replay-blocks "$W/$WL.blocks" $FLIP ENABLE_MONO_MOVE \
    > "$W/$WL-$MODE.log" 2>&1
  echo "--- stage lines ---"
  grep -E "fraction of|TPS: |GPT:|output: " "$W/$WL-$MODE.log" | tail -30 || true
}

for WL in bench-clob bench-aave; do
  run $WL legacy --disable-feature-after-init
  run $WL mono   --enable-feature-after-init
done
echo "PROFILES DONE"
