// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Session extension owning the per-TX staging map for position writes.
//! On session finalize the staged entries become `WriteOp`s in the
//! `position_write_set` bucket of `VMChangeSet`. Reads are not staged
//! here (no Move-side read path yet).

use aptos_types::state_store::{
    native_position::NativePosition, state_key::inner::TradingNativeKey,
};
use better_any::{Tid, TidAble};
use move_vm_runtime::native_extensions::{
    NativeRuntimeRefCheckModelsCompleted, UnreachableSessionListener,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

/// Bench-only counters of *staged* writes. Process-global, never reset,
/// and incremented inside the session, so Block-STM re-execution
/// over-counts vs committed writes — a coarse signal, not an exact tally.
pub static POSITION_SETS_STAGED: AtomicU64 = AtomicU64::new(0);
pub static POSITION_DELETES_STAGED: AtomicU64 = AtomicU64::new(0);

pub fn total_positions_staged() -> u64 {
    POSITION_SETS_STAGED.load(Ordering::Relaxed) + POSITION_DELETES_STAGED.load(Ordering::Relaxed)
}

/// Per-TX staged writes keyed by `TradingNativeKey`; `None` is a deletion.
#[derive(Default)]
pub struct PositionTxCache {
    pub staged: BTreeMap<TradingNativeKey, Option<NativePosition>>,
}

#[derive(Tid, Default)]
pub struct NativePositionContext {
    cache: RefCell<PositionTxCache>,
}

impl NativePositionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage_set(&self, key: TradingNativeKey, value: NativePosition) {
        POSITION_SETS_STAGED.fetch_add(1, Ordering::Relaxed);
        self.cache.borrow_mut().staged.insert(key, Some(value));
    }

    pub fn stage_delete(&self, key: TradingNativeKey) {
        POSITION_DELETES_STAGED.fetch_add(1, Ordering::Relaxed);
        self.cache.borrow_mut().staged.insert(key, None);
    }

    /// Drain the staged writes for routing into the VMChangeSet bucket.
    pub fn into_change_maps(self) -> BTreeMap<TradingNativeKey, Option<NativePosition>> {
        self.cache.into_inner().staged
    }
}

impl UnreachableSessionListener for NativePositionContext {}

impl NativeRuntimeRefCheckModelsCompleted for NativePositionContext {
    // The position natives are value-only: no references escape. So the
    // default (empty) runtime ref-check model is sufficient.
}
