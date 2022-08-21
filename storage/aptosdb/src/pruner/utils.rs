// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides common utilities for the DB pruner.

use crate::{
    pruner::{ledger_store::ledger_store_pruner::LedgerPruner, state_store::StateMerklePruner},
    EventStore, StateStore, TransactionStore,
};

use crate::pruner::state_store::generics::StaleNodeIndexSchemaTrait;
use aptos_jellyfish_merkle::StaleNodeIndex;
use schemadb::schema::KeyCodec;
use schemadb::DB;
use std::sync::Arc;

/// A utility function to instantiate the state pruner
pub fn create_state_pruner<S: StaleNodeIndexSchemaTrait>(
    state_merkle_db: Arc<DB>,
) -> Arc<StateMerklePruner<S>>
where
    StaleNodeIndex: KeyCodec<S>,
{
    Arc::new(StateMerklePruner::<S>::new(Arc::clone(&state_merkle_db)))
}

/// A utility function to instantiate the ledger pruner
pub(crate) fn create_ledger_pruner(
    ledger_db: Arc<DB>,
    state_store: Arc<StateStore>,
) -> Arc<LedgerPruner> {
    Arc::new(LedgerPruner::new(
        Arc::clone(&ledger_db),
        Arc::new(TransactionStore::new(Arc::clone(&ledger_db))),
        Arc::new(EventStore::new(Arc::clone(&ledger_db))),
        state_store,
    ))
}
