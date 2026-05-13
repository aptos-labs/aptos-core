# Native-DS Branch — Implementation Summary

What's shipped on the `native_ds` branch (aptos-core + etna) for the
on-chain-liquidation pipeline. This is a snapshot of "what exists"
rather than a forward-looking design — for the consensus-integration
spec see `PLAN_native_liquidation_validator_txn.md`; for the original
storage architecture see `PLAN_native_position.md`.

## Goal

Move liquidation discovery on-chain via a validator transaction
emitted at every block boundary. The proposer scans an in-memory
mirror of position + collateral state, emits a `LiquidationPlan` as
the validator-txn payload, and the VM dispatches per-market
`liquidate_positions(accounts, market)` calls into etna's existing
Move surface. Move stays authoritative; the native mirror is a
write-only side channel populated by dual-writes from etna.

This branch ships the read side (mirror, reader API, scanner, plan
generation) and the etna integration. Consensus integration
(validator-txn variant + proposer hook + VM dispatch) is specced but
not wired.

## Architecture at a glance

```
┌── etna (Move authoritative) ─────────────────────────────────────────────┐
│                                                                         │
│  perp_positions::update_position(...)                                   │
│  collateral_balance_sheet::deposit_collateral(...)                      │
│        │                          │                                     │
│        │                          │                                     │
│        ▼ mirror_*_position        ▼ mirror_balance                      │
│        decibel_dex::native_bridge                                       │
│        │                                                                │
└────────┼────────────────────────────────────────────────────────────────┘
         │
         ▼ (write-only)
┌── aptos_experimental (framework Move) ──────────────────────────────────┐
│  native_position::{create,update,remove}_position(cap, account, market) │
│  native_collateral::{set,remove}_collateral(cap, account, kind)         │
│  no read API exposed to Move                                            │
└────────┬────────────────────────────────────────────────────────────────┘
         │ (native fns stage in NativePositionContext)
         ▼
┌── aptos-vm-types::VMChangeSet ─────────────────────────────────────────┐
│  position_write_set (StateKeyInner::Position)                          │
│  collateral_write_set (StateKeyInner::Collateral)                      │
│  position_ephemeral_writes — kept as empty backward-compat bucket      │
└────────┬───────────────────────────────────────────────────────────────┘
         │ via TransactionOutput.native_writes (#[serde(skip)] side-channel)
         ▼
┌── aptos-db (storage) ──────────────────────────────────────────────────┐
│  NativeStateCommitter::apply(version, position_writes, collat_writes)  │
│       │                                                                │
│       ▼                                                                │
│  NativeStateStore.users : DashMap<UserKey, UserState>                  │
│       UserKey  = (exchange_id, account)                                │
│       UserState = { positions: BTreeMap<market, sv>,                   │
│                     collateral: BTreeMap<BalanceKind, sv> }            │
│  position_db RocksDB CF (positions durable; collateral in-memory only) │
└────────┬───────────────────────────────────────────────────────────────┘
         │ install_global_reader(InMemoryNativeStateReader)
         ▼
┌── validator-side reader / scanner ─────────────────────────────────────┐
│  trait NativeStateReader { iter_users, get_account_*, ... }            │
│  trait LiquidationScanner { fn scan(&reader, exchange_id) -> Vec<...> }│
│  AnyPositionScanner (production default)                               │
│  build_liquidation_plan → LiquidationPlan { by_market, cross, ... }    │
│  group_by_market helper                                                │
└────────────────────────────────────────────────────────────────────────┘
         │ BCS-serialize the plan
         ▼
   [Phase F.3 — not yet wired]  ValidatorTransaction::LiquidationBatch
                                  → consensus → VM dispatch
                                  → etna::liquidate_positions(accounts, market)
```

## What's in tree

### Framework (aptos-core, Move)

`aptos-move/framework/aptos-experimental/sources/`
- `native_position.move` — write-only Move surface. Public: `register`,
  `unregister`, `deny`, `reenable`, `update_ceiling`, `exchange_id`,
  `create_position`, `update_position`, `remove_position`, plus
  `Position` constructors / accessors. Native fns delegate to the
  position-natives Rust crate. **No read API exposed to Move.**
- `native_collateral.move` — parallel write-only surface for
  collateral. Public: `set_collateral`, `remove_collateral`, plus
  `BalanceKind` (`Cross`, `Isolated{market}`) and `CollateralData::V1`
  constructors. Same `ExchangeCapability` issued by `native_position`.

### Native bridge (aptos-core, Rust)

`aptos-move/framework/position-natives/src/`
- `position.rs` — `NativePosition` enum (`PerpV1` / `SpotV1`),
  compact-binary codec.
- `collateral.rs` — `NativeCollateral::V1`, `BalanceKind`,
  `SecondaryBalance`, codec.
- `context.rs` — `NativePositionContext` session extension. Stages
  position + collateral writes into per-TX cache; on session-finalize
  drains into `VMChangeSet`. **UserMarkets staging removed** — the
  unified store derives the markets list from
  `UserState.positions.keys()`.
- `natives.rs` — native fn implementations registered under
  `aptos_experimental::native_position` and `aptos_experimental::native_collateral`.
- `timing.rs` — micro-benchmark probe natives (legacy from earlier
  perf work).

### Storage layer (aptos-core, Rust)

`storage/aptosdb/src/`
- `native_state_store.rs` — **unified per-user store**. `UserKey`,
  `UserState`, `NativeStateStore`, `NativeStateResolver`. Replaces
  the older split `PositionInMemory` + `CollateralInMemory` +
  `user_markets` index. Includes `populate_from_rows` (cold-load) and
  `decode_position_state_key_pub` (state-sync chunks).
- `native_state_committer.rs` — single committer that consumes the
  side-channel `output.native_writes()` and dispatches Position
  writes (durable + in-memory + JMT leaves) and Collateral writes
  (in-memory only) into the unified store.
- `native_state_reader.rs` — `NativeStateReader` trait +
  `InMemoryNativeStateReader` impl over the unified store. Process-
  global handle (`install_global_reader` / `global_reader`) so
  ad-hoc consumers reach the reader without threading the AptosDB.
- `liquidation_scanner.rs` — `LiquidationScanner` trait, two impls,
  `LiquidationPlan` BCS-serializable artifact, helpers.
- `position_db.rs`, `position_merkle_db.rs`,
  `position_merkle_committer.rs` — durable RocksDB + JMT (positions
  only; collateral durability is Phase E).
- `position_pruner.rs`, `position_backup.rs`, `position_state_sync.rs`
  — durable-side bookkeeping.
- `position_metrics.rs` — Prometheus metrics:
  - `aptos_position_in_memory_count` / `aptos_collateral_in_memory_count`
    (gauges)
  - `aptos_position_writes` / `aptos_collateral_writes` (counters,
    labeled `kind`)
  - `aptos_position_reads` / `aptos_position_read_misses`
  - `aptos_user_markets_reads` (legacy, still exposed for the
    resolver's `read_user_markets` path)
  - `aptos_position_cold_load_seconds`
  - `aptos_position_prune_rows`
  - `aptos_liquidation_plan_seconds` (build-plan latency histogram)

### Cross-cutting (aptos-core, Rust)

- `types/src/state_store/state_key/inner.rs` —
  `StateKeyInner::Position { exchange_id, account, market }` (tag 2)
  and `StateKeyInner::Collateral { exchange_id, account, kind }`
  (tag 4). `StateKeyInner::UserMarkets` (tag 3) is still a defined
  variant for backward compat but no longer emitted.
- `types/src/transaction/mod.rs` —
  `TransactionOutput.native_writes : NativeTransactionWrites`
  (`#[serde(skip)]` side-channel). Carries
  `position_writes` + `collateral_writes` + (always-empty)
  `user_markets_writes` from VMOutput materialization to commit.
- `aptos-move/aptos-vm-types/src/change_set.rs` —
  `VMChangeSet.position_write_set` + `collateral_write_set` +
  `position_ephemeral_writes` buckets, with set/get accessors and
  an invariant-checked setter. All three buckets are dropped during
  WriteSet materialization (they bypass main state).
- `aptos-move/aptos-vm-types/src/output.rs` —
  `VMOutput::into_transaction_output` snapshots the three buckets
  into `native_writes` before drop.
- `aptos-move/aptos-vm/src/move_vm_ext/session/mod.rs` — drains
  `NativePositionContext::into_change_maps()` (4-tuple), builds
  `StateKey::Position` / `StateKey::Collateral` write maps, installs
  into `VMChangeSet` buckets.

### Bench harness (aptos-core, Rust)

`execution/executor-benchmark/src/`
- `lib.rs::print_native_mirror_sizes()` — end-of-bench dump of
  resident position + collateral counts, plus diagnostic
  `positions_staged` / `collateral_staged` counters.
- `lib.rs::print_scanner_summary()` — runs `build_liquidation_plan`
  for every active exchange, logs per-market plan, asserts the BCS
  bytes are byte-stable across two consecutive scans.
- `transaction_committer.rs::maybe_report_per_block_scanner()` —
  optional per-block telemetry gated by `APTOS_SCANNER_PER_BLOCK=1`.
- `init_db()` calls `db.init_native_position(...)` when
  `APTOS_INIT_NATIVE_POSITION=1`.

### etna (decibel_dex, Move)

`move/perp/sources/position_management/`
- `native_bridge.move` — singleton `NativeBridgeCapStore` holding the
  per-exchange `ExchangeCapability`. Exposes `mirror_create_position`,
  `mirror_update_position`, `mirror_remove_position`,
  `mirror_set_collateral`, `mirror_remove_collateral`. Lives separate
  from `perp_positions` to break a circular import (perp_positions
  already depends on `collateral_balance_sheet`).
- `perp_positions.move` — dual-write hooks at all 4 position-mutation
  sites (`configure_user_settings_for_market` remove + create,
  `update_position` iter_modify + add).

`move/perp/sources/collateral/`
- `collateral_balance_sheet.move` — `mirror_balance(self, balance_type)`
  helper invoked at the 4 leaf mutators (`set_reserved_collateral`,
  `deposit_collateral_into_account_on_balance_sheet`, the two
  `withdraw_*_unchecked` paths, `transfer_to_backstop_liquidator`).
  Wrapper functions inherit via the inner calls. Guards on
  `native_bridge::ready()` so test setups that don't publish the
  perp engine are unaffected.

## Verification

Smoke bench, etna realistic workload, `concurrency_level=1`,
5 blocks, `APTOS_INIT_NATIVE_POSITION=1`:

```
native-mirror-sizes positions=506 collateral=1005
                    positions_staged=515 collateral_staged=1049
scanner-summary exchange_id=1 positions=506 collateral=1005
                underwater=0 any_position=23 dispatch_fanout=22
scanner-plan exchange_id=1 bcs_size=824 stable=true
scanner-plan exchange_id=1 cross_accounts=20
scanner-plan exchange_id=1 market=0x...d37f9ae isolated_accounts=2
scanner-plan exchange_id=1 market=0x...0cb86b9de isolated_accounts=1
```

- 506 positions / 1005 collateral entries resident in the in-memory
  mirror — pipeline writes flow end-to-end.
- 23 candidates: 20 cross-margin accounts (deduped per user) +
  3 isolated entries across 2 markets.
- 824-byte BCS payload — fits well within any reasonable validator-
  txn budget.
- Byte-stable across consecutive scans — required for cross-validator
  agreement.
- 9/9 unit tests pass in `liquidation_scanner.rs`.
- Bench TPS unaffected (~2100 in-block at c=1).

## Phase ledger

| Phase | What |
| --- | --- |
| A | Strip framework Move read surface (write-only mirror) |
| B | Add `aptos_experimental::native_collateral` Move + Rust natives |
| C | Etna `native_bridge` + dual-write at all position + collateral mutation sites |
| D | End-to-end commit pipeline + `NativeStateReader` trait |
| E | Wire `TransactionOutput.native_writes` side-channel; close the gap that silently dropped writes; ship `LiquidationScanner` + `UnderwaterScanner` |
| F.1 | Bench-end summary + global reader handle (Mutex swap) |
| F.2 | `AnyPositionScanner` + `group_by_market` per-market dispatch grouping |
| F.3 | `LiquidationPlan` first-class type + `build_liquidation_plan` + BCS serialization + scanner latency metric + design doc |
| Per-block telemetry | Optional `APTOS_SCANNER_PER_BLOCK=1` for proposer-cost modeling |
| Refactor | Per-user unified `NativeStateStore` (replaced split layout); user-scan semantics in `AnyPositionScanner` (dedup cross-margin per user) |
| F.4 | Collateral durability: separate `collateral_db` + `collateral_merkle_db` instances, three-way state-root composer with `APTOS::StateRoot::Native` domain, `CollateralChunk` state-sync format, cold-load from disk, stale-index emission at commit, `CollateralPruner`, global pruner handles |

Branch tips: aptos-core `9ee0a6938d`, etna `70eca44f2`.

## Pending work (out of scope for this branch)

1. **Validator-txn injection** (`PLAN_native_liquidation_validator_txn.md`)
   - New `ValidatorTransaction::LiquidationBatch { exchange_id,
     market_buckets, cross_accounts }` variant
   - Block-proposer hook calling `build_liquidation_plan`
   - Cross-validator verifier (subset-relation tolerance for laggy
     validators)
   - VM dispatch to N parallel `liquidate_positions(accounts, market)`
     Move calls, best-effort

2. **MVHashMap correctness gap** — both the position resolver and the
   collateral path bypass MVHashMap. Bench-safe at `c=1`; production
   parallel exec needs the wrapper described in
   `aptos-vm-types/src/resolver.rs:197-203`. See
   `project_native_position_blockstm_gap.md` for the failure mode.

3. **TransactionInfo composite-state-root propagation** —
   `compose_native_state_root` exists but proof verifiers (SDKs,
   bridges, state-sync chunk-boundary checks) don't yet enforce the
   three-way root. Cross-crate work, deferred.

4. **Pruner scheduling** — `PositionPruner` + `CollateralPruner` exist
   but aren't wired into the periodic prune loop in
   `LedgerPrunerManager`. Today callers invoke `prune_up_to(horizon)`
   manually. The bench drains at end-of-run for verification.

5. **Position counter for collateral** — positions get an
   `AggregatorV2`-bounded ceiling per exchange; collateral does not.
   Bounded by position count in practice; worth confirming.

6. **Smarter scanner filter** — `AnyPositionScanner` is pessimistic
   (any non-zero size flagged). Realistic margin check requires mark
   price + funding state. Either add a mark-price mirror or punt the
   filter to Move re-validation (today's posture).

7. **Etna `liquidate_cross_margin(account)` entry function** — etna's
   existing `liquidate_positions(accounts, market)` is per-market.
   Cross-margin candidates from the scanner currently need a
   "representative market" choice from the dispatcher. A dedicated
   per-account entry on the etna side would simplify the validator-txn
   payload.

## Reading order for new contributors

1. `PLAN_native_position.md` — original storage design, useful for
   context (note: the in-memory layout has since been refactored to a
   per-user nested store).
2. This document (`PLAN_native_ds_summary.md`) — current state.
3. `PLAN_native_liquidation_validator_txn.md` — forward-looking
   consensus-integration design.
4. `storage/aptosdb/src/native_state_store.rs` — the canonical data
   structure.
5. `storage/aptosdb/src/liquidation_scanner.rs` — scanner trait,
   `LiquidationPlan`, `build_liquidation_plan`.
6. `aptos-move/framework/position-natives/src/{context,natives}.rs`
   — staging path from Move.
7. `etna/move/perp/sources/position_management/native_bridge.move` +
   `perp_positions.move` + `collateral_balance_sheet.move` — etna
   dual-write integration.
