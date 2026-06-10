#!/usr/bin/env bash
# Downloads a fixed set of transactions (by version) into
# aptos-move/aptos-mono-move/data/ using aptos-replay-benchmark. For each
# version it `download`s the single transaction (non-block-aligned, via
# --allow-partial-blocks) and `initialize`s its complete read-set (captured at
# the get_state_slot hook, so all touched modules are included), writing
# <version>_txns and <version>_inputs. The replay harness then reads those.
#
# Requires X_API_KEY to be exported (free key from https://developers.aptoslabs.com).
# Usage: scripts/download.sh [endpoint]
set -euo pipefail

ENDPOINT="${1:-https://api.mainnet.aptoslabs.com/v1}"

if [[ -z "${X_API_KEY:-}" ]]; then
  echo "error: export X_API_KEY first (https://developers.aptoslabs.com)" >&2
  exit 1
fi

ROOT="$(git rev-parse --show-toplevel)"
DATA_DIR="$ROOT/aptos-move/aptos-mono-move/data"
mkdir -p "$DATA_DIR"

# Each entry is "<version>"; the trailing comment records module::function.
VERSIONS=(
  5663816686  # admin_apis::update_mark_for_composite_chainlink
  5663816678  # admin_apis::update_mark_with_blended_chainlink_feeds
  5663996576  # admin_apis::update_secondary_asset_oracle
  5663816685  # dex_accounts_entry::cancel_tp_sl_order_for_position
  5663902387  # dex_accounts_entry::cancel_client_order_to_subaccount
  5663935038  # dex_accounts_entry::cancel_bulk_order_to_subaccount
  5663816684  # dex_accounts_entry::place_order_to_subaccount
  5663816673  # dex_accounts_entry::place_bulk_orders_to_subaccount
  5664025267  # dex_accounts_entry::place_tp_sl_order_for_position
  5663983784  # public_apis::process_perp_collateral_withdrawals
  5663916074  # public_apis::process_perp_market_pending_requests
  5663925741  # vault_api::process_pending_requests
  5663947285  # router_v3::exact_output_swap_entry
  5664092385  # router_v3::swap_batch
  5663958512  # primary_fungible_store::transfer
  5664049853  # router::swap_exact_in_router_entry
  5664062408  # pools::batch_update_pool_data
  5664072623  # aptos_account::transfer
)

echo "Building aptos-replay-benchmark (release)..."
cargo build -p aptos-replay-benchmark --release
BIN="$ROOT/target/release/aptos-replay-benchmark"

failed=()
for V in "${VERSIONS[@]}"; do
  echo "==> version $V"
  TXNS="$DATA_DIR/${V}_txns"
  INPUTS="$DATA_DIR/${V}_inputs"
  if "$BIN" download \
      --rest-endpoint "$ENDPOINT" \
      --api-key "$X_API_KEY" \
      --transactions-file "$TXNS" \
      --begin-version "$V" \
      --end-version "$((V + 1))" \
      --allow-partial-blocks \
    && "$BIN" initialize \
      --rest-endpoint "$ENDPOINT" \
      --api-key "$X_API_KEY" \
      --transactions-file "$TXNS" \
      --inputs-file "$INPUTS"; then
    :
  else
    echo "warn: version $V failed (continuing)" >&2
    failed+=("$V")
  fi
done

echo "Done. $(ls "$DATA_DIR"/*_inputs 2>/dev/null | wc -l | tr -d ' ') transaction(s) in $DATA_DIR"
if (( ${#failed[@]} > 0 )); then
  echo "Failed versions: ${failed[*]}" >&2
fi
