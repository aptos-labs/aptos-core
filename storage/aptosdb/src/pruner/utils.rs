// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides common utilities for the DB pruner.

use crate::{
    pruner::{
        db_pruner::DBPruner, ledger_store::ledger_store_pruner::LedgerPruner,
        state_store::StateStorePruner,
    },
    EventStore, LedgerStore, TransactionStore,
};
use aptos_config::config::StoragePrunerConfig;
use aptos_infallible::Mutex;
use schemadb::DB;
use std::{sync::Arc, time::Instant};

/// A useful utility function to instantiate all db pruners.
pub fn create_db_pruners(
    ledger_db: Arc<DB>,
    state_merkle_db: Arc<DB>,
    storage_pruner_config: StoragePrunerConfig,
) -> Vec<Option<Mutex<Arc<dyn DBPruner + Send + Sync>>>> {
    vec![
        if storage_pruner_config.state_store_prune_window.is_some() {
            Some(Mutex::new(Arc::new(StateStorePruner::new(
                Arc::clone(&state_merkle_db),
                0,
                Instant::now(),
            ))))
        } else {
            None
        },
        if storage_pruner_config.ledger_prune_window.is_some() {
            Some(Mutex::new(Arc::new(LedgerPruner::new(
                Arc::clone(&ledger_db),
                Arc::new(TransactionStore::new(Arc::clone(&ledger_db))),
                Arc::new(EventStore::new(Arc::clone(&ledger_db))),
                Arc::new(LedgerStore::new(Arc::clone(&ledger_db))),
            ))))
        } else {
            None
        },
    ]
}
