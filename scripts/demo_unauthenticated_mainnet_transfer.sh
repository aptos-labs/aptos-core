#!/usr/bin/env bash
# Demo: mainnet-fork session + --unauthenticated transfer + view
#
# Requires a locally built aptos CLI that includes --unauthenticated support.
# Usage:
#   ./scripts/demo_unauthenticated_mainnet_transfer.sh
#   APTOS_BIN=./target/debug/aptos ./scripts/demo_unauthenticated_mainnet_transfer.sh

set -euo pipefail

APTOS_BIN="${APTOS_BIN:-}"
if [[ -z "${APTOS_BIN}" ]]; then
  if [[ -x "./target/debug/aptos" ]]; then
    APTOS_BIN="./target/debug/aptos"
  elif [[ -x "./target/release/aptos" ]]; then
    APTOS_BIN="./target/release/aptos"
  else
    echo "error: build the CLI first, e.g. cargo build -p aptos" >&2
    echo "       or set APTOS_BIN=/path/to/aptos" >&2
    exit 1
  fi
fi

SESSION="${SESSION:-/tmp/aptos-unauth-mainnet-demo}"
# Real mainnet account with APT (from the original bug report). No private key needed.
SENDER="${SENDER:-0x8694b4b543b37c7593887721ba3144107de77787ee1ba2c527a673a3b2742027}"
# Fresh recipient — aptos_account::transfer will create it if missing.
RECIPIENT="${RECIPIENT:-0xaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa}"
AMOUNT_OCTA="${AMOUNT_OCTA:-1}"

view_balance() {
  local who="$1"
  local label="$2"
  echo
  echo "=== view ${label} APT balance (${who}) ==="
  "${APTOS_BIN}" move view \
    --session "${SESSION}" \
    --function-id 0x1::coin::balance \
    --type-args 0x1::aptos_coin::AptosCoin \
    --args "address:${who}"
}

echo "Using CLI: ${APTOS_BIN}"
"${APTOS_BIN}" --version || true

# Fresh session each run
rm -rf "${SESSION}"

echo
echo "=== 1) init mainnet fork session at ${SESSION} ==="
"${APTOS_BIN}" move sim init --path "${SESSION}" --network mainnet

echo
echo "=== 2) balances BEFORE transfer ==="
view_balance "${SENDER}" "sender"
view_balance "${RECIPIENT}" "recipient"

echo
echo "=== 3) unauthenticated transfer ${AMOUNT_OCTA} octa ==="
echo "    sender=${SENDER}"
echo "    recipient=${RECIPIENT}"
"${APTOS_BIN}" move run \
  --session "${SESSION}" \
  --unauthenticated \
  --assume-yes \
  --sender-account "${SENDER}" \
  --function-id 0x1::aptos_account::transfer \
  --args "address:${RECIPIENT}" "u64:${AMOUNT_OCTA}"

echo
echo "=== 4) balances AFTER transfer ==="
view_balance "${SENDER}" "sender"
view_balance "${RECIPIENT}" "recipient"

echo
echo "Done. Expected:"
echo "  - step 3 vm_status is Success (not INVALID_AUTH_KEY)"
echo "  - sender balance dropped by ${AMOUNT_OCTA} + gas"
echo "  - recipient balance rose by ${AMOUNT_OCTA}"
echo "Session dir: ${SESSION}"
