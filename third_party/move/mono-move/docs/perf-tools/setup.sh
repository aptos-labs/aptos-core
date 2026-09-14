#!/bin/bash
set -euo pipefail
cd "$(git -C "$(dirname "${BASH_SOURCE[0]}")" rev-parse --show-toplevel)"
W=${MONOPROF_WORK:?set MONOPROF_WORK to a scratch dir}
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
BIN=target/release/aptos-executor-benchmark

if [ ! -d "$W/db" ]; then
  echo "=== create-db ==="
  RUST_BACKTRACE=1 $BIN \
    --block-executor-type aptos-vm-with-block-stm \
    --block-size 500 --execution-threads 8 \
    create-db --data-dir "$W/db" --num-accounts 22000
fi

for WL in bench-clob bench-aave; do
  if [ ! -f "$W/$WL.blocks" ]; then
    echo "=== record $WL ==="
    RUST_BACKTRACE=1 $BIN \
      --block-executor-type aptos-vm-with-block-stm \
      --execution-threads 1 --generate-then-execute \
      --num-generator-workers 1 --block-size 500 \
      run-executor --data-dir "$W/db" --checkpoint-dir "$W/$WL-recorded-db" \
      --transaction-type $WL --module-working-set-size 1 \
      --main-signer-accounts 1000 --additional-dst-pool-accounts 30000 \
      --blocks 30 --dump-blocks "$W/$WL.blocks"
  fi
done
echo "SETUP DONE"
