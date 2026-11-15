// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::state_store::{
    collateral::CollateralOverlay,
    orders::OrderOverlay,
    positions::PositionOverlay,
    sharded_jmt_state::NativeStateWithSummary,
    state::LedgerState,
    state_summary::LedgerStateSummary,
    state_update_refs::StateUpdateRefs,
    state_view::cached_state_view::ShardedStateCache,
    state_with_summary::{LedgerStateWithSummary, LedgerWithSummary, StateWithSummary},
    HotStateUpdates,
};
use aptos_types::transaction::{
    PersistedAuxiliaryInfo, Transaction, TransactionInfo, TransactionOutput, Version,
};

#[derive(Clone)]
pub struct ChunkToCommit<'a> {
    pub first_version: Version,
    pub transactions: &'a [Transaction],
    pub persisted_auxiliary_infos: &'a [PersistedAuxiliaryInfo],
    pub transaction_outputs: &'a [TransactionOutput],
    pub transaction_infos: &'a [TransactionInfo],
    pub state: &'a LedgerState,
    pub state_summary: &'a LedgerStateSummary,
    pub state_update_refs: &'a StateUpdateRefs<'a>,
    pub state_reads: &'a ShardedStateCache,
    pub hot_state_updates: &'a HotStateUpdates,
    /// Position summary computed at execution time (checkpoint stage), to
    /// be persisted/merklized at commit without recomputation. `None` when
    /// native position is disabled.
    pub position_state_summary: Option<&'a LedgerWithSummary<NativeStateWithSummary>>,
    /// Order summary computed at execution time, as `position_state_summary`
    /// is. `None` unless the trading-native state roots are computed.
    pub order_state_summary: Option<&'a LedgerWithSummary<NativeStateWithSummary>>,
    /// Collateral summary computed at execution time, as
    /// `position_state_summary` is. `None` unless the trading-native state
    /// roots are computed.
    pub collateral_state_summary: Option<&'a LedgerWithSummary<NativeStateWithSummary>>,
    /// Position writes since the base, computed in the execution phase
    /// — earlier than `position_state_summary`, which is a checkpoint-stage
    /// product. Commit publishes this as the bundle's tip so the
    /// validator-side scanner sees the latest committed view. `None` when
    /// native position is disabled.
    ///
    /// The chunk's latest only. The executor tracks a last-checkpoint
    /// overlay alongside it, but nothing outside the executor reads one.
    pub positions: Option<&'a PositionOverlay>,
    /// Order writes since the base, published by commit as the order tip
    /// as `positions` is. `None` when trading-native storage is off.
    pub orders: Option<&'a OrderOverlay>,
    /// Collateral writes since the base, published by commit as the
    /// collateral tip as `orders` is. `None` when trading-native storage is
    /// off.
    pub collateral: Option<&'a CollateralOverlay>,
    pub is_reconfig: bool,
}

impl ChunkToCommit<'_> {
    pub fn len(&self) -> usize {
        self.transactions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn next_version(&self) -> Version {
        self.first_version + self.len() as Version
    }

    pub fn expect_last_version(&self) -> Version {
        self.next_version() - 1
    }

    pub fn result_ledger_state_with_summary(&self) -> LedgerStateWithSummary {
        LedgerStateWithSummary::new_at_checkpoint(StateWithSummary::new(
            self.state.last_checkpoint().clone(),
            self.state_summary.last_checkpoint().clone(),
        ))
    }

    pub fn estimated_total_state_updates(&self) -> usize {
        self.state_update_refs
            .for_last_checkpoint_batched()
            .map_or(0, |x| x.len())
    }
}
