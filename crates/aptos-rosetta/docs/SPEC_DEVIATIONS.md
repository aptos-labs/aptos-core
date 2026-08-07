# aptos-rosetta — Deviations from the Rosetta / Mesh Spec

This document is a complete inventory of every place where `aptos-rosetta`
deviates from, extends, or reinterprets the [Rosetta / Mesh API
spec](https://www.rosetta-api.org/docs/Reference.html). Many of these were
undocumented but are load-bearing: integrators and the conformance harness
(`rosetta_cli.json`, `aptos.ros`) depend on them.

Citations use `file:line` against the source at the time of writing. When code
moves during the rewrite, update the citations here rather than deleting them.

> Scope note: this file documents deviations that are **intentional and
> preserved**. Quirks and bugs that the rewrite deliberately *changes* are
> tracked separately in [`BEHAVIOR_CHANGES.md`](./BEHAVIOR_CHANGES.md).

---

## 1. Models are hand-rolled (no Mesh model crate)

Every Rosetta model is defined by hand in `src/types/` — there is no dependency
on a generated Mesh/Rosetta model crate. This is why custom fields can be freely
added to any model. Modules:

- `src/types/identifiers.rs` — `AccountIdentifier`, `SubAccountIdentifier`,
  `BlockIdentifier`, `NetworkIdentifier`, `OperationIdentifier`,
  `PartialBlockIdentifier`, `TransactionIdentifier`.
- `src/types/objects/` (split into submodules `currency.rs`, `operation.rs`,
  `transaction.rs`, `internal_op.rs`) — `Allow`, `Amount`, `Currency`, `Block`,
  `Operation`, `PublicKey`, `Signature`, `Transaction`, `InternalOperation`, etc.
- `src/types/misc.rs` — `Error`, `OperationStatus`, `Version`, `OperationType`,
  `OperationStatusType`.
- `src/types/requests.rs` — all request/response envelopes.
- `src/types/move_types.rs` — Aptos on-chain Move structs (not Rosetta models).

Consequence: JSON serialization rules are defined per-struct. All optional
fields use `#[serde(skip_serializing_if = "Option::is_none")]`, and all `u64`s
in metadata are serialized as strings via `aptos_api_types::U64` (see README
"Data types"). Any rewrite MUST preserve these serde attributes byte-for-byte.

---

## 2. Block model: "block = range of transactions", fake hashes

Aptos has no indexable block hash, so the implementation fabricates one.

- **Fake block hash.** `BlockIdentifier.hash` is the string `"<chain_id>-<block_height>"`,
  **not** a real hash. `src/common.rs:294-297` (the `BlockHash` doc),
  `src/types/identifiers.rs:464-465`, formatting at `src/common.rs:376-380`,
  parsing at `src/common.rs:327-374`.
- **Block index = block height.** `BlockIdentifier.index` is the Aptos block
  height, and a "block" is the range of transactions in that height
  (`src/types/identifiers.rs:456-478`, `src/block.rs`).
- **Both index and hash accepted.** The spec's `PartialBlockIdentifier` expects
  one or the other; this impl accepts both and silently prefers `index` without
  checking they agree. `src/common.rs:258-267` ("This is required. Rosetta
  originally only took one or the other, and this failed in integration
  testing.").
- **Genesis parent = genesis.** Block 0's parent identifier is itself, per the
  Rosetta "malformed genesis block" guidance (`src/block.rs:126-131`).
- **Genesis block info is hardcoded** (height 0, timestamp `Y2K_MS`, version 0)
  (`src/block.rs:188-198`).
- **`timestamp_start_index = 2`.** Genesis (block 0) and the first block (block
  1, no timestamp) are excluded from timestamp validation
  (`src/network.rs:115-116`).
- **Empty-transaction dropping.** Non-standard `BlockRequest.metadata.keep_empty_transactions`
  flag; when false (default) transactions with zero operations are dropped from
  the block to reduce payload size (`src/types/requests.rs:78-82`,
  `src/block.rs:55-101`).

---

## 3. Timestamps clamped to the year 2000

Rosetta requires millisecond timestamps at or after 2000-01-01. Aptos genesis
timestamps are ~0, so any timestamp older than `Y2K_MS = 946713600000` is
clamped up to that value (`src/common.rs:25-26`, `src/common.rs:111-121`).

---

## 4. `AccountIdentifier` / `SubAccountIdentifier` overloading

The spec's `sub_account.address` is meant to be an address. Here it is an
overloaded sentinel that selects a *view* of an account's stake or a fungible
store (`src/types/identifiers.rs:247-454`):

| `sub_account.address` | Meaning |
|---|---|
| `stake` | total staking-contract stake (`is_total_stake`) |
| `stake-<operator_hex>` | operator-specific stake (`new_operator_stake`, `operator_address()`) |
| `pending_active_stake` | pending-active slice |
| `active_stake` | active slice |
| `pending_inactive_stake` | pending-inactive slice |
| `inactive_stake` | inactive slice |
| `commission` | commission amount |
| `rewards` | accumulated rewards |
| `secondary_store` | a non-primary fungible store (with `metadata.currency`) |

- **Custom `SubAccountIdentifierMetadata`** with non-spec `pool_address` and
  `currency` fields (`src/types/identifiers.rs:437-454`). `pool_address`
  distinguishes delegation-pool staking from staking-contract staking (same
  sentinel, presence of metadata flips the meaning — see
  `is_delegator_active_stake` vs `is_active_stake`,
  `src/types/identifiers.rs:381-411`).
- `AccountIdentifier` omits the spec's optional top-level `metadata` field.

---

## 5. `Currency` + custom `CurrencyMetadata`

`Currency.metadata` carries non-spec fields (`src/types/objects/currency.rs`,
`CurrencyMetadata`):

- `move_type` — the Move coin type, e.g. `0x1::aptos_coin::AptosCoin`.
- `fa_address` — the Fungible Asset metadata address, e.g. `0xA`.

Currency resolution rules:

- **APT native coin** is `symbol=APT, decimals=8, move_type=0x1::aptos_coin::AptosCoin,
  fa_address=None` — even though APT's FA metadata address is `0xA`. The `0xA`
  address is deliberately omitted for backward compatibility
  (`src/common.rs:149-179`); `is_native_coin` still treats `0xA` as APT
  (`src/common.rs:176-179`).
- **Coin lookup** matches on `move_type` (`find_coin_currency`, `src/common.rs:208-223`).
- **FA lookup** matches on `fa_address`, short-circuiting `0xA`→APT
  (`find_fa_currency`, `src/common.rs:224-249`).
- **Hardcoded USDC** currencies for mainnet/testnet, added automatically by
  chain (`src/common.rs:181-206`, `src/lib.rs:64-69`).
- Additional currencies are loaded from a CLI-supplied JSON file
  (`src/main.rs:194-273`).

---

## 6. Operations

### 6.1 Custom operation types (`src/types/misc.rs`, `OperationType`)

15 `OperationType` variants; only `deposit`, `withdraw`, `fee` resemble typical
Rosetta usage. The other 12 are Aptos-specific:

`create_account`, `withdraw`, `deposit`, `staking_reward`, `set_operator`,
`set_voter`, `initialize_stake_pool`, `reset_lockup`, `unlock_stake`,
`update_commission`, `withdraw_undelegated_funds`, `distribute_staking_rewards`,
`add_delegated_stake`, `unlock_delegated_stake`, `fee`.

- **Enum order is load-bearing.** `CreateAccount` first, `Withdraw` before
  `Deposit`, `Fee` last. The derived `Ord` is used to sort operations
  deterministically within a transaction (`src/types/misc.rs`, `OperationType`;
  `Operation` `Ord` at `src/types/objects/operation.rs`).

### 6.2 Custom `OperationMetadata` (`src/types/objects/operation.rs`)

A flattened bag of 10 non-spec fields, populated per op type: `sender`,
`operator`, `old_operator`, `new_operator`, `new_voter`, `staked_balance`,
`commission_percentage`, `amount`, `staker`, `pool_address`.

### 6.3 Operation statuses (`src/types/misc.rs`, `OperationStatusType`)

Only two: `success` / `failure`. For a **failed** transaction, operations parsed
from the payload are all marked `failure`.

### 6.4 Fees & storage refunds

- Every committed user transaction gets a synthetic `fee` operation with a
  negative amount `-(gas_used * gas_unit_price)` (`src/types/objects/operation.rs`
  fee handling; `Amount::suggested_gas_fee` at `src/types/objects/currency.rs`).
- Storage-fee refunds are emitted as extra `deposit` operations parsed from
  `FeeStatement` events, and attributed to the fee payer when present
  (`src/types/objects/operation.rs`; see the `test_*_storage_refund_*` tests in
  `src/test/mod.rs`).

---

## 7. `Transaction` + custom `TransactionMetadata`

`Transaction.metadata` is non-spec (`src/types/objects/transaction.rs`,
`TransactionMetadata`):

- `transaction_type` — custom `TransactionType` enum: `User`, `Genesis`,
  `BlockMetadata`, `BlockMetadataExt`, `StateCheckpoint`, `Validator`,
  `BlockEpilogue`. **`Display` renames** `BlockMetadata → "BlockResource"` and
  `BlockMetadataExt → "BlockResourceExt"` (`src/types/objects/transaction.rs`,
  `impl Display for TransactionType`).
- `version` — the ledger version (u64 as string).
- `failed` — whether the transaction failed.
- `vm_status` — the VM status string.

---

## 8. `Allow` / `network/options` hardcoded fields

`Allow` (`src/types/objects/currency.rs`) plus the values set in
`src/network.rs:109-122`:

- `historical_balance_lookup = true`.
- `timestamp_start_index = 2` (see §2).
- `call_methods = []` (no `/call` support).
- `balance_exemptions = []`.
- **`mempool_coins` — non-standard field**, always `false`.
- `Version.middleware_version` hardcoded `"0.1.0"`; `Version.node_version` is the
  crate constant `NODE_VERSION` ("0.1"), **not** fetched from the node
  (`src/network.rs:88-93`, TODO at `:90`). `ROSETTA_VERSION = "1.4.12"`
  (`src/lib.rs:38-39`).

---

## 9. Errors

- **36 custom error codes**, all returned as **HTTP 500** regardless of kind
  (`ApiError::info` and `status_code` in `src/error.rs`). Rosetta allows any codes
  but most implementations vary HTTP status; this one does not.
- **Custom `ErrorDetails { details: String }`** attached via the optional
  `details` field (`src/types/misc.rs`; `ApiError::into_error` in `src/error.rs`).
- **Custom retriable set** (`ApiError::info` in `src/error.rs`): `AccountNotFound`,
  `BlockNotFound`, `MempoolIsFull`, `GasEstimationFailed`,
  `CoinTypeFailedToBeFetched`, `RateLimited`.
- Codes 18-36 are proxied from the node's `AptosErrorCode` via
  `From<RestError>` (`src/error.rs`, `impl From<RestError> for ApiError`).
- `ApiError::all()` enumerates every error so `network/options` can list them
  (`src/error.rs`).

> Note: the rewrite consolidated `code`/`message`/`retriable` into a single
> `ApiError::info` table (BC-3), and fixed message typos (BC-1) and one
> mis-mapping (BC-2) — see `BEHAVIOR_CHANGES.md`. The wire `code`/`retriable`
> values are unchanged.

---

## 10. Construction API specifics

- **Ed25519 single-signer only.**
  - `combine` rejects `signatures.len() != 1` (`UnsupportedSignatureCount`) and
    any non-Ed25519 curve/signature (`InvalidSignatureType`)
    (`src/construction/combine.rs`).
  - Only `CurveType::Edwards25519` and `SignatureType::Ed25519` exist
    (`src/types/objects/currency.rs`, `CurveType`/`SignatureType`).
- **`derive` supports only the original Ed25519 auth-key scheme** and is
  documented to be wrong for rotated accounts
  (`src/construction/derive.rs`).
- **Metadata is mandatory in `payloads`** even though the spec marks it optional,
  because the `RawTransaction` cannot be built offline without it
  (`src/construction/payloads.rs`).
- **Operator-fill hack.** `payloads` overrides/derives operator fields to keep
  operations consistent with metadata (`src/construction/payloads.rs`;
  `fill_in_operator` in `src/construction/helpers.rs`).
- **Custom `InternalOperation`** (`src/types/objects/internal_op.rs`) is a non-spec
  type that encodes the resolved high-level action (Transfer, CreateAccount,
  staking ops, …). It is smuggled verbatim through
  preprocess → metadata → payloads inside `MetadataOptions.internal_operation`
  and `ConstructionMetadata.internal_operation`
  (`src/types/requests.rs:203-231`, `:247-261`) so `payloads` never re-parses
  operations.
- **Transfer payload dispatch** (`InternalOperation::payload`,
  `src/types/objects/internal_op.rs`) — deviates from a single transfer path:
  - APT → `aptos_account::transfer` (special-cased "so behavior doesn't change").
  - Any currency with a `move_type` → `aptos_account::transfer_coins`, **even if
    the coin has migrated to FA**.
  - FA-only currency → hand-built `primary_fungible_store::transfer` entry
    function.
- **Metadata / gas estimation extensions** (`src/types/requests.rs:199-231`;
  `construction_metadata` in `src/construction/metadata.rs`, `simulate_transaction`
  in `src/construction/helpers.rs`):
  - `gas_price_multiplier` (percent of estimate), `gas_price_priority`
    (`Low`/`Normal`/`High` → deprioritized/normal/prioritized estimate),
    `public_keys` (required when `max_gas_amount` absent), `expiry_time_secs`
    (default 30s), `sequence_number`.
  - Gas estimation simulates the transaction with a **dummy all-zero signature**
    (`src/construction/helpers.rs`, `simulate_transaction`).
- **`ConstructionParseMetadata`** returns the raw/signed BCS transaction back to
  the caller — a non-spec extra (`src/types/requests.rs:294-300`).
- **`submit`** uses `submit_bcs` and returns the committed hash.

---

## 11. Endpoints implemented vs. not

Implemented (routes in `src/lib.rs:164-189`):
`/account/balance`, `/block`, all 8 `/construction/*`,
`/network/list|options|status`, and `GET /-/healthy` (a non-spec health proxy).

**Defined as types but NOT registered as routes** (so effectively unsupported):

- `/account/coins` — not implemented.
- `/block/transaction` — not implemented.
- `/mempool`, `/mempool/transaction` — request/response types exist
  (`src/types/requests.rs:471-507`) but **no routes**; `mempool_coins=false`.
- `/call`, `/events/blocks`, `/search/transactions` — not implemented.

---

## 12. Balances

- Balances are always reported **as of the end of a block** (last version in the
  block), not by ledger version (`src/account.rs:72-78`,
  README "Balances").
- If the block/version has been pruned, the call errors out.
- Staking/delegation balances come from **Move view functions**
  (`0x1::staking_contract::staking_contract_amounts`,
  `pending_attribution_snapshot`, `0x1::delegation_pool::get_stake`,
  `0x1::stake::get_lockup_secs`) rather than from write sets
  (`src/types/misc.rs:287-491`, `src/account.rs:291-396`).
- Sequence number falls back to 0 when the account/resource does not exist
  (`src/account.rs:247-289`).
- FA balances require a two-pass write-set walk to resolve store→owner and
  store→currency, because FA stores don't carry owner/currency inline
  (`src/types/objects/operation.rs` FA parsing; primary vs secondary store derived
  via `create_derived_object_address`).

---

## 14. Identifier hex format: no `0x` prefix

`TransactionIdentifier.hash` (and other identifiers built via
`common::to_hex_lower`, which is `format!("{:x}")`) is **bare lowercase hex with
no `0x` prefix** — e.g. `a38d2cfc...` (64 chars for a transaction hash). This is
confirmed by `test::construction::apt_transfer_full_offline_round_trip`. Note the
README claims account identifiers "begin with 0x"; the actual encoding depends on
the `LowerHex` impl of the underlying type and does not add a prefix. Pinned by
the construction round-trip test; revisit consistency during the rewrite.

## 13. On-chain function name typo (documented, not fixable here)

`UPDATE_COMMISSION_FUNCTION = "update_commision"` (misspelled) mirrors the
on-chain entry function name, which cannot be corrected without a framework
change (`src/types/move_types.rs:54-56`).

---

## 15. rosetta-cli conformance assets

Conformance is a **manual** step (no CI/smoke-test wiring): run `rosetta-cli`
with `rosetta_cli.json` + `aptos.ros` against a running online (`:8082`) /
offline (`:8083`) pair. The automated in-process equivalent is
`testsuite/smoke-test/src/rosetta.rs` (drives the Rust `RosettaClient` against a
LocalSwarm). Load-bearing dependencies these assets bake in:

- **Network name `"TESTING"`** — `ChainId::from_str("TESTING")` must resolve to
  `ChainId::test()` (chain id 4). Verified by `types/src/chain_id.rs`.
- **APT currency shape** — `aptos.ros` uses APT as `{symbol: APT, decimals: 8,
  metadata.move_type: 0x1::aptos_coin::AptosCoin}` with **no `fa_address`**,
  exactly matching `common::native_coin` (see §5). Changing `native_coin` to
  carry `fa_address = 0xA` would break the DSL's currency matching.
- **Block hash separator is `-`** (`<chain_id>-<block_height>`), not `:` — see
  §2. (`README.md` previously stated `:`; corrected 2026-07.)
- Only `create_account` / `withdraw` / `deposit` operations and Ed25519
  single-signer construction are exercised (see §6.1, §10).
