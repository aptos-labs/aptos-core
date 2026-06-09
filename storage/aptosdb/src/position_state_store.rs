// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::PipelineStateStore,
    ledger_db::LedgerDb,
    position_buffered_state::{
        PositionBufferedState, PositionLedgerStateWithSummary, PositionStateWithSummary,
        POSITION_TARGET_ITEMS,
    },
    position_merkle_db::PositionMerkleDb,
    position_pruner::PositionPruner,
};
use aptos_infallible::Mutex;
use std::sync::Arc;

pub(crate) type PositionStateStore =
    PipelineStateStore<PositionLedgerStateWithSummary, PositionBufferedState>;

impl PositionStateStore {
    pub fn new_at_snapshot(
        merkle_db: Arc<PositionMerkleDb>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: PositionStateWithSummary,
        position_pruner: Arc<PositionPruner>,
    ) -> Self {
        let current_state = Arc::new(Mutex::new(
            PositionLedgerStateWithSummary::new_at_checkpoint(last_snapshot.clone()),
        ));
        let buffered_state = PositionBufferedState::new_at_snapshot(
            merkle_db,
            ledger_db,
            last_snapshot,
            POSITION_TARGET_ITEMS,
            Arc::clone(&current_state),
            position_pruner,
        );
        Self::from_parts(current_state, buffered_state)
    }
}
