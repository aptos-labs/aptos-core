// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::state_store::{
    collateral::CollateralOverlay, orders::OrderOverlay, positions::PositionOverlay,
    sharded_jmt_state::NativeStateWithSummary, state::LedgerState,
    state_summary::LedgerStateSummary, state_with_summary::LedgerWithSummary,
};
use aptos_config::config::HotStateConfig;
use aptos_types::{
    proof::accumulator::{InMemoryAccumulator, InMemoryTransactionAccumulator},
    transaction::Version,
};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct LedgerSummary {
    pub state: LedgerState,
    pub state_summary: LedgerStateSummary,
    pub transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    /// Pre-committed native-position summary; `None` when position is disabled.
    pub position_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    /// Pre-committed native-order summary; `None` when the trading-native
    /// state roots are not computed.
    pub order_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    /// Pre-committed native-collateral summary; `None` when the trading-native
    /// state roots are not computed.
    pub collateral_state_summary: Option<LedgerWithSummary<NativeStateWithSummary>>,
    /// Pre-committed position overlay; `None` when position is disabled.
    pub positions: Option<PositionOverlay>,
    /// Pre-committed order overlay; `None` when trading-native storage is off.
    pub orders: Option<OrderOverlay>,
    /// Pre-committed collateral overlay; `None` when trading-native storage
    /// is off.
    pub collateral: Option<CollateralOverlay>,
}

impl LedgerSummary {
    pub fn new(
        state: LedgerState,
        state_summary: LedgerStateSummary,
        transaction_accumulator: Arc<InMemoryTransactionAccumulator>,
    ) -> Self {
        state_summary.assert_versions_match(&state);

        Self {
            state,
            state_summary,
            transaction_accumulator,
            position_state_summary: None,
            order_state_summary: None,
            collateral_state_summary: None,
            positions: None,
            orders: None,
            collateral: None,
        }
    }

    pub fn next_version(&self) -> Version {
        self.transaction_accumulator.num_leaves()
    }

    pub fn version(&self) -> Option<Version> {
        self.next_version().checked_sub(1)
    }

    pub fn new_empty(hot_state_config: HotStateConfig) -> Self {
        let state = LedgerState::new_empty(hot_state_config);
        let state_summary = LedgerStateSummary::new_empty();
        Self::new(
            state,
            state_summary,
            Arc::new(InMemoryAccumulator::new_empty()),
        )
    }
}
