// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use aptos_config::config::HotStateConfig;
use aptos_crypto::HashValue;
use aptos_drop_helper::DropHelper;
use aptos_storage_interface::state_store::{
    sharded_jmt_state::PositionStateWithSummary, state_summary::LedgerStateSummary,
    state_with_summary::LedgerWithSummary, user_positions::UserPositions,
};
use derive_more::Deref;
use std::sync::Arc;

#[derive(Clone, Debug, Deref)]
pub struct StateCheckpointOutput {
    #[deref]
    inner: Arc<DropHelper<Inner>>,
}

impl StateCheckpointOutput {
    pub fn new(
        state_summary: LedgerStateSummary,
        state_checkpoint_hashes: Vec<Option<HashValue>>,
        hot_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
        position_state_summary: Option<LedgerWithSummary<PositionStateWithSummary>>,
        position_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
        user_positions: Option<LedgerWithSummary<UserPositions>>,
    ) -> Self {
        Self::new_impl(Inner {
            state_summary,
            state_checkpoint_hashes,
            hot_state_checkpoint_hashes,
            position_state_summary,
            position_state_checkpoint_hashes,
            user_positions,
        })
    }

    pub fn new_empty(
        parent_state_summary: LedgerStateSummary,
        position_state_summary: Option<LedgerWithSummary<PositionStateWithSummary>>,
        user_positions: Option<LedgerWithSummary<UserPositions>>,
    ) -> Self {
        Self::new_impl(Inner {
            state_summary: parent_state_summary,
            state_checkpoint_hashes: vec![],
            hot_state_checkpoint_hashes: None,
            position_state_summary,
            position_state_checkpoint_hashes: None,
            user_positions,
        })
    }

    pub fn new_dummy() -> Self {
        Self::new_empty(
            LedgerStateSummary::new_empty(HotStateConfig::default()),
            None,
            None,
        )
    }

    fn new_impl(inner: Inner) -> Self {
        Self {
            inner: Arc::new(DropHelper::new(inner)),
        }
    }

    pub fn reconfig_suffix(&self) -> Self {
        // An empty reconfig-suffix block produces no position writes, so
        // both `position_state_summary` and `user_positions` are unchanged
        // — propagate them for the next block's freeze base / read floor.
        Self::new_empty(
            self.state_summary.clone(),
            self.position_state_summary.clone(),
            self.user_positions.clone(),
        )
    }
}

#[derive(Debug)]
pub struct Inner {
    pub state_summary: LedgerStateSummary,
    pub state_checkpoint_hashes: Vec<Option<HashValue>>,
    // TODO(HotState): this is currently None in testnet and mainnet, since we don't run hot state
    // root hashes in consensus or state-sync yet.
    pub hot_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    /// Native-position summary after this chunk (latest + last_checkpoint),
    /// computed at execution time, persisted at commit without recompute.
    /// `None` unless the position-state-root feature is on.
    pub position_state_summary: Option<LedgerWithSummary<PositionStateWithSummary>>,
    /// Per-transaction position state root: `Some` at the checkpoint index,
    /// `None` elsewhere. `None` (the whole option) unless the feature is on.
    pub position_state_checkpoint_hashes: Option<Vec<Option<HashValue>>>,
    /// Per-account position index after this chunk, computed at execution
    /// time alongside `position_state_summary`. Commit publishes it onto
    /// `bundle.user_positions` for validator-side scanner reads. `None`
    /// when native position is disabled.
    pub user_positions: Option<LedgerWithSummary<UserPositions>>,
}
