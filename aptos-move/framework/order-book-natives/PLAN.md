# Native PriceTimeIndex Implementation Plan

## Context

The PriceTimeIndex is a secondary index over active orders in the OrderBook. It's currently backed by BigOrderedMap (Move B+Tree), requiring ~6-8 storage reads per matching operation. This plan replaces it with a native Rust implementation using a **base+delta overlay** pattern:

- **Base:** Shared immutable `Arc<BTreeMap>` in a global cache (~4MB for 100K orders)
- **Delta:** Small per-transaction overlay (~5-10 entries, ~200 bytes serialized)
- **Persistence:** None for the index. Orders stay in BigOrderedMap. Index is rebuilt on cold start.
- **Block-STM:** Delta flows through MVHashMap via a tiny `OrderBookDelta` resource.

## Architecture

```
Global Cache (cross-block, in AptosVM process):
  DashMap<MarketAddr, Arc<PriceTimeBase>>

Per-Transaction:
  OverlayIndex {
    base: Arc<PriceTimeBase>,               // shared, read-only
    delta_buys: BTreeMap<BuyKey, Option<OrderData>>,   // Some=upsert, None=tombstone
    delta_sells: BTreeMap<SellKey, Option<OrderData>>,
  }

On-Chain:
  OrderBookDelta { block_height: u64, data: vector<u8> }   // ~200 bytes, for MVHashMap
  NativeOrderBookRegistry { market_addresses: vector<address> }
  NativeIndexCheckpoint { buys_data, sells_data, delta_log }  // for cold start
```

## Task Breakdown (Realistic Estimate: ~16-20 weeks human, ~83h Claude)

Assumes risks R1 (BigOrderedMap iteration), R3 (matching overlay bugs), R9 (forge flakiness), R12 (production readiness) are hit.

### Phase 1: Core Rust Implementation (Weeks 1-5, 24 days)

| Task | Human | Claude | Deps | Risk |
|---|---|---|---|---|
| 1.1 Design overlay data structures (OverlayIndex, PriceTimeBase, OrderBookDelta) | 2d | 2h | — | |
| 1.2 Overlay read logic (best_bid, best_ask with tombstone skipping) | 3d | 3h | 1.1 | |
| 1.3 Overlay write logic (place, cancel, update_size, match) | 3d | 3h | 1.1 | |
| 1.4 `get_single_match_result` with overlay (most complex op) | 2d | 2h | 1.2, 1.3 | R3 |
| 1.5 OrderBookCache (DashMap, Arc, pre-warm stub) | 2d | 2h | 1.1 | |
| 1.6 native_acquire / native_release (delta ser/deser) | 2d | 2h | 1.5 | |
| 1.7 Block-STM safety: base never modified within a block | 2d | 1h | 1.5, 1.6 | R2 |
| 1.8 Block boundary lazy merge (stale delta → apply to base) | 1d | 1h | 1.6, 1.7 | |
| 1.9 State sync detection (block_height gap → invalidate) | 1d | 1h | 1.6 | |
| 1.10 Rust unit tests (~15 scenarios) | 3d | 3h | 1.2-1.4 | |
| 1.11 Debug overlay correctness in matching flow | 3d | 3h | 1.4, 1.10 | R3 |

### Phase 2: Move Integration (Weeks 4-7, 21 days)

| Task | Human | Claude | Deps | Risk |
|---|---|---|---|---|
| 2.1 PriceTimeIndex: `Native { market_addr, handle }` variant | 2d | 2h | P1 | |
| 2.2 ensure_loaded / get_or_temp_handle / flush for Native | 2d | 2h | 2.1 | R4 |
| 2.3 All PriceTimeIndex functions: add Native dispatch | 2d | 2h | 2.2 | |
| 2.4 OrderBookDelta resource (separate from OrderBook) | 1d | 1h | 2.1 | |
| 2.5 OrderBook NativeV3 variant, acquire/release wiring | 2d | 1h | 2.3, 2.4 | |
| 2.6 market_types.move: create OrderBookDelta on market creation | 1d | 0.5h | 2.4 | |
| 2.7 Add `for_each_ref` to BigOrderedMap (if missing) | 5d | 4h | — | R1 |
| 2.8 Cold start rebuild in single_order_book.move | 2d | 1h | 2.7 | |
| 2.9 Cold start rebuild in bulk_order_book.move | 1d | 1h | 2.7 | |
| 2.10 migrate_to_native() (V1 → Native) | 2d | 1h | 2.7-2.9 | |
| 2.11 Feature flag (NATIVE_ORDER_BOOK_INDEX) | 1d | 0.5h | 2.5 | |

### Phase 3: VM Wiring & Registry (Weeks 7-8, 11 days)

| Task | Human | Claude | Deps | Risk |
|---|---|---|---|---|
| 3.1 NativeOrderBookContext accepts Arc\<OrderBookCache\> | 1d | 1h | P1 | |
| 3.2 Wire cache into session creation | 0.5d | 0.5h | 3.1 | |
| 3.3 Wire cache into test extensions | 0.5d | 0.5h | 3.1 | |
| 3.4 NativeOrderBookRegistry resource + register/unregister | 2d | 1h | P2 | |
| 3.5 NativeIndexCheckpoint resource + delta log | 2d | 2h | 3.4 | |
| 3.6 Pre-warm: Rust reads checkpoint blobs at VM startup | 3d | 2h | 3.4, 3.5 | |
| 3.7 Gas model definition | 2d | 1h | P1 | R10 |

### Phase 4: Testing & Correctness (Weeks 8-12, 25 days)

| Task | Human | Claude | Deps | Risk |
|---|---|---|---|---|
| 4.1 Move tests: place, match, cancel on Native | 3d | 2h | P2 | |
| 4.2 Move tests: bulk order match + level progression | 2d | 2h | P2 | R3 |
| 4.3 Move tests: migration V1→Native | 2d | 1h | 2.10 | |
| 4.4 Move tests: cold start rebuild | 1d | 1h | 2.8-2.9 | |
| 4.5 Block-STM tests (conflict, abort, re-execution) | 3d | 2h | P3 | R2 |
| 4.6 State sync recovery test | 1d | 1h | 3.6 | |
| 4.7 Shadow-mode: V1 vs Native parallel comparison | 5d | 3h | 4.1-4.4 | R12 |
| 4.8 Property-based / fuzz testing of overlay | 5d | 3h | P1 | R12 |
| 4.9 Gas calibration (benchmark + tune) | 3d | 2h | 3.7, 4.1 | |

### Phase 5: Benchmark & Forge (Weeks 12-16, 18 days)

| Task | Human | Claude | Deps | Risk |
|---|---|---|---|---|
| 5.1 Native workload variants in TransactionTypeArg | 1d | 1h | P2 | |
| 5.2 Native market setup in move_workloads.rs | 1d | 1h | 5.1 | |
| 5.3 A/B comparison benchmark (V1 vs Native) | 3d | 2h | 5.1-5.2 | |
| 5.4 Analyze results, identify regressions/wins | 2d | 1h | 5.3 | |
| 5.5 Native orderbook forge test suite | 2d | 2h | 5.1-5.2 | |
| 5.6 Tune forge TPS thresholds | 2d | 1h | 5.5 | R9 |
| 5.7 Validator restart mid-test forge variant | 2d | 1h | 5.5, 3.6 | |
| 5.8 Debug forge test flakiness | 5d | 4h | 5.5-5.7 | R9 |

### Phase 6: Code Review & Hardening (Weeks 14-16, 16 days, overlaps P5)

| Task | Human | Claude | Deps | Risk |
|---|---|---|---|---|
| 6.1 Review round 1: Rust overlay + cache | 3d | — | P1 | |
| 6.2 Review round 2: Move + migration | 3d | — | P2-P3 | |
| 6.3 Review round 3: Testing + forge | 2d | — | P4-P5 | |
| 6.4 Address review feedback | 5d | 4h | 6.1-6.3 | |
| 6.5 Determinism audit | 3d | 2h | All | R8 |

### Totals

| Phase | Human days | Human weeks | Claude hours |
|---|---|---|---|
| 1. Core Rust | 24 | 5 | 23 |
| 2. Move Integration | 21 | 4 | 16 |
| 3. VM & Registry | 11 | 2 | 8 |
| 4. Testing | 25 | 5 | 17 |
| 5. Benchmark & Forge | 18 | 4 | 13 |
| 6. Review & Hardening | 16 | 3 | 6 |
| **Total** | **115 days** | **~16-20 weeks** | **~83 hours** |

Critical path: ~16 weeks. With 2 engineers (Rust + Move): ~10-12 weeks.

## Risk Areas

| # | Risk | Likelihood | Impact | Extra time |
|---|---|---|---|---|
| R1 | BigOrderedMap `for_each_ref` missing | High | Must add to core framework | +1-2 weeks |
| R2 | Block-STM delta ordering bugs | Medium | Hard to reproduce, parallel-only | +1-2 weeks |
| R3 | Overlay bugs in matching flow | Medium | PriceTimeIndex ↔ BulkOrderBook back-and-forth | +1 week |
| R4 | Read-only handle acquisition perf | Low-Med | Temp overlay overhead on hot path | +3-5 days |
| R8 | Validator state divergence | Low | Consensus failure, critical safety | +2 weeks |
| R9 | Forge test flakiness | High | Non-deterministic failures | +1 week |
| R10 | Gas model gaming | Medium | Variable-cost ops exploitable | +3-5 days |
| R12 | Production readiness gap | High | Shadow-mode + fuzz testing needed | +3-4 weeks |

## Key Files

| File | Role |
|---|---|
| `aptos-move/framework/order-book-natives/src/lib.rs` | Core Rust: overlay, cache, all native functions |
| `aptos-move/framework/order-book-natives/Cargo.toml` | Deps: dashmap, parking_lot |
| `aptos-move/framework/aptos-experimental/sources/trading/order_book/price_time_index.move` | PriceTimeIndex: Native variant + dispatch |
| `aptos-move/framework/aptos-experimental/sources/trading/order_book/order_book.move` | OrderBook: NativeV3, OrderBookDelta wiring |
| `aptos-move/framework/aptos-experimental/sources/trading/order_book/single_order_book.move` | Cold start rebuild iteration |
| `aptos-move/framework/aptos-experimental/sources/trading/order_book/bulk_order_book.move` | Cold start rebuild iteration |
| `aptos-move/framework/aptos-experimental/sources/trading/market/market_types.move` | OrderBookDelta creation |
| `aptos-move/aptos-vm/src/move_vm_ext/session/mod.rs` | Cache → context wiring |
| `aptos-move/aptos-vm/src/natives.rs` | Cache → test context wiring |
| `testsuite/forge-cli/src/suites/realistic_environment.rs` | Forge test suite |
| `crates/transaction-workloads-lib/src/args.rs` | Benchmark workload variants |
| `crates/transaction-workloads-lib/src/move_workloads.rs` | Benchmark market setup |

## Performance Targets

| Metric | V1 Baseline | Native Target |
|---|---|---|
| Storage reads per match | ~6-8 | 0 + 1 (metadata) |
| Storage reads per place | ~3-4 | 0 + 1 (metadata) |
| Latency per match (µs) | ~500-1000 | ~10-50 |
| TPS (1 market, 80% overlap) | ~320 | ~800+ |
| TPS (50 markets, 80% overlap) | ~2000 | ~4500+ |
| Memory per 100K orders | 0 | ~4MB |
| Cold start per market | 0 | ~16ms (one-time) |
