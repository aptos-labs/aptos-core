// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

//! This module provides common utilities for the DB pruner.

use crate::{
    pruner::{
        db_pruner::DBPruner,
        event_store::event_store_pruner::EventStorePruner,
        ledger_store::ledger_store_pruner::LedgerStorePruner,
        state_store::StateStorePruner,
        transaction_store::{
            transaction_store_pruner::TransactionStorePruner, write_set_pruner::WriteSetPruner,
        },
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
        Mutex::new(Arc::new(TransactionStorePruner::new(
            Arc::clone(&db),
            Arc::clone(&transaction_store),
        ))),
        Mutex::new(Arc::new(LedgerStorePruner::new(
            Arc::clone(&db),
            Arc::clone(&ledger_store),
        ))),
        Mutex::new(Arc::new(EventStorePruner::new(
            Arc::clone(&db),
            Arc::clone(&event_store),
        ))),
        Mutex::new(Arc::new(WriteSetPruner::new(
            Arc::clone(&db),
            Arc::clone(&transaction_store),
        ))),
    ]
}
