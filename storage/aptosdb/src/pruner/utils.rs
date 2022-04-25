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
use aptos_infallible::Mutex;
use schemadb::DB;
use std::{sync::Arc, time::Instant};

/// A useful utility function to instantiate all db pruners.
pub fn create_db_pruners(
    db: Arc<DB>,
    transaction_store: Arc<TransactionStore>,
    ledger_store: Arc<LedgerStore>,
    event_store: Arc<EventStore>,
) -> Vec<Mutex<Arc<dyn DBPruner + Send + Sync>>> {
    vec![
        Mutex::new(Arc::new(StateStorePruner::new(
            Arc::clone(&db),
            0,
            Instant::now(),
        ))),
        Mutex::new(Arc::new(LedgerPruner::new(
            Arc::clone(&db),
            Arc::clone(&transaction_store),
            Arc::clone(&event_store),
            Arc::clone(&ledger_store),
        ))),
    ]
}
