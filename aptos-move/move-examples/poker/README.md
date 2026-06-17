# Poker (Nitro Enclave Multiplayer Example)

A small multiplayer poker dapp that uses **AWS Nitro Enclave** attestation so the game server (table) runs in a TEE. Players lock APT as chips, play hands, and can leave after the current hand; on exit they receive 95% of their balance (5% fee to the table).

## Architecture

- **On-chain (Move)**  
  - Table registration only with a valid Nitro attestation rooted in the chain-managed Nitro root store.
  - The attestation must bind the TEE to the table account with `user_data = b"APTOS_POKER_TABLE_V1" || bcs(table_address)`.
  - Player **enter_table**: lock APT as chips.  
  - Player **request_leave**: leave after current hand.  
  - **settle_hand**: table applies hand result (chip deltas).  
  - **settle_leaving_players**: pay out leavers (95% to player, 5% to table fee pool).

- **Table client (runs in TEE)**  
  - Creates/funds the table account, gets Nitro attestation, calls `register_table` on-chain.  
  - Runs hands one after another; if there are not enough players, waits.  
  - Submits `settle_hand` and `settle_leaving_players` after each hand.

- **Player client**  
  - **Enter**: send transaction to `enter_table` with table address and APT amount.  
  - **Leave**: send `request_leave`; balance is returned (minus 5% fee) after the current hand when the table calls `settle_leaving_players`.

## Move API (summary)

| Function | Who | Description |
|----------|-----|-------------|
| `register_table(table, attestation_doc)` | Table (TEE) | Register table; verifies Nitro attestation with `aws_nitro_utils::verify_attestation_user_data`, requiring `user_data` to be bound to the table address. |
| `enter_table(player, table_addr, amount)` | Player | Lock APT as chips and join from next hand. |
| `request_leave(player, table_addr)` | Player | Request to leave after current hand. |
| `settle_hand(table, table_addr, deduct_from, deduct_amounts, add_to, add_amounts)` | Table | Apply hand result (conservation: sum deduct = sum add). |
| `settle_leaving_players(table, table_addr, leaving_players)` | Table | Process leavers: 95% to player, 5% to table. |

Views: `table_exists`, `min_players`, `player_balance`, `player_pending_leave`.

## Table client (TEE)

The table client is intended to run inside an **AWS Nitro Enclave**:

1. **Bootstrap**  
   - Ensure the chain-managed Nitro root store has been initialized by framework governance with the AWS Nitro root certificate DER bytes.
   - Create/fund the table account (e.g. with Aptos SDK).  
   - Get an attestation document from the Nitro NSM (e.g. `nsm_fd` / GetAttestationDocument) with `user_data = b"APTOS_POKER_TABLE_V1" || bcs(table_address)`.
   - Call `register_table(table_signer, attestation_doc)`.

2. **Game loop**  
   - Wait until at least `min_players` (e.g. 2) are seated (query `player_balance` / events).  
   - Run one hand (deal, betting, winner) off-chain.  
   - Build `deduct_from` / `deduct_amounts` and `add_to` / `add_amounts` from the hand result.  
   - Submit `settle_hand(table, table_addr, deduct_from, deduct_amounts, add_to, add_amounts)`.  
   - Collect addresses that requested leave (from events or local state) and call  
     `settle_leaving_players(table, table_addr, leaving_players)`.  
   - Repeat; if not enough players, wait and retry.

See `clients/` for a minimal table client (dev: placeholder attestation) and player client.

## Player client

- **Enter table**: sign and submit `enter_table(table_addr, amount)` (must have approved APT).  
- **Leave**: sign and submit `request_leave(table_addr)`; funds are sent when the table runs `settle_leaving_players` for that hand.

## Fee

- On leave, 5% of the returned balance is kept by the table; 95% is sent back to the player.

## Build and test (Move)

From the repo root, use the Move examples test harness (uses local framework with `aws_nitro_utils`):

```bash
cargo test -p aptos-move-examples test_poker -- --nocapture
```

Or compile the package (with a framework that includes Nitro utils):

```bash
cd aptos-move/move-examples/poker
aptos move compile
```

## Test-only entry

For unit tests, attestation is bypassed with:

- `register_table_for_test(table)`  
  so tests don’t need a real Nitro attestation document.
