// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_crypto::HashValue;
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::state_store::{
    sharded_jmt_state::NativeStateWithSummary, state_summary::LedgerStateSummary,
    state_with_summary::LedgerWithSummary,
};
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Deref)]
pub struct StateCheckpointOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

#[bon::bon]
impl StateCheckpointOutput {
    #[builder(finish_fn = build)]
    pub fn new(
        state_summary: LedgerStateSummary,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        hot_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
        position_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
        position_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
        order_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
        order_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
        collateral_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
        collateral_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    ) -> Self {
        Self::new_impl(Inner {
            state_summary,
            state_checkpoint_hashes,
            hot_state_checkpoint_hashes,
            position_state_summary,
            position_state_checkpoint_hashes,
            order_state_summary,
            order_state_checkpoint_hashes,
            collateral_state_summary,
            collateral_state_checkpoint_hashes,
        })
    }

    pub fn new_empty(
        parent_state_summary: LedgerStateSummary,
        position_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
        order_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
        collateral_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    ) -> Self {
        Self::new_impl(Inner {
            state_summary: parent_state_summary,
            state_checkpoint_hashes: vec![],
            hot_state_checkpoint_hashes: None,
            position_state_summary,
            position_state_checkpoint_hashes: None,
            order_state_summary,
            order_state_checkpoint_hashes: None,
            collateral_state_summary,
            collateral_state_checkpoint_hashes: None,
        })
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(LedgerStateSummary::new_empty(), None, None, None)
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        // No native writes in a reconfig suffix, so the summaries carry
        // over as the next block's freeze bases.
        Self::new_empty(
            self.state_summary.clone(),
            self.position_state_summary.clone(),
            self.order_state_summary.clone(),
            self.collateral_state_summary.clone(),
        )
    }
}

#[derive(Debug)]
pub struct Inner {
    pub state_summary: LedgerStateSummary,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
    /// Per-transaction hot state root, laid out like `state_checkpoint_hashes`.
    /// `None` (the whole option) unless the hot-state-root-in-txn-info feature is on.
    pub hot_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    /// Native-position summary after this chunk (latest + last_checkpoint),
    /// computed at execution time, persisted at commit without recompute.
    /// `None` unless the position-state-root feature is on.
    pub position_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    /// Per-transaction position state root: `Some` at the checkpoint index,
    /// `None` elsewhere. `None` (the whole option) unless the feature is on.
    pub position_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    /// Native-order summary after this chunk, as `position_state_summary`
    /// is. `None` unless the trading-native state roots are computed.
    pub order_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    /// Per-transaction order state root, laid out like
    /// `position_state_checkpoint_hashes`.
    pub order_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    /// Native-collateral summary after this chunk, as
    /// `position_state_summary` is. `None` unless the trading-native state
    /// roots are computed.
    pub collateral_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    /// Per-transaction collateral state root, laid out like
    /// `position_state_checkpoint_hashes`.
    pub collateral_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
}
