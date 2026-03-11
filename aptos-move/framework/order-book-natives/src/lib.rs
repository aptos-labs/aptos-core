// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at
// https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

// SafeNativeResult<SmallVec<[Value; 1]>> is the standard return type for native functions.
#![allow(clippy::result_large_err)]

//! Native Rust implementation of the PriceTimeIndex for the order book.
//!
//! ## Design: Base + Delta Overlay
//!
//! The PriceTimeIndex is a derived view of active orders, kept in validator memory
//! as BTreeMaps. It is never stored on-chain. On cold start (validator restart),
//! the index is rebuilt from orders stored in BigOrderedMap.
//!
//! ### Per-TX Overlay
//! Each TX keeps one `OverlayIndex` per market. Reads walk the chain:
//! overlay → parent layer → ... → base. Writes go to overlay delta only.
//!
//! ### Per-Block State
//! `BlockNativeState` holds the base indices and layers from prior TXs in this block.
//! Each fork gets its own `BlockNativeState`, dropped when abandoned.
//!
//! ### Handle as Mapping
//! A version handle stored in `OrderBookVersion` (Move resource) is the native index's
//! sole representation in MVHashMap. Every mutation writes a new handle; reads depend
//! on it — enabling Block-STM conflict detection.
//!
//! ### Timing
//! `finalize()` stores the layer in `BlockNativeState` BEFORE `apply_updates` publishes
//! the handle to MVHashMap, ensuring a speculative TX always finds the layer when it
//! reads the new handle.

use aptos_native_interface::{
    safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext, SafeNativeError,
    SafeNativeResult,
};
use better_any::{Tid, TidAble};
use dashmap::DashMap;
use move_core_types::{
    account_address::AccountAddress, gas_algebra::InternalGas, identifier::Identifier,
};
use move_vm_runtime::{
    native_extensions::{NativeRuntimeRefCheckModelsCompleted, UnreachableSessionListener},
    native_functions::NativeFunctionTable,
};
use move_vm_types::{loaded_data::runtime_types::Type, values::Value};
use smallvec::{smallvec, SmallVec};
use std::{
    cell::RefCell,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet, VecDeque},
    sync::Arc,
};

use std::sync::atomic::{AtomicU64, Ordering as AtomicOrdering};
use std::time::Instant;

// ===========================================================================================
// Timing counters (global, atomic)

macro_rules! define_counter {
    ($name_calls:ident, $name_nanos:ident) => {
        static $name_calls: AtomicU64 = AtomicU64::new(0);
        static $name_nanos: AtomicU64 = AtomicU64::new(0);
    };
}

define_counter!(IS_ACQUIRED_CALLS, IS_ACQUIRED_NANOS);
define_counter!(ENSURE_ACQUIRED_CALLS, ENSURE_ACQUIRED_NANOS);
define_counter!(FLUSH_CALLS, FLUSH_NANOS);
define_counter!(BEST_BID_CALLS, BEST_BID_NANOS);
define_counter!(BEST_ASK_CALLS, BEST_ASK_NANOS);
define_counter!(PLACE_MAKER_CALLS, PLACE_MAKER_NANOS);
define_counter!(CANCEL_CALLS, CANCEL_NANOS);
define_counter!(IS_TAKER_CALLS, IS_TAKER_NANOS);
define_counter!(MATCH_CALLS, MATCH_NANOS);
define_counter!(REBUILD_ADD_CALLS, REBUILD_ADD_NANOS);
define_counter!(REBUILD_COMPLETE_CALLS, REBUILD_COMPLETE_NANOS);
define_counter!(MID_PRICE_CALLS, MID_PRICE_NANOS);
define_counter!(SLIPPAGE_CALLS, SLIPPAGE_NANOS);
define_counter!(INCREASE_CALLS, INCREASE_NANOS);
define_counter!(DECREASE_CALLS, DECREASE_NANOS);
define_counter!(FINALIZE_CALLS, FINALIZE_NANOS);

pub fn reset_native_timing_stats() {
    macro_rules! reset_counter {
        ($calls:ident, $nanos:ident) => {
            $calls.store(0, AtomicOrdering::Relaxed);
            $nanos.store(0, AtomicOrdering::Relaxed);
        };
    }
    reset_counter!(IS_ACQUIRED_CALLS, IS_ACQUIRED_NANOS);
    reset_counter!(ENSURE_ACQUIRED_CALLS, ENSURE_ACQUIRED_NANOS);
    reset_counter!(FLUSH_CALLS, FLUSH_NANOS);
    reset_counter!(BEST_BID_CALLS, BEST_BID_NANOS);
    reset_counter!(BEST_ASK_CALLS, BEST_ASK_NANOS);
    reset_counter!(PLACE_MAKER_CALLS, PLACE_MAKER_NANOS);
    reset_counter!(CANCEL_CALLS, CANCEL_NANOS);
    reset_counter!(IS_TAKER_CALLS, IS_TAKER_NANOS);
    reset_counter!(MATCH_CALLS, MATCH_NANOS);
    reset_counter!(REBUILD_ADD_CALLS, REBUILD_ADD_NANOS);
    reset_counter!(REBUILD_COMPLETE_CALLS, REBUILD_COMPLETE_NANOS);
    reset_counter!(MID_PRICE_CALLS, MID_PRICE_NANOS);
    reset_counter!(SLIPPAGE_CALLS, SLIPPAGE_NANOS);
    reset_counter!(INCREASE_CALLS, INCREASE_NANOS);
    reset_counter!(DECREASE_CALLS, DECREASE_NANOS);
    reset_counter!(FINALIZE_CALLS, FINALIZE_NANOS);
    for i in 0..NUM_PROBES {
        V1_PROBE_CALLS[i].store(0, AtomicOrdering::Relaxed);
        V1_PROBE_NANOS[i].store(0, AtomicOrdering::Relaxed);
        for ci in 0..NUM_CONTEXTS {
            CTX_PROBE_CALLS[ci][i].store(0, AtomicOrdering::Relaxed);
            CTX_PROBE_NANOS[ci][i].store(0, AtomicOrdering::Relaxed);
        }
    }
    reset_histograms();
    eprintln!("[NATIVE-TIMING] Counters reset");
}

pub fn print_native_timing_stats() {
    macro_rules! print_stat {
        ($label:expr, $calls:ident, $nanos:ident) => {
            let calls = $calls.load(AtomicOrdering::Relaxed);
            let nanos = $nanos.load(AtomicOrdering::Relaxed);
            if calls > 0 {
                println!(
                    "  {:30} calls={:>8}  total={:>10.3}ms  avg={:>8.0}ns",
                    $label, calls, nanos as f64 / 1_000_000.0, nanos as f64 / calls as f64
                );
            }
        };
    }
    println!("\n=== Native Orderbook Timing Stats ===");
    print_stat!("is_acquired", IS_ACQUIRED_CALLS, IS_ACQUIRED_NANOS);
    print_stat!("ensure_acquired", ENSURE_ACQUIRED_CALLS, ENSURE_ACQUIRED_NANOS);
    print_stat!("flush", FLUSH_CALLS, FLUSH_NANOS);
    print_stat!("best_bid_price", BEST_BID_CALLS, BEST_BID_NANOS);
    print_stat!("best_ask_price", BEST_ASK_CALLS, BEST_ASK_NANOS);
    print_stat!("place_maker_order", PLACE_MAKER_CALLS, PLACE_MAKER_NANOS);
    print_stat!("cancel_active_order", CANCEL_CALLS, CANCEL_NANOS);
    print_stat!("is_taker_order", IS_TAKER_CALLS, IS_TAKER_NANOS);
    print_stat!("get_single_match_result", MATCH_CALLS, MATCH_NANOS);
    print_stat!("rebuild_add", REBUILD_ADD_CALLS, REBUILD_ADD_NANOS);
    print_stat!("rebuild_complete", REBUILD_COMPLETE_CALLS, REBUILD_COMPLETE_NANOS);
    print_stat!("get_mid_price", MID_PRICE_CALLS, MID_PRICE_NANOS);
    print_stat!("get_slippage_price", SLIPPAGE_CALLS, SLIPPAGE_NANOS);
    print_stat!("increase_order_size", INCREASE_CALLS, INCREASE_NANOS);
    print_stat!("decrease_order_size", DECREASE_CALLS, DECREASE_NANOS);
    print_stat!("finalize", FINALIZE_CALLS, FINALIZE_NANOS);
    // Timing probes
    for i in 0..75u64 {
        let calls = V1_PROBE_CALLS[i as usize].load(AtomicOrdering::Relaxed);
        let nanos = V1_PROBE_NANOS[i as usize].load(AtomicOrdering::Relaxed);
        if calls > 0 {
            let label = match i {
                0 => "v1:best_bid_price",
                1 => "v1:best_ask_price",
                2 => "v1:place_maker_order",
                3 => "v1:cancel_active_order",
                4 => "v1:is_taker_order",
                5 => "v1:get_single_match_result",
                6 => "v1:get_mid_price",
                7 => "v1:increase_order_size",
                8 => "v1:decrease_order_size",
                9 => "v1:get_slippage_price",
                10 => "ob:place_maker_order",
                11 => "ob:cancel_single_order",
                12 => "ob:get_single_match",
                13 => "ob:is_taker_order",
                14 => "ob:best_bid_price",
                15 => "ob:best_ask_price",
                16 => "ob:place_bulk_order",
                17 => "ob:cancel_bulk_order",
                18 => "ob:try_cancel_w_client_id",
                19 => "ob:try_cancel_single",
                20 => "ob:reinsert_order",
                21 => "ob:get_bulk_order",
                22 => "ob:decrease_single_size",
                23 => "ob:get_single_order",
                24 => "ob:client_order_id_exists",
                25 => "ob:get_single_metadata",
                26 => "ob:set_single_metadata",
                27 => "ob:get_order_by_client_id",
                28 => "ob:take_price_based",
                29 => "ob:take_time_based",
                30 => "bob:orders.remove_or_none",
                31 => "bob:cancel_active_orders",
                32 => "bob:sanitize+best_price",
                33 => "bob:orders.add",
                34 => "bob:activate_levels",
                40 => "entry:place_order",
                41 => "entry:place_bulk_orders",
                42 => "entry:oracle_update",
                43 => "api:place_order",
                44 => "api:place_bulk_order",
                47 => "ch:validate_order",
                48 => "ch:validate_bulk_order",
                49 => "ch:settle_trade",
                56 => "pe:place_maker_or_queue",
                57 => "pe:trigger_match(order)",
                58 => "pe:trigger_match(bulk)",
                59 => "oracle:price_update",
                60 => "oracle:refresh_liq_trig",
                61 => "pe:bulk_order_placement",
                62 => "mbo:ch_validate",
                63 => "mbo:sanitize",
                64 => "mbo:destructure",
                65 => "mbo:event",
                66 => "mbo:ch_callback",
                67 => "om:get_position_info",
                68 => "om:free_collateral",
                69 => "om:pending_validate",
                70 => "cv:can_place+size_check",
                71 => "cv:collateral_validate",
                72 => "st:pre_checks",
                73 => "st:oi_cap",
                74 => "st:pending_order",
                75 => "st:post_settle",
                50 => "st:taker_validate",
                51 => "st:maker_validate",
                52 => "st:taker_commit",
                53 => "st:maker_commit",
                54 => "st:distribute_fees",
                55 => "st:reduce_only_check",
                _ => "unknown",
            };
            let ctx_names = ["MM", "Ret", "Orc"];
            let mut ctx_str = String::new();
            for ci in 0..NUM_CONTEXTS {
                let cc = CTX_PROBE_CALLS[ci][i as usize].load(AtomicOrdering::Relaxed);
                let cn = CTX_PROBE_NANOS[ci][i as usize].load(AtomicOrdering::Relaxed);
                if cc > 0 {
                    ctx_str += &format!(" {}:{}/{:.1}ms", ctx_names[ci], cc, cn as f64/1e6);
                }
            }
            println!(
                "  {:30} calls={:>8}  total={:>10.3}ms  avg={:>8.0}ns |{}",
                label, calls, nanos as f64 / 1_000_000.0, nanos as f64 / calls as f64, ctx_str
            );
        }
    }
    print_histograms();
}

const NUM_PROBES: usize = 80;
const NUM_CONTEXTS: usize = 3; // 0=MM, 1=Retail, 2=Oracle

// Per-context probe counters: CTX_PROBE_CALLS[ctx][label], CTX_PROBE_NANOS[ctx][label]
static CTX_PROBE_CALLS: [[AtomicU64; NUM_PROBES]; NUM_CONTEXTS] = {
    const INNER: [AtomicU64; NUM_PROBES] = {
        const Z: AtomicU64 = AtomicU64::new(0);
        [Z; NUM_PROBES]
    };
    [INNER; NUM_CONTEXTS]
};
static CTX_PROBE_NANOS: [[AtomicU64; NUM_PROBES]; NUM_CONTEXTS] = {
    const INNER: [AtomicU64; NUM_PROBES] = {
        const Z: AtomicU64 = AtomicU64::new(0);
        [Z; NUM_PROBES]
    };
    [INNER; NUM_CONTEXTS]
};

thread_local! {
    static CURRENT_CTX: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    // Gap tracking: record timestamps when specific probes end
    static GAP_TIMESTAMP: std::cell::Cell<Option<Instant>> = const { std::cell::Cell::new(None) };
    static GAP_LABEL: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// Gap probe virtual labels: 73=oi_cap (55→50), 74=pending_order (53→54), 75=post_settle (54→49)
fn record_gap_on_end(end_label: usize) {
    // When these probes END, record the timestamp for gap measurement
    match end_label {
        55 | 53 | 54 => {
            GAP_TIMESTAMP.with(|t| t.set(Some(Instant::now())));
            GAP_LABEL.with(|l| l.set(end_label));
        }
        _ => {}
    }
}

/// Called from native_timing_start. If a gap is being tracked,
/// the start of the next timer marks the end of the gap.
fn check_gap_on_next_start() {
    let gap_label = GAP_LABEL.with(|l| l.get());
    if gap_label == 0 { return; }

    if let Some(start) = GAP_TIMESTAMP.with(|t| t.get()) {
        let elapsed = start.elapsed().as_nanos() as u64;
        let virtual_label = match gap_label {
            55 => 73, // st:oi_cap (reduce_only_check end → taker_validate start)
            53 => 74, // st:pending_order (maker_commit end → distribute_fees start)
            54 => 75, // st:post_settle (distribute_fees end → next start)
            _ => { return; },
        };
        if virtual_label < NUM_PROBES {
            record_histogram(virtual_label, elapsed);
            V1_PROBE_CALLS[virtual_label].fetch_add(1, AtomicOrdering::Relaxed);
            V1_PROBE_NANOS[virtual_label].fetch_add(elapsed, AtomicOrdering::Relaxed);
            let ctx = CURRENT_CTX.with(|c| c.get());
            if ctx < NUM_CONTEXTS {
                CTX_PROBE_CALLS[ctx][virtual_label].fetch_add(1, AtomicOrdering::Relaxed);
                CTX_PROBE_NANOS[ctx][virtual_label].fetch_add(elapsed, AtomicOrdering::Relaxed);
            }
        }
    }
    GAP_LABEL.with(|l| l.set(0));
    GAP_TIMESTAMP.with(|t| t.set(None));
}

/// Called from native_timing_end for probe 49 (settle_trade end) to close
/// the post_settle gap, since there's no native_timing_start after it.
fn check_gap_on_settle_end(label: usize) {
    if label != 49 { return; }
    let gap_label = GAP_LABEL.with(|l| l.get());
    if gap_label != 54 { return; } // only close post_settle gap
    if let Some(start) = GAP_TIMESTAMP.with(|t| t.get()) {
        let elapsed = start.elapsed().as_nanos() as u64;
        let virtual_label = 75; // st:post_settle
        record_histogram(virtual_label, elapsed);
        V1_PROBE_CALLS[virtual_label].fetch_add(1, AtomicOrdering::Relaxed);
        V1_PROBE_NANOS[virtual_label].fetch_add(elapsed, AtomicOrdering::Relaxed);
        let ctx = CURRENT_CTX.with(|c| c.get());
        if ctx < NUM_CONTEXTS {
            CTX_PROBE_CALLS[ctx][virtual_label].fetch_add(1, AtomicOrdering::Relaxed);
            CTX_PROBE_NANOS[ctx][virtual_label].fetch_add(elapsed, AtomicOrdering::Relaxed);
        }
    }
    GAP_LABEL.with(|l| l.set(0));
    GAP_TIMESTAMP.with(|t| t.set(None));
}

static HIST: std::sync::LazyLock<std::sync::Mutex<Vec<Vec<u64>>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(vec![Vec::new(); NUM_PROBES]));

// Per-context histograms: CTX_HIST[ctx] = Vec<Vec<u64>> (one per probe)
static CTX_HIST: std::sync::LazyLock<std::sync::Mutex<Vec<Vec<Vec<u64>>>>> =
    std::sync::LazyLock::new(|| {
        std::sync::Mutex::new(vec![vec![Vec::new(); NUM_PROBES]; NUM_CONTEXTS])
    });

fn record_histogram(label: usize, elapsed_ns: u64) {
    fn push_sample(v: &mut Vec<u64>, val: u64) {
        if v.len() < 500_000 {
            v.push(val);
        } else {
            let idx = (val as usize) % v.len();
            v[idx] = val;
        }
    }
    if label < NUM_PROBES {
        if let Ok(mut histograms) = HIST.try_lock() {
            push_sample(&mut histograms[label], elapsed_ns);
        }
        let ctx = CURRENT_CTX.with(|c| c.get());
        if ctx < NUM_CONTEXTS {
            if let Ok(mut ctx_hist) = CTX_HIST.try_lock() {
                push_sample(&mut ctx_hist[ctx][label], elapsed_ns);
            }
        }
    }
}

fn histogram_labels() -> Vec<(usize, &'static str)> {
    vec![
        (40, "entry:place_order"),
        (41, "entry:place_bulk_orders"),
        (42, "entry:oracle_update"),
        (49, "ch:settle_trade"),
        (50, "st:taker_validate"),
        (51, "st:maker_validate"),
        (52, "st:taker_commit"),
        (53, "st:maker_commit"),
        (54, "st:distribute_fees"),
        (55, "st:reduce_only_check"),
        (16, "ob:place_bulk_order"),
        (10, "ob:place_maker_order"),
        (12, "ob:get_single_match"),
        (13, "ob:is_taker_order"),
        (14, "ob:best_bid_price"),
        (15, "ob:best_ask_price"),
        (48, "ch:validate_bulk_order"),
        (47, "ch:validate_order"),
        (57, "pe:trigger_match(order)"),
        (58, "pe:trigger_match(bulk)"),
        (60, "oracle:refresh_liq_trig"),
        (61, "pe:bulk_order_placement"),
        (62, "mbo:ch_validate"),
        (63, "mbo:sanitize"),
        (64, "mbo:destructure"),
        (65, "mbo:event"),
        (66, "mbo:ch_callback"),
        (67, "om:get_position_info"),
        (68, "om:free_collateral"),
        (69, "om:pending_validate"),
        (70, "cv:can_place+size_check"),
        (72, "st:pre_checks"),
        (73, "st:oi_cap"),
        (74, "st:pending_order"),
        (75, "st:post_settle"),
    ]
}

fn print_histogram_table(title: &str, histograms: &[Vec<u64>]) {
    let labels = histogram_labels();
    println!("\n=== {} ===", title);
    println!("  {:35} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "Timer", "p50", "p90", "p99", "max", "avg", "count");
    println!("  {}", "-".repeat(85));
    for (idx, label) in &labels {
        let v = &histograms[*idx];
        if v.is_empty() { continue; }
        let mut sorted = v.clone();
        sorted.sort();
        let n = sorted.len();
        let p50 = sorted[n * 50 / 100];
        let p90 = sorted[n * 90 / 100];
        let p99 = sorted[n * 99 / 100];
        let max = sorted[n - 1];
        let avg: u64 = sorted.iter().sum::<u64>() / n as u64;
        println!("  {:35} {:>7.0} {:>7.0} {:>7.0} {:>7.0} {:>7.0} {:>7}",
            label,
            p50 as f64 / 1000.0, p90 as f64 / 1000.0,
            p99 as f64 / 1000.0, max as f64 / 1000.0,
            avg as f64 / 1000.0, n);
    }
}

fn print_histograms() {
    if let Ok(histograms) = HIST.lock() {
        print_histogram_table("Percentile Distribution (μs) — ALL", &histograms);
    }
    let ctx_names = ["MM (ctx=0)", "Retail (ctx=1)", "Oracle (ctx=2)"];
    if let Ok(ctx_hist) = CTX_HIST.lock() {
        for ci in 0..NUM_CONTEXTS {
            print_histogram_table(
                &format!("Percentile Distribution (μs) — {}", ctx_names[ci]),
                &ctx_hist[ci],
            );
        }
    }
}

/// Call to print histogram data. Safe from any thread since HIST is global.
pub fn print_timing_histograms() {
    print_histograms();
}

fn reset_histograms() {
    if let Ok(mut histograms) = HIST.lock() {
        for v in histograms.iter_mut() {
            v.clear();
        }
    }
    if let Ok(mut ctx_hist) = CTX_HIST.lock() {
        for ctx in ctx_hist.iter_mut() {
            for v in ctx.iter_mut() {
                v.clear();
            }
        }
    }
}
static V1_PROBE_CALLS: [AtomicU64; NUM_PROBES] = {
    const ZERO: AtomicU64 = AtomicU64::new(0);
    [ZERO; NUM_PROBES]
};
static V1_PROBE_NANOS: [AtomicU64; NUM_PROBES] = {
    const ZERO: AtomicU64 = AtomicU64::new(0);
    [ZERO; NUM_PROBES]
};

macro_rules! timed {
    ($calls:ident, $nanos:ident, $body:expr) => {{
        let _start = Instant::now();
        let _result = $body;
        let _elapsed = _start.elapsed().as_nanos() as u64;
        $calls.fetch_add(1, AtomicOrdering::Relaxed);
        $nanos.fetch_add(_elapsed, AtomicOrdering::Relaxed);
        _result
    }};
}

// ===========================================================================================
// Constants

const MAX_U128: u128 = u128::MAX;
const SLIPPAGE_PCT_PRECISION: u64 = 100;

// Gas constants (placeholder — should be calibrated in Phase 4.10)
const BASE_GAS: InternalGas = InternalGas::new(300);

// ===========================================================================================
// Key Types

/// Key for buy orders. BTreeMap sorts ascending, so the *last* entry is the best bid
/// (highest price, earliest time via DecreasingIdx tie-breaker).
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Debug)]
struct BuyKey {
    price: u64,
    tie_breaker: u128, // DecreasingIdx = MAX_U128 - IncreasingIdx
}

/// Key for sell orders. BTreeMap sorts ascending, so the *first* entry is the best ask
/// (lowest price, earliest time via IncreasingIdx tie-breaker).
#[derive(Ord, PartialOrd, Eq, PartialEq, Clone, Debug)]
struct SellKey {
    price: u64,
    tie_breaker: u128, // IncreasingIdx value
}

#[derive(Clone, Debug)]
struct OrderData {
    order_id: u128,
    order_type: u16,
    size: u64,
}

// ===========================================================================================
// Overlay Data Structures

/// Immutable base state for a market. Populated from the committed/parent block's
/// finalized state, or from a cold-start rebuild.
pub struct PriceTimeBase {
    buys: BTreeMap<BuyKey, OrderData>,
    sells: BTreeMap<SellKey, OrderData>,
}

/// Pointer to the parent state in the overlay chain.
pub enum PriceTimeParent {
    Base(Arc<PriceTimeBase>),
    Layer(Arc<PriceTimeLayer>),
}

/// Immutable delta from a completed TX. Stored in `BlockNativeState.layers`.
pub struct PriceTimeLayer {
    delta_buys: BTreeMap<BuyKey, Option<OrderData>>,  // Some=upsert, None=tombstone
    delta_sells: BTreeMap<SellKey, Option<OrderData>>,
    parent: PriceTimeParent,
}

/// Current TX's mutable overlay for one market.
struct OverlayIndex {
    delta_buys: BTreeMap<BuyKey, Option<OrderData>>,
    delta_sells: BTreeMap<SellKey, Option<OrderData>>,
    parent: PriceTimeParent,
    modified: bool,
}

// ===========================================================================================
// Chain Walk Trait — shared between OverlayIndex and PriceTimeLayer

trait DeltaState {
    fn delta_buys(&self) -> &BTreeMap<BuyKey, Option<OrderData>>;
    fn delta_sells(&self) -> &BTreeMap<SellKey, Option<OrderData>>;
    fn parent(&self) -> &PriceTimeParent;
}

impl DeltaState for OverlayIndex {
    fn delta_buys(&self) -> &BTreeMap<BuyKey, Option<OrderData>> { &self.delta_buys }
    fn delta_sells(&self) -> &BTreeMap<SellKey, Option<OrderData>> { &self.delta_sells }
    fn parent(&self) -> &PriceTimeParent { &self.parent }
}

impl DeltaState for PriceTimeLayer {
    fn delta_buys(&self) -> &BTreeMap<BuyKey, Option<OrderData>> { &self.delta_buys }
    fn delta_sells(&self) -> &BTreeMap<SellKey, Option<OrderData>> { &self.delta_sells }
    fn parent(&self) -> &PriceTimeParent { &self.parent }
}

// ===========================================================================================
// Chain Walk Functions
//
// Walk from current layer down to base, collecting tombstones and tracking the best entry.
// Chain depth = 1 per TX that modified this market in this block (typically 1–5).

/// Find the best (max) BuyKey + OrderData across the overlay chain.
fn find_best_buy(state: &dyn DeltaState) -> Option<(BuyKey, OrderData)> {
    let mut tombstones = BTreeSet::new();
    let mut best: Option<(BuyKey, OrderData)> = None;
    walk_find_best_buy(state, &mut tombstones, &mut best);
    best
}

fn walk_find_best_buy(
    state: &dyn DeltaState,
    tombstones: &mut BTreeSet<BuyKey>,
    best: &mut Option<(BuyKey, OrderData)>,
) {
    for (k, v) in state.delta_buys() {
        match v {
            Some(data) if !tombstones.contains(k) => {
                if best.as_ref().is_none_or(|(b, _)| k > b) {
                    *best = Some((k.clone(), data.clone()));
                }
            },
            None => { tombstones.insert(k.clone()); },
            _ => {}, // tombstoned upsert from higher layer — skip
        }
    }
    match state.parent() {
        PriceTimeParent::Layer(layer) => walk_find_best_buy(layer.as_ref(), tombstones, best),
        PriceTimeParent::Base(base) => {
            // Base is sorted ascending; iterate from back (highest) to find best.
            for (k, data) in base.buys.iter().rev() {
                if !tombstones.contains(k) {
                    if best.as_ref().is_none_or(|(b, _)| k > b) {
                        *best = Some((k.clone(), data.clone()));
                    }
                    break; // Sorted — no later entry can beat this.
                }
            }
        },
    }
}

/// Find the best (min) SellKey + OrderData across the overlay chain.
fn find_best_sell(state: &dyn DeltaState) -> Option<(SellKey, OrderData)> {
    let mut tombstones = BTreeSet::new();
    let mut best: Option<(SellKey, OrderData)> = None;
    walk_find_best_sell(state, &mut tombstones, &mut best);
    best
}

fn walk_find_best_sell(
    state: &dyn DeltaState,
    tombstones: &mut BTreeSet<SellKey>,
    best: &mut Option<(SellKey, OrderData)>,
) {
    for (k, v) in state.delta_sells() {
        match v {
            Some(data) if !tombstones.contains(k) => {
                if best.as_ref().is_none_or(|(b, _)| k < b) {
                    *best = Some((k.clone(), data.clone()));
                }
            },
            None => { tombstones.insert(k.clone()); },
            _ => {},
        }
    }
    match state.parent() {
        PriceTimeParent::Layer(layer) => walk_find_best_sell(layer.as_ref(), tombstones, best),
        PriceTimeParent::Base(base) => {
            for (k, data) in base.sells.iter() {
                if !tombstones.contains(k) {
                    if best.as_ref().is_none_or(|(b, _)| k < b) {
                        *best = Some((k.clone(), data.clone()));
                    }
                    break;
                }
            }
        },
    }
}

/// Look up a specific BuyKey's OrderData in the chain. Returns None if tombstoned or absent.
fn lookup_buy(state: &dyn DeltaState, key: &BuyKey) -> Option<OrderData> {
    if let Some(entry) = state.delta_buys().get(key) {
        return entry.clone(); // Some(data) or None (tombstone)
    }
    match state.parent() {
        PriceTimeParent::Layer(layer) => lookup_buy(layer.as_ref(), key),
        PriceTimeParent::Base(base) => base.buys.get(key).cloned(),
    }
}

/// Look up a specific SellKey's OrderData in the chain.
fn lookup_sell(state: &dyn DeltaState, key: &SellKey) -> Option<OrderData> {
    if let Some(entry) = state.delta_sells().get(key) {
        return entry.clone();
    }
    match state.parent() {
        PriceTimeParent::Layer(layer) => lookup_sell(layer.as_ref(), key),
        PriceTimeParent::Base(base) => base.sells.get(key).cloned(),
    }
}

// ===========================================================================================
// OverlayIndex Implementation

impl OverlayIndex {
    fn new(parent: PriceTimeParent) -> Self {
        Self {
            delta_buys: BTreeMap::new(),
            delta_sells: BTreeMap::new(),
            parent,
            modified: false,
        }
    }

    // ----- Read operations -----

    fn best_bid_price(&self) -> Option<u64> {
        find_best_buy(self).map(|(k, _)| k.price)
    }

    fn best_ask_price(&self) -> Option<u64> {
        find_best_sell(self).map(|(k, _)| k.price)
    }

    fn get_mid_price(&self) -> Option<u64> {
        let bid = self.best_bid_price()?;
        let ask = self.best_ask_price()?;
        // Overflow-safe mid-price: bid/2 + ask/2 + (bid%2 + ask%2)/2
        Some(bid / 2 + ask / 2 + (bid % 2 + ask % 2) / 2)
    }

    fn get_slippage_price(&self, is_bid: bool, slippage_bps: u64) -> Result<Option<u64>, u64> {
        if !is_bid && slippage_bps > SLIPPAGE_PCT_PRECISION * 100 {
            return Err(3); // EINVALID_SLIPPAGE_BPS
        }
        let mid_price = match self.get_mid_price() {
            Some(p) => p,
            None => return Ok(None),
        };
        let slippage = mid_price
            .checked_mul(slippage_bps)
            .and_then(|v| v.checked_div(SLIPPAGE_PCT_PRECISION * 100))
            .unwrap_or(0);
        if is_bid {
            Ok(Some(mid_price + slippage))
        } else {
            Ok(Some(mid_price.saturating_sub(slippage)))
        }
    }

    fn is_taker_order(&self, price: u64, is_bid: bool) -> bool {
        if is_bid {
            self.best_ask_price().is_some_and(|ask| price >= ask)
        } else {
            self.best_bid_price().is_some_and(|bid| price <= bid)
        }
    }

    // ----- Write operations -----

    fn place_maker_order(
        &mut self,
        order_id: u128,
        order_type: u16,
        price: u64,
        unique_priority_idx: u128,
        size: u64,
        is_bid: bool,
    ) -> Result<(), u64> {
        if self.is_taker_order(price, is_bid) {
            return Err(1); // EINVALID_MAKER_ORDER
        }
        let data = OrderData { order_id, order_type, size };
        if is_bid {
            let key = BuyKey { price, tie_breaker: MAX_U128 - unique_priority_idx };
            self.delta_buys.insert(key, Some(data));
        } else {
            let key = SellKey { price, tie_breaker: unique_priority_idx };
            self.delta_sells.insert(key, Some(data));
        }
        self.modified = true;
        Ok(())
    }

    fn cancel_active_order(
        &mut self,
        price: u64,
        unique_priority_idx: u128,
        is_bid: bool,
    ) -> Result<u64, u64> {
        if is_bid {
            let key = BuyKey { price, tie_breaker: MAX_U128 - unique_priority_idx };
            let data = match lookup_buy(self, &key) {
                Some(d) => d,
                None => return Err(0x0D_0001), // SPECULATIVE_ABORT: order not found
            };
            let size = data.size;
            self.delta_buys.insert(key, None); // tombstone
            self.modified = true;
            Ok(size)
        } else {
            let key = SellKey { price, tie_breaker: unique_priority_idx };
            let data = match lookup_sell(self, &key) {
                Some(d) => d,
                None => return Err(0x0D_0002), // SPECULATIVE_ABORT: order not found
            };
            let size = data.size;
            self.delta_sells.insert(key, None);
            self.modified = true;
            Ok(size)
        }
    }

    fn increase_order_size(
        &mut self,
        price: u64,
        unique_priority_idx: u128,
        size_delta: u64,
        is_bid: bool,
    ) -> Result<(), u64> {
        if is_bid {
            let key = BuyKey { price, tie_breaker: MAX_U128 - unique_priority_idx };
            let mut data = lookup_buy(self, &key).ok_or(0x0D_0003u64)?;
            data.size += size_delta;
            self.delta_buys.insert(key, Some(data));
        } else {
            let key = SellKey { price, tie_breaker: unique_priority_idx };
            let mut data = lookup_sell(self, &key).ok_or(0x0D_0004u64)?;
            data.size += size_delta;
            self.delta_sells.insert(key, Some(data));
        }
        self.modified = true;
        Ok(())
    }

    fn decrease_order_size(
        &mut self,
        price: u64,
        unique_priority_idx: u128,
        size_delta: u64,
        is_bid: bool,
    ) -> Result<(), u64> {
        if is_bid {
            let key = BuyKey { price, tie_breaker: MAX_U128 - unique_priority_idx };
            let mut data = lookup_buy(self, &key).ok_or(0x0D_0005u64)?;
            data.size -= size_delta;
            self.delta_buys.insert(key, Some(data));
        } else {
            let key = SellKey { price, tie_breaker: unique_priority_idx };
            let mut data = lookup_sell(self, &key).ok_or(0x0D_0006u64)?;
            data.size -= size_delta;
            self.delta_sells.insert(key, Some(data));
        }
        self.modified = true;
        Ok(())
    }

    // ----- Match -----

    /// Match a taker order against the best counterparty.
    /// Returns (order_id, matched_size, remaining_maker_size, order_type).
    fn get_single_match_result(
        &mut self,
        price: u64,
        size: u64,
        is_bid: bool,
    ) -> (u128, u64, u64, u16) {
        if is_bid {
            let (key, data) = find_best_sell(self).expect("no sell orders to match against");
            assert!(price >= key.price, "internal invariant broken");

            let is_fully_consumed = data.size <= size;
            let matched_size = if is_fully_consumed { data.size } else { size };
            let remaining = data.size - matched_size;

            if is_fully_consumed {
                self.delta_sells.insert(key, None);
            } else {
                let reduced = OrderData { size: remaining, ..data.clone() };
                self.delta_sells.insert(key, Some(reduced));
            }
            self.modified = true;
            (data.order_id, matched_size, remaining, data.order_type)
        } else {
            let (key, data) = find_best_buy(self).expect("no buy orders to match against");
            assert!(price <= key.price, "internal invariant broken");

            let is_fully_consumed = data.size <= size;
            let matched_size = if is_fully_consumed { data.size } else { size };
            let remaining = data.size - matched_size;

            if is_fully_consumed {
                self.delta_buys.insert(key, None);
            } else {
                let reduced = OrderData { size: remaining, ..data.clone() };
                self.delta_buys.insert(key, Some(reduced));
            }
            self.modified = true;
            (data.order_id, matched_size, remaining, data.order_type)
        }
    }

    // ----- Conversion -----

    fn into_layer(self) -> PriceTimeLayer {
        PriceTimeLayer {
            delta_buys: self.delta_buys,
            delta_sells: self.delta_sells,
            parent: self.parent,
        }
    }
}

// ===========================================================================================
// PriceTimeBase

impl PriceTimeBase {
    pub fn empty() -> Self {
        Self { buys: BTreeMap::new(), sells: BTreeMap::new() }
    }

    /// Flatten a layer chain into a single base by applying all deltas bottom-up.
    pub fn flatten_from(layer: &PriceTimeLayer) -> Self {
        // Collect base
        let base = Self::get_base_ref(layer);
        let mut buys: BTreeMap<BuyKey, OrderData> =
            base.buys.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        let mut sells: BTreeMap<SellKey, OrderData> =
            base.sells.iter().map(|(k, v)| (k.clone(), v.clone())).collect();

        // Collect layers bottom-up
        let mut layers_stack = vec![layer];
        let mut cur = layer;
        while let PriceTimeParent::Layer(parent) = &cur.parent {
            layers_stack.push(parent.as_ref());
            cur = parent.as_ref();
        }

        // Apply bottom-up (reverse of the stack we built top-down)
        for l in layers_stack.into_iter().rev() {
            for (k, v) in &l.delta_buys {
                match v {
                    Some(data) => { buys.insert(k.clone(), data.clone()); },
                    None => { buys.remove(k); },
                }
            }
            for (k, v) in &l.delta_sells {
                match v {
                    Some(data) => { sells.insert(k.clone(), data.clone()); },
                    None => { sells.remove(k); },
                }
            }
        }

        PriceTimeBase { buys, sells }
    }

    fn get_base_ref(layer: &PriceTimeLayer) -> &PriceTimeBase {
        match &layer.parent {
            PriceTimeParent::Base(base) => base,
            PriceTimeParent::Layer(parent) => Self::get_base_ref(parent),
        }
    }
}

// ===========================================================================================
// Block-Level State

/// Per-block execution state. Each consensus fork gets its own instance.
/// All fields use DashMap for interior mutability — allowing finalize_block
/// to work through a shared Arc reference after block execution completes.
pub struct BlockNativeState {
    /// Base indices per market, from parent block's finalized state.
    pub bases: DashMap<AccountAddress, Arc<PriceTimeBase>>,
    /// Handle values at block start per market.
    pub start_handles: DashMap<AccountAddress, u64>,
    /// Layers from completed TXs in this block. Key = (market_addr, handle).
    pub layers: DashMap<(AccountAddress, u64), Arc<PriceTimeLayer>>,
}

impl Default for BlockNativeState {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for BlockNativeState {
    fn drop(&mut self) {
        print_native_timing_stats();
    }
}

impl BlockNativeState {
    pub fn new() -> Self {
        Self {
            bases: DashMap::new(),
            start_handles: DashMap::new(),
            layers: DashMap::new(),
        }
    }

    /// Create state for a child block from parent's finalized state.
    pub fn from_parent(parent: &BlockNativeState) -> Self {
        let bases = DashMap::new();
        for entry in parent.bases.iter() {
            bases.insert(*entry.key(), entry.value().clone());
        }
        let start_handles = DashMap::new();
        for entry in parent.start_handles.iter() {
            start_handles.insert(*entry.key(), *entry.value());
        }
        Self { bases, start_handles, layers: DashMap::new() }
    }

    /// Create state from committed (post-commit) state.
    pub fn from_committed(committed: &CommittedOrderBookState) -> Self {
        let bases = DashMap::new();
        for entry in committed.bases.iter() {
            bases.insert(*entry.key(), entry.value().clone());
        }
        let start_handles = DashMap::new();
        for entry in committed.start_handles.iter() {
            start_handles.insert(*entry.key(), *entry.value());
        }
        Self { bases, start_handles, layers: DashMap::new() }
    }

    /// After Block-STM converges, flatten layer chains into new bases for child blocks.
    /// Takes &self (not &mut self) because fields use DashMap for interior mutability.
    pub fn finalize_block(&self) {
        // Collect all markets that have layers in this block
        let mut final_handles: HashMap<AccountAddress, u64> = HashMap::new();
        for entry in self.layers.iter() {
            let (addr, handle) = entry.key();
            let current = final_handles.entry(*addr).or_insert(0);
            if *handle > *current {
                *current = *handle;
            }
        }

        // For each modified market, flatten the final layer into a new base
        for (addr, final_handle) in &final_handles {
            if let Some(layer_ref) = self.layers.get(&(*addr, *final_handle)) {
                let new_base = Arc::new(PriceTimeBase::flatten_from(layer_ref.value()));
                self.bases.insert(*addr, new_base);
                self.start_handles.insert(*addr, *final_handle);
            }
        }

        // Clear layers (no longer needed — children use bases)
        self.layers.clear();
    }
}

/// Committed state promoted after a block is committed. Shared across the validator.
pub struct CommittedOrderBookState {
    pub bases: DashMap<AccountAddress, Arc<PriceTimeBase>>,
    pub start_handles: DashMap<AccountAddress, u64>,
}

impl Default for CommittedOrderBookState {
    fn default() -> Self {
        Self::new()
    }
}

impl CommittedOrderBookState {
    pub fn new() -> Self {
        Self { bases: DashMap::new(), start_handles: DashMap::new() }
    }

    /// Promote a finalized block's state to committed.
    pub fn commit(&self, block_state: &BlockNativeState) {
        for entry in block_state.bases.iter() {
            self.bases.insert(*entry.key(), entry.value().clone());
        }
        for entry in block_state.start_handles.iter() {
            self.start_handles.insert(*entry.key(), *entry.value());
        }
    }
}

// ===========================================================================================
// Native Context

/// Per-TX session extension. Created per session with a reference to the block's native state.
#[derive(Tid)]
pub struct NativeOrderBookContext {
    block_state: Arc<BlockNativeState>,
    /// Active overlays for markets touched in this TX.
    active: RefCell<HashMap<AccountAddress, OverlayIndex>>,
    /// Markets that were modified and need a new handle written.
    pending: RefCell<HashMap<AccountAddress, u64>>,
    /// Markets that need a cold-start rebuild.
    needs_rebuild: RefCell<HashSet<AccountAddress>>,
}

impl UnreachableSessionListener for NativeOrderBookContext {}

impl NativeRuntimeRefCheckModelsCompleted for NativeOrderBookContext {}

impl NativeOrderBookContext {
    /// Create with an empty standalone BlockNativeState (all markets cold-start).
    /// Phase 3 will replace this with proper block state lifecycle.
    pub fn new() -> Self {
        Self::new_with_block_state(Arc::new(BlockNativeState::new()))
    }

    pub fn new_with_block_state(block_state: Arc<BlockNativeState>) -> Self {
        Self {
            block_state,
            active: RefCell::new(HashMap::new()),
            pending: RefCell::new(HashMap::new()),
            needs_rebuild: RefCell::new(HashSet::new()),
        }
    }

    /// Acquire or reuse the overlay for a market. Returns true if cold-start rebuild is needed.
    ///
    /// Called by `native_ensure_acquired` (from OrderBook.ensure_native_index_ready).
    /// The handle comes from `OrderBookVersion` in MVHashMap — creating the Block-STM
    /// read dependency.
    fn ensure_acquired(&self, addr: AccountAddress, handle: u64) -> bool {
        let mut active = self.active.borrow_mut();
        if active.contains_key(&addr) {
            return self.needs_rebuild.borrow().contains(&addr);
        }

        let parent = match self.block_state.start_handles.get(&addr) {
            Some(sh) if handle == *sh => {
                // No prior TX modified this market. Use base.
                PriceTimeParent::Base(
                    self.block_state.bases.get(&addr).expect("base not found").value().clone()
                )
            },
            Some(_) => {
                // Prior TX modified. Handle points to their layer.
                PriceTimeParent::Layer(
                    self.block_state
                        .layers
                        .get(&(addr, handle))
                        .expect("layer not found for handle")
                        .value()
                        .clone(),
                )
            },
            None => {
                // Market not in start_handles. Check if a prior TX in this block rebuilt it.
                match self.block_state.layers.get(&(addr, handle)) {
                    Some(layer) => PriceTimeParent::Layer(layer.value().clone()),
                    None => {
                        // True cold start — create empty overlay, mark for rebuild.
                        active.insert(
                            addr,
                            OverlayIndex::new(PriceTimeParent::Base(Arc::new(
                                PriceTimeBase::empty(),
                            ))),
                        );
                        self.needs_rebuild.borrow_mut().insert(addr);
                        return true;
                    },
                }
            },
        };

        active.insert(addr, OverlayIndex::new(parent));
        false
    }

    /// Mark a market as flushed with a new handle. Called by `native_flush`.
    fn flush(&self, addr: AccountAddress, new_handle: u64) -> bool {
        let active = self.active.borrow();
        match active.get(&addr) {
            Some(overlay) if overlay.modified => {
                self.pending.borrow_mut().insert(addr, new_handle);
                true
            },
            _ => false,
        }
    }

    /// Move overlays for pending markets into `BlockNativeState.layers`.
    /// Also auto-flushes any modified overlays that weren't explicitly flushed via
    /// `maybe_flush_handle`. Returns the list of (market_addr, new_handle) pairs that
    /// were auto-flushed and need `OrderBookVersion` writes injected into the change set.
    ///
    /// **Must be called after TX execution but BEFORE `apply_updates` writes to MVHashMap.**
    pub fn finalize(&self) -> Vec<(AccountAddress, u64)> {
        let _start = Instant::now();
        let _result = self.finalize_inner();
        FINALIZE_CALLS.fetch_add(1, AtomicOrdering::Relaxed);
        FINALIZE_NANOS.fetch_add(_start.elapsed().as_nanos() as u64, AtomicOrdering::Relaxed);
        _result
    }

    fn finalize_inner(&self) -> Vec<(AccountAddress, u64)> {
        let mut pending = self.pending.borrow_mut();
        let active = self.active.borrow();
        let mut auto_flushed = Vec::new();

        // Auto-flush modified overlays that weren't explicitly flushed.
        for (addr, overlay) in active.iter() {
            if overlay.modified && !pending.contains_key(addr) {
                // Use the overlay's start_handle + 1 as the new handle.
                let new_handle = self
                    .block_state
                    .start_handles
                    .get(addr)
                    .map_or(1, |h| *h + 1);
                pending.insert(*addr, new_handle);
                auto_flushed.push((*addr, new_handle));
            }
        }
        drop(active);

        let mut active = self.active.borrow_mut();
        for (addr, handle) in pending.iter() {
            let overlay = active.remove(addr).expect("pending market not in active");
            self.block_state
                .layers
                .insert((*addr, *handle), Arc::new(overlay.into_layer()));
        }

        auto_flushed
    }
}

impl Default for NativeOrderBookContext {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================================
// Native Functions — PriceTimeIndex module (operation natives)
//
// These take market_addr as first arg. The overlay must already be acquired
// (via native_ensure_acquired called from OrderBook.ensure_native_index_ready).

fn native_best_bid_price(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(BEST_BID_CALLS, BEST_BID_NANOS, {
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let active = ctx.active.borrow();
    let overlay = active.get(&market_addr).expect("overlay not acquired");
    match overlay.best_bid_price() {
        Some(price) => Ok(smallvec![Value::bool(true), Value::u64(price)]),
        None => Ok(smallvec![Value::bool(false), Value::u64(0)]),
    }
})
}

fn native_best_ask_price(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(BEST_ASK_CALLS, BEST_ASK_NANOS, {
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let active = ctx.active.borrow();
    let overlay = active.get(&market_addr).expect("overlay not acquired");
    match overlay.best_ask_price() {
        Some(price) => Ok(smallvec![Value::bool(true), Value::u64(price)]),
        None => Ok(smallvec![Value::bool(false), Value::u64(0)]),
    }
})
}

fn native_get_mid_price(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(MID_PRICE_CALLS, MID_PRICE_NANOS, {
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let active = ctx.active.borrow();
    let overlay = active.get(&market_addr).expect("overlay not acquired");
    match overlay.get_mid_price() {
        Some(mid) => Ok(smallvec![Value::bool(true), Value::u64(mid)]),
        None => Ok(smallvec![Value::bool(false), Value::u64(0)]),
    }
})
}

fn native_get_slippage_price(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(SLIPPAGE_CALLS, SLIPPAGE_NANOS, {
    let slippage_bps = safely_pop_arg!(args, u64);
    let is_bid = safely_pop_arg!(args, bool);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let active = ctx.active.borrow();
    let overlay = active.get(&market_addr).expect("overlay not acquired");
    match overlay.get_slippage_price(is_bid, slippage_bps) {
        Ok(Some(price)) => Ok(smallvec![Value::bool(true), Value::u64(price)]),
        Ok(None) => Ok(smallvec![Value::bool(false), Value::u64(0)]),
        Err(code) => Err(SafeNativeError::abort(code)),
    }
})
}

fn native_is_taker_order(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(IS_TAKER_CALLS, IS_TAKER_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let price = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let active = ctx.active.borrow();
    let overlay = active.get(&market_addr).expect("overlay not acquired");
    Ok(smallvec![Value::bool(overlay.is_taker_order(price, is_bid))])
})
}

fn native_place_maker_order(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(PLACE_MAKER_CALLS, PLACE_MAKER_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let size = safely_pop_arg!(args, u64);
    let unique_priority_idx = safely_pop_arg!(args, u128);
    let price = safely_pop_arg!(args, u64);
    let order_type = safely_pop_arg!(args, u64) as u16;
    let order_id = safely_pop_arg!(args, u128);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let mut active = ctx.active.borrow_mut();
    let overlay = active.get_mut(&market_addr).expect("overlay not acquired");
    overlay
        .place_maker_order(order_id, order_type, price, unique_priority_idx, size, is_bid)
        .map_err(SafeNativeError::abort)?;
    Ok(smallvec![])
})
}

fn native_cancel_active_order(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(CANCEL_CALLS, CANCEL_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let unique_priority_idx = safely_pop_arg!(args, u128);
    let price = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let mut active = ctx.active.borrow_mut();
    let overlay = active.get_mut(&market_addr).expect("overlay not acquired");
    let size = overlay.cancel_active_order(price, unique_priority_idx, is_bid)
        .map_err(SafeNativeError::abort)?;
    Ok(smallvec![Value::u64(size)])
})
}

fn native_get_single_match_result(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(MATCH_CALLS, MATCH_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let size = safely_pop_arg!(args, u64);
    let price = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let mut active = ctx.active.borrow_mut();
    let overlay = active.get_mut(&market_addr).expect("overlay not acquired");
    let (order_id, matched_size, remaining_maker_size, order_type) =
        overlay.get_single_match_result(price, size, is_bid);

    Ok(smallvec![
        Value::u128(order_id),
        Value::u64(matched_size),
        Value::u64(remaining_maker_size),
        Value::u64(order_type as u64),
    ])
})
}

fn native_increase_order_size(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(INCREASE_CALLS, INCREASE_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let size_delta = safely_pop_arg!(args, u64);
    let unique_priority_idx = safely_pop_arg!(args, u128);
    let price = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let mut active = ctx.active.borrow_mut();
    let overlay = active.get_mut(&market_addr).expect("overlay not acquired");
    overlay.increase_order_size(price, unique_priority_idx, size_delta, is_bid)
        .map_err(SafeNativeError::abort)?;
    Ok(smallvec![])
})
}

fn native_decrease_order_size(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(DECREASE_CALLS, DECREASE_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let size_delta = safely_pop_arg!(args, u64);
    let unique_priority_idx = safely_pop_arg!(args, u128);
    let price = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let mut active = ctx.active.borrow_mut();
    let overlay = active.get_mut(&market_addr).expect("overlay not acquired");
    overlay.decrease_order_size(price, unique_priority_idx, size_delta, is_bid)
        .map_err(SafeNativeError::abort)?;
    Ok(smallvec![])
})
}

// ===========================================================================================
// Native Functions — OrderBook module (lifecycle natives)

/// Fast check: is the overlay already acquired for this market in this TX?
/// Avoids the expensive borrow_global<OrderBookVersion> on repeated calls.
fn native_is_acquired(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(IS_ACQUIRED_CALLS, IS_ACQUIRED_NANOS, {
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let active = ctx.active.borrow();
    Ok(smallvec![Value::bool(active.contains_key(&market_addr))])
})
}

/// Called from `OrderBook.ensure_native_index_ready`. Acquires the overlay for a market.
/// Returns true if cold-start rebuild is needed.
fn native_ensure_acquired(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(ENSURE_ACQUIRED_CALLS, ENSURE_ACQUIRED_NANOS, {
    let handle = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let needs_rebuild = ctx.ensure_acquired(market_addr, handle);
    Ok(smallvec![Value::bool(needs_rebuild)])
})
}

/// Called from `OrderBook.maybe_flush_handle`. Marks the overlay for finalization.
/// Returns true if the market was modified (handle should be bumped).
fn native_flush(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(FLUSH_CALLS, FLUSH_NANOS, {
    let new_handle = safely_pop_arg!(args, u64);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let modified = ctx.flush(market_addr, new_handle);
    Ok(smallvec![Value::bool(modified)])
})
}

/// Add one order during cold-start rebuild. Called per active order.
fn native_rebuild_add(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(REBUILD_ADD_CALLS, REBUILD_ADD_NANOS, {
    let is_bid = safely_pop_arg!(args, bool);
    let size = safely_pop_arg!(args, u64);
    let unique_priority_idx = safely_pop_arg!(args, u128);
    let price = safely_pop_arg!(args, u64);
    let order_type = safely_pop_arg!(args, u64) as u16;
    let order_id = safely_pop_arg!(args, u128);
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    let mut active = ctx.active.borrow_mut();
    let overlay = active.get_mut(&market_addr).expect("rebuild: overlay not acquired");
    let data = OrderData { order_id, order_type, size };
    if is_bid {
        let key = BuyKey { price, tie_breaker: MAX_U128 - unique_priority_idx };
        overlay.delta_buys.insert(key, Some(data));
    } else {
        let key = SellKey { price, tie_breaker: unique_priority_idx };
        overlay.delta_sells.insert(key, Some(data));
    }
    overlay.modified = true;
    Ok(smallvec![])
})
}

/// Signal that cold-start rebuild is complete for a market.
fn native_rebuild_complete(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    timed!(REBUILD_COMPLETE_CALLS, REBUILD_COMPLETE_NANOS, {
    let market_addr = safely_pop_arg!(args, AccountAddress);
    context.charge(BASE_GAS)?;

    let ctx = context.extensions().get::<NativeOrderBookContext>();
    ctx.needs_rebuild.borrow_mut().remove(&market_addr);
    Ok(smallvec![])
})
}

// ===========================================================================================
// V1 timing probes — called from Move code to measure BigOrderedMap operations

thread_local! {
    static TIMING_STACK: std::cell::RefCell<Vec<Instant>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Native batch validation of price/size arrays.
/// Replaces the Move interpreter loop in validate_array_of_price_and_size.
fn native_validate_prices_and_sizes(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let t0 = Instant::now();
    let sizes = safely_pop_arg!(args, Vec<u64>);
    let prices = safely_pop_arg!(args, Vec<u64>);
    let precision_multiplier = safely_pop_arg!(args, u128);
    let min_size = safely_pop_arg!(args, u64);
    let lot_size = safely_pop_arg!(args, u64);
    let ticker_size = safely_pop_arg!(args, u64);
    context.charge(BASE_GAS)?;
    let t1 = Instant::now();
    let arg_pop_ns = (t1 - t0).as_nanos() as u64;

    eprintln!("[VALIDATE-NATIVE] CALLED! prices={} sizes={}", prices.len(), sizes.len());
    if prices.len() != sizes.len() {
        return Err(SafeNativeError::abort(11)); // EPRICE_SIZES_LENGTH_MISMATCH
    }
    let max_product = (i64::MAX as u128) * precision_multiplier;
    for i in 0..prices.len() {
        let price = prices[i];
        let size = sizes[i];
        if price == 0 {
            return Err(SafeNativeError::abort(8)); // EINVALID_PRICE
        }
        if price % ticker_size != 0 {
            return Err(SafeNativeError::abort(6)); // EPRICE_NOT_RESPECTING_TICKER_SIZE
        }
        if size == 0 {
            return Err(SafeNativeError::abort(9)); // EINVALID_SIZE
        }
        if size > i64::MAX as u64 {
            return Err(SafeNativeError::abort(10)); // EORDER_SIZE_TOO_LARGE
        }
        if size % lot_size != 0 {
            return Err(SafeNativeError::abort(5)); // ESIZE_NOT_RESPECTING_LOT_SIZE
        }
        if size < min_size {
            return Err(SafeNativeError::abort(4)); // ESIZE_NOT_RESPECTING_MIN_SIZE
        }
        if (price as u128) * (size as u128) > max_product {
            return Err(SafeNativeError::abort(10)); // EORDER_SIZE_TOO_LARGE
        }
    }
    let validate_ns = t1.elapsed().as_nanos() as u64;
    let total_ns = t0.elapsed().as_nanos() as u64;
    {
        use std::sync::atomic::{AtomicU64, Ordering as AO};
        static V_CALLS: AtomicU64 = AtomicU64::new(0);
        static V_ARG_POP: AtomicU64 = AtomicU64::new(0);
        static V_VALIDATE: AtomicU64 = AtomicU64::new(0);
        static V_TOTAL: AtomicU64 = AtomicU64::new(0);
        static V_ITEMS: AtomicU64 = AtomicU64::new(0);
        let c = V_CALLS.fetch_add(1, AO::Relaxed) + 1;
        V_ARG_POP.fetch_add(arg_pop_ns, AO::Relaxed);
        V_VALIDATE.fetch_add(validate_ns, AO::Relaxed);
        V_TOTAL.fetch_add(total_ns, AO::Relaxed);
        V_ITEMS.fetch_add(prices.len() as u64, AO::Relaxed);
        if c % 50000 == 0 {
            eprintln!("[VALIDATE-NATIVE] calls={} items={} total={:.1}ms avg={:.1}μs | arg_pop={:.1}μs validate={:.1}μs",
                c, V_ITEMS.load(AO::Relaxed),
                V_TOTAL.load(AO::Relaxed) as f64 / 1e6,
                V_TOTAL.load(AO::Relaxed) as f64 / c as f64 / 1e3,
                V_ARG_POP.load(AO::Relaxed) as f64 / c as f64 / 1e3,
                V_VALIDATE.load(AO::Relaxed) as f64 / c as f64 / 1e3);
        }
    }
    Ok(smallvec![Value::bool(true)])
}

/// Set timing context for per-txn-type attribution.
fn native_set_timing_context(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let ctx = safely_pop_arg!(args, u64) as usize;
    context.charge(BASE_GAS)?;
    if ctx < NUM_CONTEXTS {
        CURRENT_CTX.with(|c| c.set(ctx));
    }
    Ok(smallvec![])
}

/// Pushes a new timing start onto the stack. Returns a stack depth token.
fn native_timing_start(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    _args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    context.charge(BASE_GAS)?;
    // Close any pending gap measurement: this start marks the end of the gap.
    check_gap_on_next_start();
    let depth = TIMING_STACK.with(|s| {
        let mut stack = s.borrow_mut();
        let depth = stack.len() as u64;
        stack.push(Instant::now());
        depth
    });
    Ok(smallvec![Value::u64(depth)])
}

/// Pops the timing start from the stack and records elapsed time.
/// Takes &mut u64 as start_token to prevent the Move compiler from optimizing away the call.
fn native_timing_end(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    let start_token_ref = safely_pop_arg!(args, move_vm_types::values::Reference);
    let label = safely_pop_arg!(args, u64);
    context.charge(BASE_GAS)?;

    let elapsed = TIMING_STACK.with(|s| {
        let mut stack = s.borrow_mut();
        stack.pop()
            .map(|start| start.elapsed().as_nanos() as u64)
            .unwrap_or(0)
    });

    // Write elapsed time back to the mutable reference to create a side effect
    // that prevents the compiler from optimizing away the call.
    start_token_ref.write_ref(Value::u64(elapsed))
        .map_err(|_| SafeNativeError::abort(0xFF_0001))?;

    let idx = label as usize;

    // For post_settle gap: probe 49 (settle_trade end) has no following native_timing_start,
    // so we close the gap here.
    check_gap_on_settle_end(idx);

    if idx < NUM_PROBES {
        record_histogram(idx, elapsed);
        V1_PROBE_CALLS[idx].fetch_add(1, AtomicOrdering::Relaxed);
        V1_PROBE_NANOS[idx].fetch_add(elapsed, AtomicOrdering::Relaxed);

        // Also record into context-specific counters
        let ctx = CURRENT_CTX.with(|c| c.get());
        if ctx < NUM_CONTEXTS {
            CTX_PROBE_CALLS[ctx][idx].fetch_add(1, AtomicOrdering::Relaxed);
            CTX_PROBE_NANOS[ctx][idx].fetch_add(elapsed, AtomicOrdering::Relaxed);
        }
    }

    // Record timestamp for gap measurement after this probe ends
    record_gap_on_end(idx);

    Ok(smallvec![])
}

// ===========================================================================================
// Registration

pub fn order_book_natives(
    addr: AccountAddress,
    builder: &SafeNativeBuilder,
) -> NativeFunctionTable {
    // Operation natives — declared in price_time_index module
    let pti_natives: Vec<(&str, RawSafeNative)> = vec![
        ("native_best_bid_price", native_best_bid_price),
        ("native_best_ask_price", native_best_ask_price),
        ("native_get_mid_price", native_get_mid_price),
        ("native_get_slippage_price", native_get_slippage_price),
        ("native_is_taker_order", native_is_taker_order),
        ("native_place_maker_order", native_place_maker_order),
        ("native_cancel_active_order", native_cancel_active_order),
        (
            "native_get_single_match_result",
            native_get_single_match_result,
        ),
        (
            "native_increase_order_size",
            native_increase_order_size,
        ),
        (
            "native_decrease_order_size",
            native_decrease_order_size,
        ),
        ("native_timing_start", native_timing_start),
        ("native_timing_end", native_timing_end),
        ("native_set_timing_context", native_set_timing_context),
        (
            "native_validate_prices_and_sizes",
            native_validate_prices_and_sizes,
        ),
    ];

    // Lifecycle natives — declared in order_book module
    let ob_natives: Vec<(&str, RawSafeNative)> = vec![
        ("native_is_acquired", native_is_acquired),
        ("native_ensure_acquired", native_ensure_acquired),
        ("native_flush", native_flush),
        ("native_rebuild_add", native_rebuild_add),
        ("native_rebuild_complete", native_rebuild_complete),
    ];

    // Rebuild natives — declared in single_order_book and bulk_order_book modules
    let sob_natives: Vec<(&str, RawSafeNative)> = vec![
        ("native_rebuild_add", native_rebuild_add),
    ];
    let bob_natives: Vec<(&str, RawSafeNative)> = vec![
        ("native_rebuild_add", native_rebuild_add),
    ];

    let pti_module = Identifier::new("price_time_index").unwrap();
    let ob_module = Identifier::new("order_book").unwrap();
    let sob_module = Identifier::new("single_order_book").unwrap();
    let bob_module = Identifier::new("bulk_order_book").unwrap();

    let pti = builder.make_named_natives(pti_natives).map(|(name, func)| {
        (addr, pti_module.clone(), Identifier::new(name).unwrap(), func)
    });
    let ob = builder.make_named_natives(ob_natives).map(|(name, func)| {
        (addr, ob_module.clone(), Identifier::new(name).unwrap(), func)
    });
    let sob = builder.make_named_natives(sob_natives).map(|(name, func)| {
        (addr, sob_module.clone(), Identifier::new(name).unwrap(), func)
    });
    let bob = builder.make_named_natives(bob_natives).map(|(name, func)| {
        (addr, bob_module.clone(), Identifier::new(name).unwrap(), func)
    });

    pti.chain(ob).chain(sob).chain(bob).collect()
}

// ===========================================================================================
// Tests

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_overlay() -> OverlayIndex {
        OverlayIndex::new(PriceTimeParent::Base(Arc::new(PriceTimeBase::empty())))
    }

    fn dashmap_from<K: Eq + std::hash::Hash, V>(entries: Vec<(K, V)>) -> DashMap<K, V> {
        let m = DashMap::new();
        for (k, v) in entries {
            m.insert(k, v);
        }
        m
    }

    fn base_with_orders(
        buys: Vec<(u64, u128, u128, u16, u64)>,  // (price, idx, order_id, order_type, size)
        sells: Vec<(u64, u128, u128, u16, u64)>,
    ) -> Arc<PriceTimeBase> {
        let mut buy_map = BTreeMap::new();
        for (price, idx, order_id, order_type, size) in buys {
            buy_map.insert(
                BuyKey { price, tie_breaker: MAX_U128 - idx },
                OrderData { order_id, order_type, size },
            );
        }
        let mut sell_map = BTreeMap::new();
        for (price, idx, order_id, order_type, size) in sells {
            sell_map.insert(
                SellKey { price, tie_breaker: idx },
                OrderData { order_id, order_type, size },
            );
        }
        Arc::new(PriceTimeBase { buys: buy_map, sells: sell_map })
    }

    #[test]
    fn test_empty_overlay() {
        let overlay = empty_overlay();
        assert!(overlay.best_bid_price().is_none());
        assert!(overlay.best_ask_price().is_none());
        assert!(overlay.get_mid_price().is_none());
        assert!(!overlay.is_taker_order(100, true));
        assert!(!overlay.is_taker_order(100, false));
    }

    #[test]
    fn test_place_and_query() {
        let mut overlay = empty_overlay();

        // Place a sell at 200
        overlay.place_maker_order(1, 0, 200, 0, 1000, false).unwrap();
        assert_eq!(overlay.best_ask_price(), Some(200));
        assert!(overlay.best_bid_price().is_none());

        // Place a buy at 100
        overlay.place_maker_order(2, 0, 100, 1, 500, true).unwrap();
        assert_eq!(overlay.best_bid_price(), Some(100));
        assert_eq!(overlay.get_mid_price(), Some(150));

        // is_taker checks
        assert!(overlay.is_taker_order(200, true));   // bid at ask price
        assert!(!overlay.is_taker_order(199, true));   // bid below ask
        assert!(overlay.is_taker_order(100, false));   // ask at bid price
        assert!(!overlay.is_taker_order(101, false));   // ask above bid
    }

    #[test]
    fn test_cancel_order() {
        let mut overlay = empty_overlay();
        overlay.place_maker_order(1, 0, 200, 0, 1000, false).unwrap();
        overlay.place_maker_order(2, 0, 150, 1, 500, false).unwrap();
        assert_eq!(overlay.best_ask_price(), Some(150));

        let size = overlay.cancel_active_order(150, 1, false).unwrap();
        assert_eq!(size, 500);
        assert_eq!(overlay.best_ask_price(), Some(200));
    }

    #[test]
    fn test_match_single() {
        let mut overlay = empty_overlay();
        overlay.place_maker_order(1, 0, 200, 0, 1000, false).unwrap();
        overlay.place_maker_order(2, 0, 100, 1, 500, true).unwrap();

        // Taker buy at 200, size 300 — matches best ask
        let (order_id, matched, remaining, otype) =
            overlay.get_single_match_result(200, 300, true);
        assert_eq!(order_id, 1);
        assert_eq!(matched, 300);
        assert_eq!(remaining, 700);
        assert_eq!(otype, 0);

        // Ask at 200 still has 700 remaining
        assert_eq!(overlay.best_ask_price(), Some(200));

        // Full consume
        let (_, matched, remaining, _) = overlay.get_single_match_result(200, 700, true);
        assert_eq!(matched, 700);
        assert_eq!(remaining, 0);
        assert!(overlay.best_ask_price().is_none()); // all sells consumed
    }

    #[test]
    fn test_overlay_on_base() {
        let base = base_with_orders(
            vec![(100, 0, 1, 0, 500), (200, 1, 2, 0, 1000)],
            vec![(300, 2, 3, 0, 750)],
        );
        let overlay = OverlayIndex::new(PriceTimeParent::Base(base));

        assert_eq!(overlay.best_bid_price(), Some(200));
        assert_eq!(overlay.best_ask_price(), Some(300));
    }

    #[test]
    fn test_tombstone_from_base() {
        let base = base_with_orders(
            vec![(100, 0, 1, 0, 500), (200, 1, 2, 0, 1000)],
            vec![],
        );
        let mut overlay = OverlayIndex::new(PriceTimeParent::Base(base));

        // Cancel the best bid (200)
        let size = overlay.cancel_active_order(200, 1, true).unwrap();
        assert_eq!(size, 1000);

        // Best bid falls back to 100
        assert_eq!(overlay.best_bid_price(), Some(100));
    }

    #[test]
    fn test_layer_chain() {
        // Base: buy at 100
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);

        // Layer 1: adds buy at 200
        let layer1 = Arc::new(PriceTimeLayer {
            delta_buys: {
                let mut m = BTreeMap::new();
                m.insert(
                    BuyKey { price: 200, tie_breaker: MAX_U128 - 1 },
                    Some(OrderData { order_id: 2, order_type: 0, size: 300 }),
                );
                m
            },
            delta_sells: BTreeMap::new(),
            parent: PriceTimeParent::Base(base),
        });

        // Layer 2: tombstones buy at 200
        let layer2 = Arc::new(PriceTimeLayer {
            delta_buys: {
                let mut m = BTreeMap::new();
                m.insert(BuyKey { price: 200, tie_breaker: MAX_U128 - 1 }, None);
                m
            },
            delta_sells: BTreeMap::new(),
            parent: PriceTimeParent::Layer(layer1),
        });

        let overlay = OverlayIndex::new(PriceTimeParent::Layer(layer2));

        // 200 is tombstoned, so best bid is 100 from base
        assert_eq!(overlay.best_bid_price(), Some(100));
    }

    #[test]
    fn test_overlay_upsert_overrides_base() {
        let base = base_with_orders(vec![], vec![(100, 0, 1, 0, 1000)]);
        let mut overlay = OverlayIndex::new(PriceTimeParent::Base(base));

        // Match 300 against the sell at 100
        let (id, matched, remaining, _) = overlay.get_single_match_result(100, 300, true);
        assert_eq!(id, 1);
        assert_eq!(matched, 300);
        assert_eq!(remaining, 700);

        // The overlay now has an upsert for sell at 100 with size 700
        // Verify by querying through find_best_sell
        let (key, data) = find_best_sell(&overlay).unwrap();
        assert_eq!(key.price, 100);
        assert_eq!(data.size, 700);
    }

    #[test]
    fn test_increase_decrease_size() {
        let mut overlay = empty_overlay();
        overlay.place_maker_order(1, 0, 100, 0, 500, true).unwrap();

        overlay.increase_order_size(100, 0, 200, true).unwrap();
        let (_, data) = find_best_buy(&overlay).unwrap();
        assert_eq!(data.size, 700);

        overlay.decrease_order_size(100, 0, 300, true).unwrap();
        let (_, data) = find_best_buy(&overlay).unwrap();
        assert_eq!(data.size, 400);
    }

    #[test]
    fn test_slippage_price() {
        let mut overlay = empty_overlay();
        overlay.place_maker_order(1, 0, 101, 0, 100, false).unwrap();
        overlay.place_maker_order(2, 0, 99, 1, 100, true).unwrap();

        // mid = (99 + 101) / 2 = 100
        assert_eq!(overlay.get_mid_price(), Some(100));

        // 10% slippage bid = 100 + 10 = 110
        assert_eq!(overlay.get_slippage_price(true, 1000).unwrap(), Some(110));
        // 1% slippage bid = 100 + 1 = 101
        assert_eq!(overlay.get_slippage_price(true, 100).unwrap(), Some(101));
        // 15% slippage sell = 100 - 15 = 85
        assert_eq!(overlay.get_slippage_price(false, 1500).unwrap(), Some(85));
    }

    #[test]
    fn test_flatten_layer_chain() {
        let base = base_with_orders(
            vec![(100, 0, 1, 0, 500)],
            vec![(200, 0, 2, 0, 1000)],
        );

        // Layer: cancel buy at 100, add buy at 150, partial match sell at 200 → 700
        let mut delta_buys = BTreeMap::new();
        delta_buys.insert(BuyKey { price: 100, tie_breaker: MAX_U128 }, None);
        delta_buys.insert(
            BuyKey { price: 150, tie_breaker: MAX_U128 - 1 },
            Some(OrderData { order_id: 3, order_type: 0, size: 400 }),
        );
        let mut delta_sells = BTreeMap::new();
        delta_sells.insert(
            SellKey { price: 200, tie_breaker: 0 },
            Some(OrderData { order_id: 2, order_type: 0, size: 700 }),
        );

        let layer = PriceTimeLayer {
            delta_buys,
            delta_sells,
            parent: PriceTimeParent::Base(base),
        };

        let new_base = PriceTimeBase::flatten_from(&layer);

        // Buy at 100 was cancelled, buy at 150 was added
        assert_eq!(new_base.buys.len(), 1);
        let (k, v) = new_base.buys.iter().next().unwrap();
        assert_eq!(k.price, 150);
        assert_eq!(v.size, 400);

        // Sell at 200 was reduced to 700
        assert_eq!(new_base.sells.len(), 1);
        let (k, v) = new_base.sells.iter().next().unwrap();
        assert_eq!(k.price, 200);
        assert_eq!(v.size, 700);
    }

    #[test]
    fn test_block_state_finalize() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);

        let mut block_state = BlockNativeState {
            bases: dashmap_from(vec![(addr, base.clone())]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        };

        // Simulate TX0: adds buy at 200, handle becomes 6
        let layer = Arc::new(PriceTimeLayer {
            delta_buys: {
                let mut m = BTreeMap::new();
                m.insert(
                    BuyKey { price: 200, tie_breaker: MAX_U128 - 1 },
                    Some(OrderData { order_id: 2, order_type: 0, size: 300 }),
                );
                m
            },
            delta_sells: BTreeMap::new(),
            parent: PriceTimeParent::Base(base),
        });
        block_state.layers.insert((addr, 6), layer);

        block_state.finalize_block();

        // After finalization, base should have both orders
        let new_base = block_state.bases.get(&addr).unwrap();
        assert_eq!(new_base.buys.len(), 2);
        assert_eq!(*block_state.start_handles.get(&addr).unwrap(), 6);
        assert!(block_state.layers.is_empty());
    }

    #[test]
    fn test_ensure_acquired_cold_start() {
        let block_state = Arc::new(BlockNativeState::new());
        let ctx = NativeOrderBookContext::new_with_block_state(block_state);
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();

        let needs_rebuild = ctx.ensure_acquired(addr, 0);
        assert!(needs_rebuild);
        assert!(ctx.needs_rebuild.borrow().contains(&addr));
        assert!(ctx.active.borrow().contains_key(&addr));
    }

    #[test]
    fn test_ensure_acquired_warm_start() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);

        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        let ctx = NativeOrderBookContext::new_with_block_state(block_state);
        let needs_rebuild = ctx.ensure_acquired(addr, 5);
        assert!(!needs_rebuild);

        let active = ctx.active.borrow();
        let overlay = active.get(&addr).unwrap();
        assert_eq!(overlay.best_bid_price(), Some(100));
    }

    #[test]
    fn test_flush_and_finalize() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![], vec![]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
        ctx.ensure_acquired(addr, 5);

        // Modify overlay
        {
            let mut active = ctx.active.borrow_mut();
            let overlay = active.get_mut(&addr).unwrap();
            overlay.place_maker_order(1, 0, 100, 0, 500, true).unwrap();
        }

        // Flush
        let modified = ctx.flush(addr, 6);
        assert!(modified);

        // Finalize → layer stored in block state
        ctx.finalize();
        assert!(block_state.layers.contains_key(&(addr, 6)));

        // Verify the layer has the order
        let layer = block_state.layers.get(&(addr, 6)).unwrap();
        let overlay = OverlayIndex::new(PriceTimeParent::Layer(layer.value().clone()));
        assert_eq!(overlay.best_bid_price(), Some(100));
    }

    #[test]
    fn test_read_only_no_flush() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
        ctx.ensure_acquired(addr, 5);

        // Read only — no modification
        {
            let active = ctx.active.borrow();
            let overlay = active.get(&addr).unwrap();
            assert_eq!(overlay.best_bid_price(), Some(100));
        }

        // Flush should return false (not modified)
        let modified = ctx.flush(addr, 6);
        assert!(!modified);
    }

    // ============================= Phase 4: Block-STM Simulation Tests ====================================

    /// Simulate two sequential TXs in the same block sharing BlockNativeState.
    /// TX0 places an order → flush → finalize → TX1 sees TX0's layer.
    #[test]
    fn test_two_tx_sequential_layer_sharing() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![], vec![]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        // TX0: place buy at 100, flush with handle 6, finalize
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 5);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .place_maker_order(1, 0, 100, 0, 500, true).unwrap();
            }
            assert!(ctx.flush(addr, 6));
            ctx.finalize();
        }

        // TX1: reads handle=6 (TX0's write), sees TX0's layer
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            let needs_rebuild = ctx.ensure_acquired(addr, 6); // handle 6 != start 5 → layer lookup
            assert!(!needs_rebuild);

            let active = ctx.active.borrow();
            let overlay = active.get(&addr).unwrap();
            assert_eq!(overlay.best_bid_price(), Some(100)); // sees TX0's order
        }
    }

    /// Simulate TX1 placing an order on top of TX0's layer → two-deep chain.
    #[test]
    fn test_three_tx_chain() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![], vec![(200, 0, 10, 0, 1000)]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        // TX0: place buy at 100
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 5);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .place_maker_order(1, 0, 100, 0, 500, true).unwrap();
            }
            ctx.flush(addr, 6);
            ctx.finalize();
        }

        // TX1: place buy at 150 (on top of TX0's layer)
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 6);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .place_maker_order(2, 0, 150, 1, 300, true).unwrap();
            }
            ctx.flush(addr, 7);
            ctx.finalize();
        }

        // TX2: reads handle=7, should see both TX0 and TX1 orders
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 7);

            let active = ctx.active.borrow();
            let overlay = active.get(&addr).unwrap();
            assert_eq!(overlay.best_bid_price(), Some(150)); // TX1's order is best
            assert_eq!(overlay.best_ask_price(), Some(200)); // base order
        }
    }

    /// Simulate abort: TX executes but doesn't flush/finalize → no layer stored.
    #[test]
    fn test_abort_no_layer_stored() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        // TX aborts: acquires, modifies, but never flushes/finalizes
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 5);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .place_maker_order(2, 0, 200, 1, 300, true).unwrap();
            }
            // No flush, no finalize — simulating abort
            // Context dropped
        }

        // No layer at handle 6
        assert!(!block_state.layers.contains_key(&(addr, 6)));

        // Next TX still sees the original base
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 5);
            let active = ctx.active.borrow();
            let overlay = active.get(&addr).unwrap();
            assert_eq!(overlay.best_bid_price(), Some(100)); // only base order
        }
    }

    /// Two markets in the same block, independent overlays.
    #[test]
    fn test_two_markets_independent() {
        let addr1 = AccountAddress::from_hex_literal("0x1").unwrap();
        let addr2 = AccountAddress::from_hex_literal("0x2").unwrap();
        let base1 = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);
        let base2 = base_with_orders(vec![], vec![(200, 0, 2, 0, 1000)]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr1, base1), (addr2, base2)]),
            start_handles: dashmap_from(vec![(addr1, 5), (addr2, 10)]),
            layers: DashMap::new(),
        });

        // TX touches both markets
        let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
        ctx.ensure_acquired(addr1, 5);
        ctx.ensure_acquired(addr2, 10);

        {
            let mut active = ctx.active.borrow_mut();
            active.get_mut(&addr1).unwrap()
                .cancel_active_order(100, 0, true).unwrap(); // cancel from market 1
            active.get_mut(&addr2).unwrap()
                .place_maker_order(3, 0, 150, 0, 400, true).unwrap(); // add to market 2
        }

        // Flush both
        assert!(ctx.flush(addr1, 6));
        assert!(ctx.flush(addr2, 11));
        ctx.finalize();

        // Market 1: buy at 100 cancelled
        let layer1 = block_state.layers.get(&(addr1, 6)).unwrap();
        let o1 = OverlayIndex::new(PriceTimeParent::Layer(layer1.value().clone()));
        assert!(o1.best_bid_price().is_none());

        // Market 2: buy at 150 added
        let layer2 = block_state.layers.get(&(addr2, 11)).unwrap();
        let o2 = OverlayIndex::new(PriceTimeParent::Layer(layer2.value().clone()));
        assert_eq!(o2.best_bid_price(), Some(150));
        assert_eq!(o2.best_ask_price(), Some(200));
    }

    /// Fork isolation: two blocks from the same parent have independent state.
    #[test]
    fn test_fork_isolation() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);

        let parent = BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        };

        // Fork A
        let fork_a = Arc::new(BlockNativeState::from_parent(&parent));
        {
            let ctx = NativeOrderBookContext::new_with_block_state(fork_a.clone());
            ctx.ensure_acquired(addr, 5);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .place_maker_order(2, 0, 200, 1, 300, true).unwrap();
            }
            ctx.flush(addr, 6);
            ctx.finalize();
        }

        // Fork B (from same parent)
        let fork_b = Arc::new(BlockNativeState::from_parent(&parent));
        {
            let ctx = NativeOrderBookContext::new_with_block_state(fork_b.clone());
            ctx.ensure_acquired(addr, 5);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .cancel_active_order(100, 0, true).unwrap(); // cancel instead of add
            }
            ctx.flush(addr, 6);
            ctx.finalize();
        }

        // Fork A: has both orders
        assert!(fork_a.layers.contains_key(&(addr, 6)));
        let layer_a = fork_a.layers.get(&(addr, 6)).unwrap();
        let oa = OverlayIndex::new(PriceTimeParent::Layer(layer_a.value().clone()));
        assert_eq!(oa.best_bid_price(), Some(200));

        // Fork B: cancelled the only order
        assert!(fork_b.layers.contains_key(&(addr, 6)));
        let layer_b = fork_b.layers.get(&(addr, 6)).unwrap();
        let ob = OverlayIndex::new(PriceTimeParent::Layer(layer_b.value().clone()));
        assert!(ob.best_bid_price().is_none());
    }

    /// CommittedOrderBookState: promote block state, then create new block from committed.
    #[test]
    fn test_committed_state_lifecycle() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);
        let committed = CommittedOrderBookState::new();

        // Block 1: add order, finalize, commit
        let mut block1 = BlockNativeState {
            bases: dashmap_from(vec![(addr, base.clone())]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        };
        block1.layers.insert((addr, 6), Arc::new(PriceTimeLayer {
            delta_buys: {
                let mut m = BTreeMap::new();
                m.insert(
                    BuyKey { price: 200, tie_breaker: MAX_U128 - 1 },
                    Some(OrderData { order_id: 2, order_type: 0, size: 300 }),
                );
                m
            },
            delta_sells: BTreeMap::new(),
            parent: PriceTimeParent::Base(base),
        }));
        block1.finalize_block();
        committed.commit(&block1);

        // Block 2: created from committed state
        let block2 = BlockNativeState::from_committed(&committed);
        assert_eq!(*block2.start_handles.get(&addr).unwrap(), 6);
        assert_eq!(block2.bases.get(&addr).unwrap().buys.len(), 2); // both orders in flattened base

        // TX in block 2 sees the committed base
        let block2_arc = Arc::new(block2);
        let ctx = NativeOrderBookContext::new_with_block_state(block2_arc);
        ctx.ensure_acquired(addr, 6);
        let active = ctx.active.borrow();
        let overlay = active.get(&addr).unwrap();
        assert_eq!(overlay.best_bid_price(), Some(200)); // highest of 100, 200
    }

    /// Cold start rebuild simulation: empty block state, add orders via rebuild.
    #[test]
    fn test_cold_start_rebuild_via_context() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        let block_state = Arc::new(BlockNativeState::new());
        let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());

        // ensure_acquired on unknown market → cold start
        let needs_rebuild = ctx.ensure_acquired(addr, 0);
        assert!(needs_rebuild);

        // Simulate rebuild: add orders to the empty overlay
        {
            let mut active = ctx.active.borrow_mut();
            let overlay = active.get_mut(&addr).unwrap();

            // Add buy at 100
            let data1 = OrderData { order_id: 1, order_type: 0, size: 500 };
            overlay.delta_buys.insert(
                BuyKey { price: 100, tie_breaker: MAX_U128 },
                Some(data1),
            );
            // Add sell at 200
            let data2 = OrderData { order_id: 2, order_type: 0, size: 1000 };
            overlay.delta_sells.insert(
                SellKey { price: 200, tie_breaker: 0 },
                Some(data2),
            );
            overlay.modified = true;
        }

        // Clear needs_rebuild
        ctx.needs_rebuild.borrow_mut().remove(&addr);

        // Verify the overlay is usable
        let active = ctx.active.borrow();
        let overlay = active.get(&addr).unwrap();
        assert_eq!(overlay.best_bid_price(), Some(100));
        assert_eq!(overlay.best_ask_price(), Some(200));
        assert_eq!(overlay.get_mid_price(), Some(150));
        assert!(overlay.is_taker_order(200, true));
        assert!(!overlay.is_taker_order(199, true));
    }

    /// Match consuming multiple orders across layers.
    #[test]
    fn test_match_across_layers() {
        let addr = AccountAddress::from_hex_literal("0x1").unwrap();
        // Base has sell at 100 (size 500)
        let base = base_with_orders(vec![], vec![(100, 0, 1, 0, 500)]);
        let block_state = Arc::new(BlockNativeState {
            bases: dashmap_from(vec![(addr, base)]),
            start_handles: dashmap_from(vec![(addr, 5)]),
            layers: DashMap::new(),
        });

        // TX0: adds another sell at 100 (different idx, size 300)
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 5);
            {
                let mut active = ctx.active.borrow_mut();
                active.get_mut(&addr).unwrap()
                    .place_maker_order(2, 0, 100, 1, 300, false).unwrap();
            }
            ctx.flush(addr, 6);
            ctx.finalize();
        }

        // TX1: taker buy matches first sell (from base, idx=0), partially consumes it
        {
            let ctx = NativeOrderBookContext::new_with_block_state(block_state.clone());
            ctx.ensure_acquired(addr, 6);
            {
                let mut active = ctx.active.borrow_mut();
                let overlay = active.get_mut(&addr).unwrap();

                // First match: consumes order at (100, idx=0) from base — size 500, match 400
                let (id, matched, remaining, _) = overlay.get_single_match_result(100, 400, true);
                assert_eq!(id, 1);
                assert_eq!(matched, 400);
                assert_eq!(remaining, 100);

                // Second match: continues matching same price level, order at (100, idx=0) has 100 left
                let (id, matched, remaining, _) = overlay.get_single_match_result(100, 200, true);
                assert_eq!(id, 1); // same order, remaining 100
                assert_eq!(matched, 100);
                assert_eq!(remaining, 0);

                // Third match: now matches TX0's sell at (100, idx=1) — size 300
                let (id, matched, remaining, _) = overlay.get_single_match_result(100, 150, true);
                assert_eq!(id, 2); // TX0's order
                assert_eq!(matched, 150);
                assert_eq!(remaining, 150);
            }
        }
    }

    /// Ensure re-inserting a tombstoned key works correctly.
    #[test]
    fn test_tombstone_then_reinsert() {
        let base = base_with_orders(vec![(100, 0, 1, 0, 500)], vec![]);
        let mut overlay = OverlayIndex::new(PriceTimeParent::Base(base));

        // Cancel the order (tombstone)
        overlay.cancel_active_order(100, 0, true).unwrap();
        assert!(overlay.best_bid_price().is_none());

        // Re-insert at same price with different order
        overlay.place_maker_order(2, 0, 100, 1, 300, true).unwrap();
        assert_eq!(overlay.best_bid_price(), Some(100));

        // Verify it's the new order
        let (_, data) = find_best_buy(&overlay).unwrap();
        assert_eq!(data.order_id, 2);
        assert_eq!(data.size, 300);
    }
}
