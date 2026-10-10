// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::PipelineStateStore,
    ledger_db::LedgerDb,
    position_buffered_state::{
        PositionBufferedState, PositionLedgerStateWithSummary, PositionPersistedState,
        PositionStateWithSummary, POSITION_TARGET_ITEMS,
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
        persisted: PositionPersistedState,
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
            persisted,
        );
        Self::from_parts(current_state, buffered_state)
    }

    /// Re-seat the pipeline onto `last_snapshot`, the counterpart of main
    /// state's `StateStore::reset`, after a fast-sync restore moves the
    /// durable store to the target version. Drains and stops the old commit
    /// thread first, so its final `sync_commit` uses the family it was
    /// built on.
    pub fn reset_at_snapshot(
        &self,
        merkle_db: Arc<PositionMerkleDb>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: PositionStateWithSummary,
        position_pruner: Arc<PositionPruner>,
        persisted: PositionPersistedState,
    ) {
        self.buffered_state_locked().quit();
        persisted.set(last_snapshot.clone());
        *self.buffered_state_locked() = PositionBufferedState::new_at_snapshot(
            merkle_db,
            ledger_db,
            last_snapshot,
            POSITION_TARGET_ITEMS,
            self.current_state(),
            position_pruner,
            persisted,
        );
    }
}
