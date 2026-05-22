// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Owner / coordinator for the native-position async commit pipeline.
//! Type alias of the generic [`crate::common::PipelineStateStore`];
//! position-specific construction lives in the inherent impl below.
//!
//! Mirrors the role main-state's [`crate::state_store::StateStore`]
//! plays as the owner of `BufferedState`'s shared `current_state`.
//!
//! Constructed by `AptosDB::init_native_position`. Outside readers
//! query `position_state_store.current_state().lock()` for the
//! latest `position_root` without taking the heavier buffered-state
//! mutex.

#![forbid(unsafe_code)]

use crate::{
    common::PipelineStateStore,
    ledger_db::LedgerDb,
    position_buffered_state::{
        PositionBufferedState, PositionLedgerStateWithSummary, PositionStateWithSummary,
        POSITION_TARGET_ITEMS,
    },
    position_merkle_db::PositionMerkleDb,
};
use aptos_infallible::Mutex;
use std::sync::Arc;

/// Position's owner / coordinator — type alias of the generic
/// [`PipelineStateStore`] over position's `LedgerStateWithSummary` +
/// `BufferedState`. Pipeline-specific construction lives below.
///
/// `pub(crate)` because the alias resolves to a type that contains
/// `PositionBufferedState` (also `pub(crate)`) — keeping the alias's
/// visibility aligned with its underlying type.
pub(crate) type PositionStateStore =
    PipelineStateStore<PositionLedgerStateWithSummary, PositionBufferedState>;

impl PositionStateStore {
    /// Construct the store and the underlying `PositionBufferedState`.
    /// Mirrors `StateStore::new` for the buffered-state slice.
    pub fn new_at_snapshot(
        merkle_db: Arc<PositionMerkleDb>,
        ledger_db: Arc<LedgerDb>,
        last_snapshot: PositionStateWithSummary,
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
        );
        Self::from_parts(current_state, buffered_state)
    }
}
