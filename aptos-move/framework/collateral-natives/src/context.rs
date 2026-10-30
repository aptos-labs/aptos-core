// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Session extension owning the per-transaction staging map for collateral
//! writes.

use aptos_types::state_store::state_key::inner::TradingNativeKey;
use better_any::{Tid, TidAble};
use bytes::Bytes;
use move_vm_runtime::native_extensions::{
    NativeRuntimeRefCheckModelsCompleted, UnreachableSessionListener,
};
use std::{cell::RefCell, collections::BTreeMap};

/// Staged collateral writes keyed by `TradingNativeKey`. A value is the BCS of
/// a `NativeCollateral`, whose primary balance may be a snapshot identifier;
/// `None` is a deletion.
#[derive(Tid, Default)]
pub struct NativeCollateralContext {
    staged: RefCell<BTreeMap<TradingNativeKey, Option<Bytes>>>,
}

impl NativeCollateralContext {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn stage_set(&self, key: TradingNativeKey, value: Bytes) {
        self.staged.borrow_mut().insert(key, Some(value));
    }

    pub fn stage_delete(&self, key: TradingNativeKey) {
        self.staged.borrow_mut().insert(key, None);
    }

    pub fn into_change_maps(self) -> BTreeMap<TradingNativeKey, Option<Bytes>> {
        self.staged.into_inner()
    }
}

impl UnreachableSessionListener for NativeCollateralContext {}

impl NativeRuntimeRefCheckModelsCompleted for NativeCollateralContext {
    // The collateral natives are value-only: no references escape.
}
