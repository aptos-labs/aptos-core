# Poker clients

- **table-client** – Runs in TEE: register table (with Nitro attestation), then run the game loop (wait for players, run hand, `settle_hand`, `settle_leaving_players`).
- **player-client** – Enter table (lock APT as chips) and request leave.

## Setup

```bash
cd aptos-move/move-examples/poker/clients
npm install
```

Set env or edit:

- `NODE_URL` – Aptos node URL (e.g. https://fullnode.devnet.aptoslabs.com).
- `TABLE_PRIVATE_KEY` – Hex private key for the table account (TEE).
- `PLAYER_PRIVATE_KEY` – Hex private key for the player (player-client).

Deploy the poker module and set `POKER_MODULE_ADDRESS` (e.g. the address where `poker` is published).

## Table client (TEE)

For **production**, run this binary inside an AWS Nitro Enclave and obtain a real attestation document (e.g. from the NSM) before calling `register_table`.

For **dev/test**, the script uses a placeholder attestation; on-chain `register_table` will abort unless the chain accepts it (e.g. testnet with a modified or mock native). Use `register_table_for_test` only in Move unit tests, not from a client.

1. Create and fund the table account (faucet or transfer).
2. Run the table client; it will:
   - Register the table (with attestation doc; in dev you may need to publish a module that skips verification for testing).
   - Enter a loop: wait for at least 2 players, run a trivial “hand” (e.g. no-op or fixed winner), call `settle_hand` and `settle_leaving_players`.

```bash
# Example (after deploy and env set)
node table-client.js
```

## Player client

- Enter: `node player-client.js enter <table_address> <amount_octas>`
- Request leave: `node player-client.js leave <table_address>`
