# Aptos Rosetta: Full Behavioral Specification

**Date:** 2026-02-13
**Based On:** Existing implementation in `crates/aptos-rosetta` and `crates/aptos-rosetta-cli`
**Rosetta Spec Version:** 1.4.12

---

## Table of Contents

1. [Overview](#1-overview)
2. [Server Modes](#2-server-modes)
3. [Core Concepts & Design Decisions](#3-core-concepts--design-decisions)
4. [Network API](#4-network-api)
5. [Account API](#5-account-api)
6. [Block API](#6-block-api)
7. [Construction API](#7-construction-api)
8. [Health Check API](#8-health-check-api)
9. [Operation Types](#9-operation-types)
10. [Currency System](#10-currency-system)
11. [Error Handling](#11-error-handling)
12. [Transaction Parsing Behavior](#12-transaction-parsing-behavior)
13. [Staking & Delegation Behavior](#13-staking--delegation-behavior)
14. [CLI Tool Specification](#14-cli-tool-specification)
15. [Configuration](#15-configuration)
16. [Appendix: Wire Format Examples](#appendix-wire-format-examples)

---

## 1. Overview

The Aptos Rosetta implementation is a sidecar proxy server that implements the [Coinbase Rosetta API specification](https://www.rosetta-api.org/docs/Reference.html) for the Aptos blockchain. It translates Rosetta-standard HTTP/JSON requests into Aptos REST API calls against a fullnode.

### 1.1 What Is Rosetta

Rosetta is a standardized API specification designed by Coinbase to provide a uniform interface for interacting with different blockchains. It defines how to:
- Query blockchain state (blocks, balances, network info)
- Construct and submit transactions
- Parse transaction operations (transfers, staking, etc.)

### 1.2 Aptos-Specific Adaptations

Aptos has several characteristics that require adaptation from the generic Rosetta model:

| Rosetta Concept | Aptos Implementation |
|-----------------|---------------------|
| Block hash | Synthetic: `{chain_id}-{block_height}` (not a real cryptographic hash) |
| Block | Maps to Aptos block (group of transactions between block metadata transactions) |
| Account | Hex-encoded `AccountAddress` with optional sub-accounts for staking |
| Coins | Native APT (via CoinStore or Fungible Asset) + configurable additional currencies |
| Signatures | Ed25519 single-signer only |
| Mempool | Not supported |
| Timestamps | Microseconds converted to milliseconds; pre-Y2K timestamps clamped to Y2K |

### 1.3 Supported Rosetta Spec Version

- **Rosetta Version:** 1.4.12
- **Node Version:** 0.1 (hardcoded, not dynamically fetched from the node)
- **Middleware Version:** 0.1.0

---

## 2. Server Modes

The server supports three modes of operation, configured via CLI subcommands:

### 2.1 Online Mode (`online`)

Runs a **local Aptos fullnode** alongside the Rosetta server. The fullnode's REST API is used exclusively as a local proxy — it is not exposed externally.

**Behavior:**
1. Starts the Aptos fullnode in a separate thread
2. Polls the fullnode REST API every 100ms until it responds successfully
3. Logs polling failures every 10 seconds
4. Once the fullnode is ready, starts the Rosetta server
5. The Rosetta server connects to the local fullnode

**Configuration:**
- Inherits all `OfflineArgs` and `OnlineRemoteArgs`
- Additionally takes `AptosNodeArgs` for the fullnode configuration
- Default REST API URL: `http://localhost:8080`

### 2.2 Online Remote Mode (`online-remote`)

Runs a Rosetta server that connects to a **remote fullnode** (e.g., a public endpoint or a separately managed fullnode).

**Behavior:**
1. Initializes logger directly (no local fullnode)
2. Creates a REST client pointing to the configured URL
3. On bootstrap, validates that the chain ID matches between the Rosetta config and the upstream fullnode
4. Starts the Rosetta server

**Caveats:**
- Subject to network latency, throttling, and errors between Rosetta and the fullnode
- The `owner_address_file` parameter exists but is deprecated (YAML file with owner addresses)

### 2.3 Offline Mode (`offline`)

Runs a Rosetta server without any blockchain connection. Only offline-capable APIs work.

**Behavior:**
- No REST client is created
- Any API that requires on-chain data returns `ApiError::NodeIsOffline`
- Useful for transaction construction and signing workflows

**Offline-capable APIs:**
- `/construction/derive`
- `/construction/hash`
- `/construction/combine`
- `/construction/parse`
- `/construction/payloads`
- `/construction/preprocess`
- `/network/list`
- `/network/options`

**Online-only APIs:**
- `/account/balance`
- `/block`
- `/construction/metadata`
- `/construction/submit`
- `/network/status`
- `/-/healthy`

### 2.4 Common Server Configuration

| Parameter | Default | Description |
|-----------|---------|-------------|
| `listen_address` | `0.0.0.0:8082` | Address/port for the Rosetta HTTP server |
| `tls_cert_path` | None | Optional TLS certificate for HTTPS |
| `tls_key_path` | None | Optional TLS key for HTTPS |
| `content_length_limit` | None | Optional request body size limit |
| `chain_id` | `test` | Chain ID (e.g., `mainnet`, `testnet`, or numeric) |
| `transactions_page_size` | `DEFAULT_MAX_PAGE_SIZE` | Page size for fetching transactions in blocks |
| `currency_config_file` | None | JSON file with additional currencies to support |

---

## 3. Core Concepts & Design Decisions

### 3.1 RosettaContext

The central state object passed to all route handlers:

```rust
struct RosettaContext {
    rest_client: Option<Arc<aptos_rest_client::Client>>,  // None in offline mode
    chain_id: ChainId,
    block_cache: Option<Arc<BlockRetriever>>,              // None in offline mode
    currencies: HashSet<Currency>,                         // Always includes APT
}
```

**Decision: Always include APT.** Regardless of the currency config file, APT is always added to the supported currencies set. On mainnet, USDC is also auto-added. On testnet, the testnet USDC variant is auto-added.

### 3.2 Block Hash Convention

**Decision: Synthetic block hashes.** Aptos blocks don't have a directly queryable hash in the same way as Bitcoin/Ethereum. The implementation uses a synthetic hash format:

```
{chain_id}-{block_height}
```

Examples:
- `mainnet-0` — Genesis block on mainnet
- `testnet-12345` — Block 12345 on testnet
- `4-100` — Block 100 on chain ID 4

The `BlockHash` is parsed strictly:
- Must contain exactly one hyphen
- Left side must be a valid chain ID
- Right side must be a valid u64
- Chain ID must match the server's chain ID when resolving

### 3.3 Network Identifier

Every request (except `/network/list`) includes a `NetworkIdentifier`:

```json
{
    "blockchain": "aptos",
    "network": "{chain_id}"
}
```

**Validation:**
- `blockchain` must be `"aptos"` (case-sensitive)
- `network` must parse to a `ChainId` that matches the server's chain ID
- Whitespace is trimmed from `network` before parsing

### 3.4 Timestamp Handling

**Decision: Clamp pre-Y2K timestamps.**

Aptos timestamps are in **microseconds**. Rosetta expects **milliseconds**.

The conversion rule:
1. Divide microseconds by 1000 to get milliseconds
2. If the result is less than `946713600000` (January 1, 2000 UTC), set it to that value

This clamp exists because:
- Genesis block (block 0) has timestamp 0
- Block 1 typically also has timestamp 0 or very early
- Rosetta implementations require timestamps to be after Y2K

### 3.5 JSON Serialization Rules

**Decision: Skip null values.**

All optional fields use `#[serde(skip_serializing_if = "Option::is_none")]` to omit null values from JSON output, as required by the Rosetta spec.

**Decision: u64 as strings in metadata.**

When u64 values appear in metadata fields, they are serialized as JSON strings using the `U64` wrapper type (e.g., `"12345"` instead of `12345`). This prevents precision loss in JavaScript clients.

### 3.6 CORS Configuration

The server allows:
- Any origin (`*`)
- Methods: GET, POST
- Headers: Content-Type

Rejection responses also include `access-control-allow-origin: *`.

---

## 4. Network API

### 4.1 POST /network/list

**Mode:** Online + Offline

**Request Body:** Empty `MetadataRequest {}` (can be omitted or `{}`)

**Behavior:**
- Returns exactly one network identifier: the server's chain ID
- Does not require network identifier validation

**Response:**
```json
{
    "network_identifiers": [
        {
            "blockchain": "aptos",
            "network": "{chain_id}"
        }
    ]
}
```

### 4.2 POST /network/options

**Mode:** Online + Offline

**Request Body:** `NetworkRequest` with `network_identifier`

**Behavior:**
1. Validates network identifier matches server
2. Returns version information, all operation types, all operation statuses, and all error codes

**Response includes:**
- `version.rosetta_version`: `"1.4.12"`
- `version.node_version`: `"0.1"` (hardcoded)
- `version.middleware_version`: `"0.1.0"`
- `allow.operation_statuses`: `["success", "failure"]`
- `allow.operation_types`: All 15 operation type strings (see Section 9)
- `allow.errors`: All error objects from `ApiError::all()` (**note: currently missing `RejectedByFilter`**)
- `allow.historical_balance_lookup`: `true`
- `allow.timestamp_start_index`: `2` (valid timestamps start at block index 2)
- `allow.call_methods`: `[]` (no call methods supported)
- `allow.balance_exemptions`: `[]`
- `allow.mempool_coins`: `false`

### 4.3 POST /network/status

**Mode:** Online only

**Request Body:** `NetworkRequest` with `network_identifier`

**Behavior:**
1. Validates network identifier
2. Fetches genesis block info (always height 0, clamped timestamp to Y2K)
3. Fetches current ledger information from the fullnode
4. Fetches oldest available block info (post-pruning)
5. Fetches latest block info

**Response:**
```json
{
    "current_block_identifier": { "index": 12345, "hash": "mainnet-12345" },
    "current_block_timestamp": 1700000000000,
    "genesis_block_identifier": { "index": 0, "hash": "mainnet-0" },
    "oldest_block_identifier": { "index": 100, "hash": "mainnet-100" },
    "sync_status": null,
    "peers": []
}
```

**Design decisions:**
- `sync_status` is always `null` (not implemented)
- `peers` is always empty (peer discovery not exposed through this API)
- `oldest_block_identifier` reflects pruning — blocks before this height return errors

---

## 5. Account API

### 5.1 POST /account/balance

**Mode:** Online only

**Request Body:**
```json
{
    "network_identifier": { "blockchain": "aptos", "network": "mainnet" },
    "account_identifier": {
        "address": "0x1",
        "sub_account": null
    },
    "block_identifier": { "index": 12345 },
    "currencies": [{ "symbol": "APT", "decimals": 8, "metadata": { "move_type": "0x1::aptos_coin::AptosCoin" }}]
}
```

**Behavior:**

1. Validates network identifier
2. Determines block height from `block_identifier`:
   - If both `index` and `hash` provided → uses `index` (hash is ignored)
   - If only `index` → uses index directly
   - If only `hash` → parses `BlockHash`, validates chain ID, extracts height
   - If neither → fetches current ledger height
3. Fetches `BlockInfo` for the target height to get the `last_version`
4. All balances are read at `last_version` (end of block)
5. Routes to one of three balance retrieval paths based on account type

#### 5.1.1 Base Account Balances (no sub_account)

For each currency in the filter (or all configured currencies if no filter):

- **Coin currencies** (have `move_type`): Calls `0x1::coin::balance<CoinType>(owner)` view function at the target version
- **FA-only currencies** (have `fa_address` but no `move_type`): Calls `0x1::primary_fungible_store::balance(owner, metadata_address)` view function at the target version

If the view function fails or the account doesn't exist, the balance is 0.

#### 5.1.2 Staking Balances (sub_account present, no pool_address metadata)

Only applies when the currency filter includes the native coin (APT).

1. Fetches `0x1::staking_contract::Store` resource for the owner
2. Iterates over all staking contracts
3. For each contract, fetches stake pool balances
4. Based on sub-account type:
   - `stake` (total): Sum of all staked amounts minus commission across all contracts
   - `active_stake`: Active stake minus commission
   - `pending_active_stake`: Pending active from stake pool
   - `inactive_stake`: Inactive from stake pool
   - `pending_inactive_stake`: Pending inactive via `pending_attribution_snapshot` view
   - `commission`: Commission amount
   - `rewards`: Accumulated rewards
   - `stake-{operator}`: Operator-specific stake (currently not fully implemented)

The `staking_contract_amounts` view function is called per contract to get:
- `total_active_stake`
- `accumulated_rewards`
- `commission_amount`

#### 5.1.3 Delegation Pool Balances (sub_account with pool_address metadata)

Only applies when the currency filter includes the native coin (APT).

1. Calls `0x1::delegation_pool::get_stake(pool_address, delegator_address)` view at the target version
2. Returns three values: `[active, inactive, pending_inactive]`
3. Based on sub-account type:
   - `active_stake` (with pool metadata): First value
   - `inactive_stake` (with pool metadata): Second value
   - `pending_inactive_stake` (with pool metadata): Third value
   - `stake` (total, with pool metadata): Sum of all three
4. Fetches lockup expiration via `0x1::stake::get_lockup_secs(pool_address)`

#### 5.1.4 Response Metadata

The response always includes:
- `sequence_number`: The account's sequence number at the target version (0 if account doesn't exist)
- `operators`: List of operator addresses (only for staking accounts)
- `lockup_expiration_time_utc`: Lockup expiration timestamp (0 if not applicable)

---

## 6. Block API

### 6.1 POST /block

**Mode:** Online only

**Request Body:**
```json
{
    "network_identifier": { "blockchain": "aptos", "network": "mainnet" },
    "block_identifier": { "index": 12345 },
    "metadata": { "keep_empty_transactions": false }
}
```

**Behavior:**

1. Validates network identifier
2. Resolves block height (same logic as account balance)
3. Fetches the block with full transactions from the fullnode
4. For non-genesis blocks, also fetches the previous block (without transactions) to get the parent block identifier
5. For genesis (block 0): parent_block_identifier equals the genesis block itself (per Rosetta spec)
6. Converts each transaction to Rosetta format
7. If `keep_empty_transactions` is false (default), transactions with zero operations are dropped
8. Transactions are sorted by version
9. Returns the complete block

**Block Structure:**
```json
{
    "block": {
        "block_identifier": { "index": 12345, "hash": "mainnet-12345" },
        "parent_block_identifier": { "index": 12344, "hash": "mainnet-12344" },
        "timestamp": 1700000000000,
        "transactions": [...]
    }
}
```

### 6.2 Transaction Conversion

Each transaction is converted from `TransactionOnChainData` to the Rosetta `Transaction` format:

**Transaction Types Recognized:**
- `User` — User transactions (the main type with operations)
- `Genesis` — Genesis transaction
- `BlockMetadata` / `BlockMetadataExt` — Block boundary transactions
- `StateCheckpoint` — State checkpoint transactions (events are dropped)
- `Validator` — Validator transactions
- `BlockEpilogue` — Block epilogue transactions (events are dropped)

**Transaction Metadata:**
```json
{
    "transaction_type": "User",
    "version": "12345",
    "failed": false,
    "vm_status": "Success"
}
```

See Section 12 for detailed transaction parsing behavior.

---

## 7. Construction API

The Construction API follows the standard Rosetta flow:

```
Preprocess → Metadata → Payloads → [External Signing] → Combine → Submit
                                          ↗
                                    Parse (verification)
                                    
Derive (offline utility)
Hash (offline utility)
```

### 7.1 POST /construction/derive (OFFLINE)

Derives an account address from a public key.

**Request:**
```json
{
    "network_identifier": {...},
    "public_key": {
        "hex_bytes": "0x...",
        "curve_type": "edwards25519"
    }
}
```

**Behavior:**
1. Validates network identifier
2. Decodes the Ed25519 public key from hex
3. Computes `AuthenticationKey::ed25519(&public_key).account_address()`
4. Returns the derived account address

**Limitations:**
- Only works for Ed25519 keys
- Only works for the initial (un-rotated) authentication scheme
- If the account has rotated its key, the derived address will not match

### 7.2 POST /construction/preprocess (OFFLINE)

Converts high-level operations into metadata options for the metadata call.

**Request:**
```json
{
    "network_identifier": {...},
    "operations": [...],
    "metadata": {
        "expiry_time_secs": "1700000060",
        "sequence_number": "5",
        "max_gas_amount": "2000",
        "gas_price": "100",
        "public_keys": [{"hex_bytes": "...", "curve_type": "edwards25519"}],
        "gas_price_multiplier": 120,
        "gas_price_priority": "normal"
    }
}
```

**Behavior:**
1. Validates network identifier
2. Extracts `InternalOperation` from the provided operations (see Section 9 for operation-to-InternalOperation mapping)
3. Validates constraints:
   - `max_gas_amount` must be >= 1 if provided
   - `expiry_time_secs` must be in the future if provided
   - Either `max_gas_amount` or non-empty `public_keys` must be provided (for gas estimation)
4. Returns the `MetadataOptions` and `required_public_keys`

**Response:**
```json
{
    "options": {
        "internal_operation": {...},
        "max_gas_amount": "2000",
        "gas_price_per_unit": "100",
        "expiry_time_secs": "1700000060",
        "sequence_number": "5",
        "public_keys": [...],
        "gas_price_multiplier": 120,
        "gas_price_priority": "normal"
    },
    "required_public_keys": [
        { "address": "0x..." }
    ]
}
```

### 7.3 POST /construction/metadata (ONLINE)

Fetches on-chain metadata and simulates the transaction.

**Request:**
The `options` field from the preprocess response is passed directly.

**Behavior:**
1. Validates network identifier and chain ID against the fullnode
2. Fetches the sender's account information
3. Determines sequence number:
   - Uses provided value if present
   - Otherwise fetches from the fullnode
4. For `SetOperator` and `SetVoter` operations: if no old operator/operator is specified and the account has exactly one staking contract, it is automatically filled in. If there are multiple, an error is returned listing all operators.
5. Simulates the transaction:
   a. Determines gas unit price:
      - Uses provided `gas_price_per_unit` if available
      - Otherwise estimates via `estimate_gas_price` API
      - Applies `gas_price_priority` (low/normal/high) to select the estimate tier
      - Applies `gas_price_multiplier` (percentage, e.g., 120 = 120%) if provided
   b. Builds the transaction with the sender and operation payload
   c. Signs with a zero-signature for simulation
   d. Calls `simulate_bcs_with_gas_estimation`
   e. Validates simulation result:
      - If `max_gas_amount` was provided and simulation used more gas → error
      - If simulation failed → error with VM status
   f. Calculates suggested fee: `gas_unit_price * max_gas_amount`
      - If estimating max gas, applies headroom via `adjust_gas_headroom`

**Response:**
```json
{
    "metadata": {
        "sequence_number": "5",
        "max_gas_amount": "2000",
        "gas_price_per_unit": "100",
        "expiry_time_secs": "1700000060",
        "internal_operation": {...}
    },
    "suggested_fee": [{
        "value": "200000",
        "currency": { "symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"} }
    }]
}
```

### 7.4 POST /construction/payloads (OFFLINE)

Constructs an unsigned transaction and signing payloads.

**Request:**
```json
{
    "network_identifier": {...},
    "operations": [...],
    "metadata": { ... },
    "public_keys": [...]
}
```

**Behavior:**
1. Validates network identifier
2. Extracts `InternalOperation` from operations
3. Validates that the extracted operation matches the metadata's `internal_operation`:
   - For `CreateAccount` and `Transfer`: exact match required
   - For `SetOperator`: matches owner and new_operator; fills in old_operator from metadata if not provided
   - For `SetVoter`: matches owner and new_voter; fills in operator from metadata if not provided
   - For all others: key fields must match exactly
4. Builds `RawTransaction` using:
   - The operation's payload
   - Metadata's sequence number, gas price, max gas amount
   - Expiry time from metadata (defaults to 30 seconds from now if not provided)
5. Generates the signing message (BCS prefix + raw transaction bytes)

**Response:**
```json
{
    "unsigned_transaction": "hex_encoded_bcs_raw_transaction",
    "payloads": [{
        "account_identifier": { "address": "0x..." },
        "hex_bytes": "hex_encoded_signing_message",
        "signature_type": "ed25519"
    }]
}
```

### 7.5 POST /construction/combine (OFFLINE)

Combines an unsigned transaction with signatures.

**Request:**
```json
{
    "network_identifier": {...},
    "unsigned_transaction": "hex_encoded_bcs",
    "signatures": [{
        "signing_payload": {...},
        "public_key": { "hex_bytes": "...", "curve_type": "edwards25519" },
        "signature_type": "ed25519",
        "hex_bytes": "hex_encoded_ed25519_signature"
    }]
}
```

**Behavior:**
1. Validates network identifier
2. Decodes the unsigned transaction from BCS
3. Validates exactly 1 signature (multi-signer not supported)
4. Validates signature type is Ed25519
5. Decodes the public key and signature
6. Constructs a `SignedTransaction`
7. Returns BCS-encoded signed transaction

### 7.6 POST /construction/parse (OFFLINE)

Parses a transaction (signed or unsigned) into operations.

**Request:**
```json
{
    "network_identifier": {...},
    "signed": true,
    "transaction": "hex_encoded_bcs"
}
```

**Behavior:**
1. Validates network identifier
2. If `signed`:
   - Decodes as `SignedTransaction`
   - Extracts signers (sender + secondary signers)
   - Extracts the `RawTransaction`
3. If not signed:
   - Decodes as `RawTransaction`
   - No signers
4. Parses the transaction payload to extract operations

**Supported Entry Functions for Parsing:**
| Module | Function | Parsed As |
|--------|----------|-----------|
| `0x1::coin` | `transfer<T>` | withdraw + deposit (with type arg currency) |
| `0x1::aptos_account` | `transfer` | withdraw + deposit (APT) |
| `0x1::aptos_account` | `transfer_coins<T>` | withdraw + deposit (with type arg currency) |
| `0x1::aptos_account` | `create_account` | create_account |
| `0x1::aptos_account` | `transfer_fungible_assets` | withdraw + deposit (FA) |
| `0x1::primary_fungible_store` | `transfer` | withdraw + deposit (FA) |
| `0x1::fungible_asset` | `transfer` | withdraw + deposit (FA, from store) |
| `0x1::staking_contract` | `switch_operator_with_same_commission` | set_operator |
| `0x1::staking_contract` | `update_voter` | set_voter |
| `0x1::staking_contract` | `create_staking_contract` | initialize_stake_pool |
| `0x1::staking_contract` | `reset_lockup` | reset_lockup |
| `0x1::staking_contract` | `update_commision` | update_commission |
| `0x1::staking_contract` | `unlock_stake` | unlock_stake |
| `0x1::staking_contract` | `distribute` | distribute_staking_rewards |
| `0x1::delegation_pool` | `add_stake` | add_delegated_stake |
| `0x1::delegation_pool` | `unlock` | unlock_delegated_stake |
| `0x1::delegation_pool` | `withdraw` | withdraw_undelegated_funds |

Any other entry function or non-entry-function payload returns `TransactionParseError`.

### 7.7 POST /construction/hash (OFFLINE)

Computes the transaction hash.

**Behavior:**
1. Decodes the signed transaction from BCS
2. Returns `committed_hash()` as the transaction identifier

### 7.8 POST /construction/submit (ONLINE)

Submits a signed transaction.

**Behavior:**
1. Validates network identifier
2. Decodes the signed transaction from BCS
3. Computes the hash before submission
4. Submits via `submit_bcs` to the fullnode
5. Returns the transaction identifier (hash)

---

## 8. Health Check API

### GET /-/healthy

**Mode:** Online only

**Query Parameters:**
- `duration_secs` (optional): Maximum number of seconds the fullnode is allowed to be behind. Default: 300 (5 minutes)

**Behavior:**
- Proxies the health check to the fullnode's `health_check` endpoint
- Returns `"aptos-node:ok"` on success

---

## 9. Operation Types

### 9.1 Complete Operation Type List

| Operation Type | String Value | Constructable | Description |
|---------------|-------------|---------------|-------------|
| CreateAccount | `create_account` | Yes | Account creation |
| Withdraw | `withdraw` | Yes (as part of transfer) | Balance decrease |
| Deposit | `deposit` | Yes (as part of transfer) | Balance increase |
| Fee | `fee` | No (auto-generated) | Gas fee deduction |
| StakingReward | `staking_reward` | No (auto-generated) | Staking reward distribution |
| SetOperator | `set_operator` | Yes | Change staking operator |
| SetVoter | `set_voter` | Yes | Change staking voter |
| InitializeStakePool | `initialize_stake_pool` | Yes | Create staking contract |
| ResetLockup | `reset_lockup` | Yes | Reset stake lockup |
| UnlockStake | `unlock_stake` | Yes | Unlock staked tokens |
| UpdateCommission | `update_commission` | Yes | Update commission rate |
| DistributeStakingRewards | `distribute_staking_rewards` | Yes | Trigger reward distribution |
| AddDelegatedStake | `add_delegated_stake` | Yes | Add stake to delegation pool |
| UnlockDelegatedStake | `unlock_delegated_stake` | Yes | Unlock delegated stake |
| WithdrawUndelegatedFunds | `withdraw_undelegated_funds` | Yes | Withdraw undelegated stake |

### 9.2 Operation Ordering

Operations within a transaction are sorted by type in this specific order:
1. `CreateAccount` (always first)
2. `Withdraw` (before Deposit)
3. `Deposit`
4. `StakingReward`
5. `SetOperator`
6. `SetVoter`
7. `InitializeStakePool`
8. `ResetLockup`
9. `UnlockStake`
10. `UpdateCommission`
11. `WithdrawUndelegatedFunds`
12. `DistributeStakingRewards`
13. `AddDelegatedStake`
14. `UnlockDelegatedStake`
15. `Fee` (always last)

Within the same type, operations maintain their original order (by operation index).

### 9.3 Operation Statuses

- `success` — Operation was part of a committed, successful transaction
- `failure` — Operation was part of a committed but failed transaction

**Design Decision:** The `Fee` operation is **always marked as `success`**, even in failed transactions, because gas is always charged.

### 9.4 InternalOperation Mapping

Operations are grouped into `InternalOperation` variants for construction:

| InternalOperation | Required Operations |
|-------------------|-------------------|
| `CreateAccount` | 1x `create_account` |
| `Transfer` | 1x `withdraw` + 1x `deposit` (same currency, matching amounts) |
| `SetOperator` | 1x `set_operator` |
| `SetVoter` | 1x `set_voter` |
| `InitializeStakePool` | 1x `initialize_stake_pool` |
| `ResetLockup` | 1x `reset_lockup` |
| `UnlockStake` | 1x `unlock_stake` |
| `UpdateCommission` | 1x `update_commission` |
| `DistributeStakingRewards` | 1x `distribute_staking_rewards` |
| `AddDelegatedStake` | 1x `add_delegated_stake` |
| `UnlockDelegatedStake` | 1x `unlock_delegated_stake` |
| `WithdrawUndelegated` | 1x `withdraw_undelegated_funds` |

### 9.5 Transfer Operation Details

Transfers are expressed as a withdraw + deposit pair:

```json
[
    {
        "operation_identifier": {"index": 0},
        "type": "withdraw",
        "account": {"address": "0xsender"},
        "amount": {"value": "-1000", "currency": {...}}
    },
    {
        "operation_identifier": {"index": 1},
        "type": "deposit",
        "account": {"address": "0xreceiver"},
        "amount": {"value": "1000", "currency": {...}}
    }
]
```

**Validation rules for transfer extraction:**
- Withdraw amount must be negative
- Deposit amount must be positive
- Absolute values must match
- Currencies must match
- Sender and receiver must be different accounts

**Transfer payload selection:**
The implementation selects the Move entry function based on the currency:
- **APT (native coin with `move_type`):** Uses `0x1::aptos_account::transfer(receiver, amount)` — this creates the receiver account if it doesn't exist
- **Coin currencies (with `move_type`):** Uses `0x1::aptos_account::transfer_coins<CoinType>(receiver, amount)`
- **FA-only currencies (with `fa_address` only):** Uses `0x1::aptos_account::transfer_fungible_assets<ObjectType>(metadata, receiver, amount)` where `metadata` is the FA address

---

## 10. Currency System

### 10.1 Currency Structure

```json
{
    "symbol": "APT",
    "decimals": 8,
    "metadata": {
        "move_type": "0x1::aptos_coin::AptosCoin",
        "fa_address": null
    }
}
```

Both `move_type` and `fa_address` are optional. Their presence determines how balances are queried:
- `move_type` present → query via `0x1::coin::balance<T>`
- `fa_address` present (no `move_type`) → query via `0x1::primary_fungible_store::balance`
- Both present → currently queries via coin (move_type takes precedence)
- Neither present → cannot be queried

### 10.2 Built-in Currencies

**APT (always present):**
```json
{
    "symbol": "APT",
    "decimals": 8,
    "metadata": {
        "move_type": "0x1::aptos_coin::AptosCoin",
        "fa_address": null
    }
}
```

**Design decision on APT FA address:** APT is also available as a Fungible Asset at address `0xA`, but the `fa_address` is intentionally **not** set in the native coin definition for backwards compatibility. The `is_native_coin` function checks for `0xA` separately.

**USDC (auto-added on mainnet):**
```json
{
    "symbol": "USDC",
    "decimals": 6,
    "metadata": {
        "move_type": null,
        "fa_address": "0xbae207659db88bea0cbead6da0ed00aac12edcdda169e591cd41c94180b46f3b"
    }
}
```

**USDC (auto-added on testnet):**
Same structure but with `fa_address`: `"0x69091fbab5f7d635ee7ac5098cf0c1efbe31d68fec0f2cd565e8d168daf52832"`

### 10.3 Custom Currency Configuration

Additional currencies can be specified via `--currency-config-file`:

```json
[
    {
        "symbol": "TC",
        "decimals": 4,
        "metadata": {
            "fa_address": "0xb528ad40e472f8fcf0f21aa78aecd09fe68f6208036a5845e6d16b7d561c83b8",
            "move_type": "0xf5a9b6ccc95f8ad3c671ddf1e227416e71f7bcd3c971efe83c0ae8e5e028350f::test_faucet::TestFaucetCoin"
        }
    }
]
```

**Validation on startup:**
- Empty symbols are skipped with a warning
- If `move_type` is present, it must parse as a valid `StructTag`
- Invalid currencies are skipped with a warning

### 10.4 Gas Fees

Gas fees are always in APT regardless of the operation's currency. The fee operation always uses:
```json
{
    "value": "-{gas_used * gas_unit_price}",
    "currency": { "symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"} }
}
```

---

## 11. Error Handling

### 11.1 Error Format

All errors return HTTP 500 with a JSON body:
```json
{
    "code": 17,
    "message": "Internal error",
    "retriable": false,
    "details": { "details": "specific error message" }
}
```

**Design decision: All errors are 500.** The Rosetta spec requires all errors to be 500 status codes. The `retriable` field indicates whether the client should retry.

### 11.2 Error Code Table

| Code | Name | Retriable | Message |
|------|------|-----------|---------|
| 1 | TransactionIsPending | No | Transaction is pending |
| 2 | NetworkIdentifierMismatch | No | Network identifier doesn't match |
| 3 | ChainIdMismatch | No | Chain Id doesn't match |
| 4 | DeserializationFailed | No | Deserialization failed |
| 5 | InvalidTransferOperations | No | Invalid operations for a transfer |
| 6 | InvalidSignatureType | No | Invalid signature type |
| 7 | InvalidMaxGasFees | No | Invalid max gas fee |
| 8 | MaxGasFeeTooLow | No | Max fee is lower than the estimated cost of the transaction |
| 9 | InvalidGasMultiplier | No | Invalid gas multiplier |
| 10 | InvalidOperations | No | Invalid operations |
| 11 | MissingPayloadMetadata | No | Payload metadata is missing |
| 12 | UnsupportedCurrency | No | Currency is unsupported |
| 13 | UnsupportedSignatureCount | No | Number of signatures is not supported |
| 14 | NodeIsOffline | No | This API is unavailable for the node because he's offline |
| 15 | TransactionParseError | No | Transaction failed to parse |
| 16 | GasEstimationFailed | Yes | Gas estimation failed |
| 17 | InternalError | No | Internal error |
| 18 | AccountNotFound | Yes | Account not found |
| 19 | ResourceNotFound | No | Resource not found |
| 20 | ModuleNotFound | No | Module not found |
| 21 | StructFieldNotFound | No | Struct field not found |
| 22 | VersionNotFound | No | Version not found |
| 23 | TransactionNotFound | No | Transaction not found |
| 24 | TableItemNotFound | No | Table item not found |
| 25 | BlockNotFound | Yes | Block is missing events |
| 26 | VersionPruned | No | Version pruned |
| 27 | BlockPruned | No | Block pruned |
| 28 | InvalidInput | No | Invalid input |
| 29 | InvalidTransactionUpdate | No | Invalid transaction update |
| 30 | SequenceNumberTooOld | No | Sequence number too old |
| 31 | VmError | No | Transaction submission failed due to VM error |
| 32 | MempoolIsFull | Yes | Mempool is full all accounts |
| 33 | CoinTypeFailedToBeFetched | Yes | Failed to retrieve the coin type information |
| 34 | StateValueNotFound | No | StateValue not found |
| 35 | RejectedByFilter | No | Transaction was rejected by the transaction filter |

### 11.3 Retriable Errors

Only these errors are marked as retriable:
- `AccountNotFound` (18) — Account may be created soon
- `BlockNotFound` (25) — Block may not be produced yet
- `MempoolIsFull` (32) — Temporary congestion
- `GasEstimationFailed` (16) — Temporary estimation failure
- `CoinTypeFailedToBeFetched` (33) — Temporary data fetch failure

### 11.4 Error Source Mapping

Errors from the Aptos REST API are automatically mapped:
- `AptosErrorCode::AccountNotFound` → `ApiError::AccountNotFound`
- `AptosErrorCode::BlockPruned` → `ApiError::BlockPruned`
- `AptosErrorCode::VmError` → `ApiError::VmError`
- `AptosErrorCode::MempoolIsFull` → `ApiError::MempoolIsFull`
- etc.

BCS deserialization errors → `DeserializationFailed`
Hex parsing errors → `DeserializationFailed`
Account address parsing errors → `DeserializationFailed`
Integer parsing errors → `DeserializationFailed`
Generic anyhow errors → `InternalError`

---

## 12. Transaction Parsing Behavior

### 12.1 Successful Transaction Parsing

For successfully committed transactions, operations are parsed from the **write set** (state changes), not the transaction payload. This is more accurate because:
- It captures all state changes, including those from scripts
- It captures side effects (e.g., auto-account creation during transfer)

**Parsing Pipeline:**

1. **Preprocess Phase:** Iterate all write ops to:
   - Map object addresses to their owners (from `ObjectCore` resources)
   - Map fungible store addresses to their currencies (from `FungibleStore` resources)
   - Collect framework resource changes (only `0x1::*` resources are processed)

2. **Parse Phase:** For each framework resource change, match on the resource type:

   | Resource | Parsing Behavior |
   |----------|-----------------|
   | `0x1::account::Account` | If sequence_number == 0 → `create_account` operation |
   | `0x1::coin::CoinStore<T>` | Parse coin deposit/withdraw events for matching currencies |
   | `0x1::fungible_asset::FungibleStore` | Parse FA deposit/withdraw events using owner/currency maps |
   | `0x1::stake::StakePool` | Currently a no-op (balance tracking commented out) |
   | `0x1::staking_contract::Store` | Parse set_operator, set_voter, distribute events |
   | `0x1::staking_contract::StakingGroupUpdateCommissionEvent` | Parse commission update events |
   | `0x1::delegation_pool::DelegationPool` | Parse delegation pool events |

3. **Storage Fee Refund:** If there's a `FeeStatement` event with non-zero `storage_fee_refund`, a `deposit` operation is added

4. **Reorder:** Operations are sorted by type (see Section 9.2) and re-indexed

5. **Gas Fee:** A `fee` operation is appended last for user transactions

### 12.2 Failed Transaction Parsing

For failed transactions, operations are parsed from the **transaction payload** (since there are no state changes). All operations are marked with `failure` status.

**Supported payloads for failed transaction parsing:**
- `0x1::coin::transfer<T>` → withdraw + deposit with failure status
- `0x1::aptos_account::transfer` → withdraw + deposit (APT) with failure status
- `0x1::aptos_account::transfer_coins<T>` → withdraw + deposit with failure status
- `0x1::aptos_account::transfer_fungible_assets` → withdraw + deposit (FA) with failure status
- `0x1::primary_fungible_store::transfer` → withdraw + deposit (FA) with failure status
- `0x1::account::create_account` → create_account with failure status
- All staking contract operations → respective operation with failure status
- All delegation pool operations → respective operation with failure status

**Note:** Even in failed transactions, the gas fee operation is still `success` because gas is always consumed.

### 12.3 Event Handling

Events are the primary data source for operation details. The implementation supports both V1 and V2 events:

**V1 Events:** Matched by `EventKey` (creator address + creation number)
**V2 Events:** Matched by `TypeTag` (e.g., `0x1::fungible_asset::Deposit`)

Supported event types:
- `0x1::fungible_asset::Withdraw` → Withdraw operations for FA
- `0x1::fungible_asset::Deposit` → Deposit operations for FA
- `0x1::coin::CoinWithdraw` → Withdraw operations for coins
- `0x1::coin::CoinDeposit` → Deposit operations for coins
- `0x1::stake::SetOperator` → Set operator operations
- `0x1::staking_contract::UpdateVoter` → Set voter operations
- `0x1::staking_contract::Distribute` → Staking reward operations
- `0x1::staking_contract::UpdateCommission` → Commission update operations
- `FeeStatement` events → Storage fee refund deposits

### 12.4 Fungible Asset Operation Parsing

For FA operations, the parsing is more complex:

1. The `FungibleStore` write identifies the store address and FA metadata address
2. The `ObjectCore` write identifies the owner of the store
3. Events (`Withdraw`/`Deposit`) provide the store address and amount
4. The store address is mapped to an owner (via `object_to_owner` map) and currency (via `store_to_currency` map)

**Special case for native coin:** If the FA metadata address is `0xA` (the APT FA address), the currency is mapped to the native APT coin, maintaining backwards compatibility.

**Unknown currencies:** If a FA's metadata address doesn't match any configured currency, the operation is silently skipped.

---

## 13. Staking & Delegation Behavior

### 13.1 Sub-Account System

Staking balances are accessed via sub-accounts on the `AccountIdentifier`:

| Sub-Account Address | Metadata | Meaning |
|-------------------|----------|---------|
| `stake` | None | Total staking contract stake |
| `active_stake` | None | Active stake in staking contract |
| `pending_active_stake` | None | Pending active stake |
| `inactive_stake` | None | Inactive stake |
| `pending_inactive_stake` | None | Pending inactive stake |
| `commission` | None | Commission amount |
| `rewards` | None | Accumulated rewards |
| `stake-{operator_hex}` | None | Operator-specific stake (not fully implemented) |
| `stake` | `{pool_address: "0x..."}` | Total delegation pool stake |
| `active_stake` | `{pool_address: "0x..."}` | Active delegation pool stake |
| `inactive_stake` | `{pool_address: "0x..."}` | Inactive delegation pool stake |
| `pending_inactive_stake` | `{pool_address: "0x..."}` | Pending inactive delegation pool stake |

### 13.2 Staking Contract Operations

All staking operations go through `0x1::staking_contract`:

| Operation | Move Function | Arguments |
|-----------|--------------|-----------|
| SetOperator | `switch_operator_with_same_commission` | `(old_operator, new_operator)` |
| SetVoter | `update_voter` | `(operator, new_voter)` |
| InitializeStakePool | `create_staking_contract` | `(operator, voter, amount, commission_percentage)` |
| ResetLockup | `reset_lockup` | `(operator)` |
| UnlockStake | `unlock_stake` | `(operator, amount)` |
| UpdateCommission | `update_commision` | `(operator, new_commission_percentage)` |
| DistributeStakingRewards | `distribute` | `(staker, operator)` |

### 13.3 Delegation Pool Operations

| Operation | Move Function | Arguments |
|-----------|--------------|-----------|
| AddDelegatedStake | `add_stake` | `(pool_address, amount)` |
| UnlockDelegatedStake | `unlock` | `(pool_address, amount)` |
| WithdrawUndelegated | `withdraw` | `(pool_address, amount)` |

### 13.4 Operator Auto-Fill

For `SetOperator` and `SetVoter`, if no operator is provided, the metadata call will:
1. Fetch the `0x1::staking_contract::Store` for the owner
2. If there is exactly one staking contract, use that operator
3. If there are 0 or more than 1, return an error listing all operators

---

## 14. CLI Tool Specification

### 14.1 Overview

`aptos-rosetta-cli` is a testing and development tool. It uses the same `RosettaClient` as integration tests.

### 14.2 Available Commands

```
aptos-rosetta-cli account balance    # Query account balance
aptos-rosetta-cli block get          # Get a block
aptos-rosetta-cli construction create-account  # Create an account
aptos-rosetta-cli construction transfer        # Transfer coins
aptos-rosetta-cli construction set-operator    # Set staking operator
aptos-rosetta-cli construction set-voter       # Set staking voter
aptos-rosetta-cli construction create-stake-pool  # Create stake pool
aptos-rosetta-cli network list       # List networks
aptos-rosetta-cli network options    # Get network options
aptos-rosetta-cli network status     # Get network status
```

### 14.3 Common Arguments

| Argument | Default | Description |
|----------|---------|-------------|
| `--rosetta-api-url` | `http://localhost:8082` | Rosetta server URL |
| `--chain-id` | `test` | Chain ID for network identifier |
| `--expiry-offset-secs` | `60` | Seconds until transaction expires |
| `--sequence-number` | None | Override sequence number |
| `--max-gas` | None | Override max gas amount |
| `--gas-price` | None | Override gas unit price |

### 14.4 Client E2E Flow

The `RosettaClient` implements the full construction flow:
1. **Derive** account address from private key
2. **Preprocess** operations with metadata
3. **Metadata** fetch (including gas estimation)
4. **Payloads** generate unsigned transaction
5. **Parse** unsigned transaction (verification)
6. **Sign** the raw transaction locally
7. **Combine** signature with unsigned transaction
8. **Parse** signed transaction (verification)
9. **Submit** signed transaction

At each step, assertions validate correctness:
- Suggested fee currency is always native coin
- `max_gas_amount * gas_price >= suggested_fee`
- Parsed operations match input operations (unless `parse_not_same` flag is set)
- Signers match between signed parse and expected signers

---

## 15. Configuration

### 15.1 Rosetta CLI Configuration (`rosetta_cli.json`)

Used for running the official Rosetta CLI automated checks:

```json
{
    "network": { "blockchain": "aptos", "network": "TESTING" },
    "online_url": "http://localhost:8082",
    "http_timeout": 30,
    "max_retries": 5,
    "construction": {
        "offline_url": "http://localhost:8083",
        "constructor_dsl_file": "aptos.ros",
        "end_conditions": {
            "create_account": 10,
            "transfer": 20
        }
    },
    "data": {
        "historical_balance_disabled": false,
        "end_conditions": {
            "tip": true,
            "reconciliation_coverage": {
                "coverage": 0.95,
                "tip": true
            }
        }
    }
}
```

### 15.2 Construction DSL (`aptos.ros`)

Defines three workflows for the Rosetta CLI:
1. **create_account** (1 concurrent): Creates accounts using funds from a "faucet" account
2. **request_funds** (1 concurrent): Waits for an account to have sufficient funds
3. **transfer** (50 concurrent): Randomly transfers APT between accounts

All workflows use:
- APT currency: `{"symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"}}`
- Confirmation depth: 2 blocks
- Minimum balance for operations: 10,000,000 octas (0.1 APT)

---

## Appendix: Wire Format Examples

### A.1 Create Account Request

```json
POST /construction/preprocess
{
    "network_identifier": {"blockchain": "aptos", "network": "mainnet"},
    "operations": [{
        "operation_identifier": {"index": 0},
        "type": "create_account",
        "account": {"address": "0xnew_account"},
        "metadata": {"sender": {"address": "0xsender"}}
    }],
    "metadata": {
        "expiry_time_secs": "1700000060",
        "public_keys": [{"hex_bytes": "0xpubkey", "curve_type": "edwards25519"}]
    }
}
```

### A.2 Transfer Request

```json
POST /construction/preprocess
{
    "network_identifier": {"blockchain": "aptos", "network": "mainnet"},
    "operations": [
        {
            "operation_identifier": {"index": 0},
            "type": "withdraw",
            "account": {"address": "0xsender"},
            "amount": {"value": "-100000000", "currency": {"symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"}}}
        },
        {
            "operation_identifier": {"index": 1},
            "type": "deposit",
            "account": {"address": "0xreceiver"},
            "amount": {"value": "100000000", "currency": {"symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"}}}
        }
    ],
    "metadata": {
        "expiry_time_secs": "1700000060",
        "public_keys": [{"hex_bytes": "0xpubkey", "curve_type": "edwards25519"}]
    }
}
```

### A.3 Set Operator Request

```json
POST /construction/preprocess
{
    "network_identifier": {"blockchain": "aptos", "network": "mainnet"},
    "operations": [{
        "operation_identifier": {"index": 0},
        "type": "set_operator",
        "account": {"address": "0xowner"},
        "metadata": {
            "old_operator": {"address": "0xold_operator"},
            "new_operator": {"address": "0xnew_operator"}
        }
    }],
    "metadata": {
        "expiry_time_secs": "1700000060",
        "public_keys": [{"hex_bytes": "0xpubkey", "curve_type": "edwards25519"}]
    }
}
```

### A.4 Delegation Pool Add Stake Request

```json
POST /construction/preprocess
{
    "network_identifier": {"blockchain": "aptos", "network": "mainnet"},
    "operations": [{
        "operation_identifier": {"index": 0},
        "type": "add_delegated_stake",
        "account": {"address": "0xdelegator"},
        "metadata": {
            "pool_address": {"address": "0xpool"},
            "amount": "100000000"
        }
    }],
    "metadata": {
        "expiry_time_secs": "1700000060",
        "public_keys": [{"hex_bytes": "0xpubkey", "curve_type": "edwards25519"}]
    }
}
```

### A.5 Account Balance with Staking Sub-Account

```json
POST /account/balance
{
    "network_identifier": {"blockchain": "aptos", "network": "mainnet"},
    "account_identifier": {
        "address": "0xowner",
        "sub_account": {"address": "stake"}
    }
}
```

### A.6 Account Balance with Delegation Pool Sub-Account

```json
POST /account/balance
{
    "network_identifier": {"blockchain": "aptos", "network": "mainnet"},
    "account_identifier": {
        "address": "0xdelegator",
        "sub_account": {
            "address": "active_stake",
            "metadata": {"pool_address": "0xpool_address"}
        }
    }
}
```

### A.7 Block Response Example

```json
{
    "block": {
        "block_identifier": {"index": 100, "hash": "mainnet-100"},
        "parent_block_identifier": {"index": 99, "hash": "mainnet-99"},
        "timestamp": 1700000000000,
        "transactions": [{
            "transaction_identifier": {"hash": "0xabcdef..."},
            "operations": [
                {
                    "operation_identifier": {"index": 0},
                    "type": "withdraw",
                    "status": "success",
                    "account": {"address": "0xsender"},
                    "amount": {"value": "-100000000", "currency": {"symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"}}}
                },
                {
                    "operation_identifier": {"index": 1},
                    "type": "deposit",
                    "status": "success",
                    "account": {"address": "0xreceiver"},
                    "amount": {"value": "100000000", "currency": {"symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"}}}
                },
                {
                    "operation_identifier": {"index": 2},
                    "type": "fee",
                    "status": "success",
                    "account": {"address": "0xsender"},
                    "amount": {"value": "-200000", "currency": {"symbol": "APT", "decimals": 8, "metadata": {"move_type": "0x1::aptos_coin::AptosCoin"}}}
                }
            ],
            "metadata": {
                "transaction_type": "User",
                "version": "12345",
                "failed": false,
                "vm_status": "Success"
            }
        }]
    }
}
```

### A.8 Error Response Example

```json
{
    "code": 25,
    "message": "Block is missing events",
    "retriable": true,
    "details": {
        "details": "Block not found at height 999999999"
    }
}
```
