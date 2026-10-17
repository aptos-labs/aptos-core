// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![forbid(unsafe_code)]

use crate::{
    common::PipelineStateStore,
    ledger_db::LedgerDb,
    native_store::{
        buffered_state::{
            NativeBufferedState, NativeLedgerStateWithSummary, NativePersistedState,
            NativeStateWithSummary, NATIVE_TARGET_ITEMS,
        },
        merkle_db::NativeMerkleDb,
        pruner::NativeStorePruner,
        NativeStoreSpec,
    },
};
use aptos_infallible::Mutex;
use std::sync::Arc;

pub(crate) type NativeStateStore =
    PipelineStateStore<NativeLedgerStateWithSummary, NativeBufferedState>;
pub(crate) type PositionStateStore = NativeStateStore;

impl NativeStateStore {
    pub fn new_at_snapshot<T: NativeStoreSpec>(
        merkle_db: Arc<NativeMerkleDb<T>>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: NativeStateWithSummary,
        pruner: Arc<NativeStorePruner<T>>,
        persisted: NativePersistedState,
    ) -> Self {
        let current_state = Arc::new(Mutex::new(NativeLedgerStateWithSummary::new_at_checkpoint(
            last_snapshot.clone(),
        )));
        let buffered_state = NativeBufferedState::new_at_snapshot(
            merkle_db,
            ledger_db,
            last_snapshot,
            NATIVE_TARGET_ITEMS,
            Arc::clone(&current_state),
            pruner,
            persisted,
        );
        Self::from_parts(current_state, buffered_state)
    }

    /// Re-seat the pipeline onto `last_snapshot`, the counterpart of main
    /// state's `StateStore::reset`, after a fast-sync restore moves the
    /// durable store to the target version. Drains and stops the old commit
    /// thread first, so its final `sync_commit` uses the family it was
    /// built on.
    pub fn reset_at_snapshot<T: NativeStoreSpec>(
        &self,
        merkle_db: Arc<NativeMerkleDb<T>>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: NativeStateWithSummary,
        pruner: Arc<NativeStorePruner<T>>,
        persisted: NativePersistedState,
    ) {
        self.buffered_state_locked().quit();
        persisted.set(last_snapshot.clone());
        *self.buffered_state_locked() = NativeBufferedState::new_at_snapshot(
            merkle_db,
            ledger_db,
            last_snapshot,
            NATIVE_TARGET_ITEMS,
            self.current_state(),
            pruner,
            persisted,
        );
    }
}
