// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This module provides common utilities for the DB pruner.

use crate::{
    pruner::{
        ledger_store::ledger_store_pruner::LedgerPruner,
        state_kv_pruner::StateKvPruner,
        state_store::{generics::StaleNodeIndexSchemaTrait, StateMerklePruner},
    },
    EventStore, TransactionStore,
};
use aptos_jellyfish_merkle::StaleNodeIndex;
use aptos_schemadb::{schema::KeyCodec, DB};
use std::sync::Arc;

/// A utility function to instantiate the state pruner
pub fn create_state_merkle_pruner<S: StaleNodeIndexSchemaTrait>(
    state_merkle_db: Arc<DB>,
) -> Arc<StateMerklePruner<S>>
where
    StaleNodeIndex: KeyCodec<S>,
{
    Arc::new(StateMerklePruner::<S>::new(Arc::clone(&state_merkle_db)))
}

/// A utility function to instantiate the ledger pruner
pub(crate) fn create_ledger_pruner(ledger_db: Arc<DB>) -> Arc<LedgerPruner> {
    Arc::new(LedgerPruner::new(
        Arc::clone(&ledger_db),
        Arc::new(TransactionStore::new(Arc::clone(&ledger_db))),
        Arc::new(EventStore::new(Arc::clone(&ledger_db))),
    ))
}

/// A utility function to instantiate the state kv pruner.
pub(crate) fn create_state_kv_pruner(state_kv_db: Arc<DB>) -> Arc<StateKvPruner> {
    Arc::new(StateKvPruner::new(state_kv_db))
}
