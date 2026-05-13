// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Session-extension context for the native position subsystem.
//!
//! Owns the per-TX staging map that buffers position writes during
//! the session. On session finalize the staged entries are converted
//! into `WriteOp`s in the `position_write_set` bucket of `VMChangeSet`.
//!
//! Read-side plumbing (a `NativePositionResolver` wired into the data
//! view, a per-TX read cache, decode-into-`NativePosition` etc.) is
//! deferred to milestone 2 when Move-side reads from the in-memory
//! store become a supported surface.

use crate::position::{NativePosition, PositionKey};
use better_any::{Tid, TidAble};
use move_vm_runtime::native_extensions::{
    NativeRuntimeRefCheckModelsCompleted, UnreachableSessionListener,
};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    sync::atomic::{AtomicU64, Ordering},
};

/// Diagnostic counters: number of writes staged in the per-TX context
/// (broken out by kind so the bench harness can distinguish growth
/// from churn). Incremented inside the session — applied counts come
/// from `position_metrics::POSITION_WRITES` after commit.
pub static POSITION_CREATES_STAGED: AtomicU64 = AtomicU64::new(0);
pub static POSITION_UPDATES_STAGED: AtomicU64 = AtomicU64::new(0);
pub static POSITION_REMOVES_STAGED: AtomicU64 = AtomicU64::new(0);

/// Sum of creates + updates + removes; cheap aggregate for callers
/// that only need a total.
pub fn total_positions_staged() -> u64 {
    POSITION_CREATES_STAGED.load(Ordering::Relaxed)
        + POSITION_UPDATES_STAGED.load(Ordering::Relaxed)
        + POSITION_REMOVES_STAGED.load(Ordering::Relaxed)
}

/// State maintained for the duration of a single session: per-TX
/// staged writes (creates / updates / deletes) keyed by `PositionKey`.
/// `None` encodes a deletion.
#[derive(Default)]
pub struct PositionTxCache {
    pub staged: BTreeMap<PositionKey, Option<NativePosition>>,
}

/// Session extension passed into every native call in the position
/// subsystem.
///
/// Lifecycle (`register` / `deny` / `reenable`) is *not* tracked
/// here — it lives in the `ExchangeRegistry` Move resource at
/// `@aptos_framework`. The session context owns only the per-TX
/// staging map for writes.
#[derive(Tid, Default)]
pub struct NativePositionContext {
    cache: RefCell<PositionTxCache>,
}

impl NativePositionContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage_create(&self, key: PositionKey, value: NativePosition) {
        POSITION_CREATES_STAGED.fetch_add(1, Ordering::Relaxed);
        self.cache.borrow_mut().staged.insert(key, Some(value));
    }

    pub fn stage_update(&self, key: PositionKey, value: NativePosition) {
        POSITION_UPDATES_STAGED.fetch_add(1, Ordering::Relaxed);
        self.cache.borrow_mut().staged.insert(key, Some(value));
    }

    pub fn stage_remove(&self, key: PositionKey) {
        POSITION_REMOVES_STAGED.fetch_add(1, Ordering::Relaxed);
        self.cache.borrow_mut().staged.insert(key, None);
    }

    /// Consume the context and return position writes for routing by
    /// the session-finalize step. The map carries the staged entries
    /// that should land in the VMChangeSet position bucket.
    pub fn into_change_maps(self) -> BTreeMap<PositionKey, Option<NativePosition>> {
        self.cache.into_inner().staged
    }
}

impl UnreachableSessionListener for NativePositionContext {}

impl NativeRuntimeRefCheckModelsCompleted for NativePositionContext {
    // The position natives are value-only: no references escape. So the
    // default (empty) runtime ref-check model is sufficient.
}
