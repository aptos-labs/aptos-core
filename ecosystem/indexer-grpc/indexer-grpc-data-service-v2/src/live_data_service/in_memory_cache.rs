// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    connection_manager::ConnectionManager,
    live_data_service::{data_manager::DataManager, fetch_manager::FetchManager},
    metrics::TIMER,
};
use velor_protos::transaction::v1::Transaction;
use velor_transaction_filter::{BooleanTransactionFilter, Filterable};
use prost::Message;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::trace;

pub(super) struct InMemoryCache<'a> {
    pub(super) data_manager: Arc<RwLock<DataManager>>,
    pub(super) fetch_manager: Arc<FetchManager<'a>>,
}

impl<'a> InMemoryCache<'a> {
    pub(super) fn new(
        connection_manager: Arc<ConnectionManager>,
        known_latest_version: u64,
        num_slots: usize,
        size_limit_bytes: usize,
    ) -> Self {
        let data_manager = Arc::new(RwLock::new(DataManager::new(
            known_latest_version + 1,
            num_slots,
            size_limit_bytes,
        )));
        let fetch_manager = Arc::new(FetchManager::new(data_manager.clone(), connection_manager));
        Self {
            data_manager,
            fetch_manager,
        }
    }

    pub(super) async fn get_data(
        &'a self,
        starting_version: u64,
        ending_version: u64,
        max_num_transactions_per_batch: usize,
        max_bytes_per_batch: usize,
        filter: &Option<BooleanTransactionFilter>,
    ) -> Option<(Vec<Transaction>, usize, u64)> {
        let _timer = TIMER.with_label_values(&["cache_get_data"]).start_timer();

        while starting_version >= self.data_manager.read().await.end_version {
            trace!("Reached head, wait...");
            let num_transactions = self
                .fetch_manager
                .fetching_latest_data_task
                .read()
                .await
                .as_ref()
                .unwrap()
                .clone()
                .await;

            trace!("Done waiting, got {num_transactions} transactions at head.");
        }

        loop {
            let data_manager = self.data_manager.read().await;

            trace!("Getting data from cache, requested_version: {starting_version}, oldest available version: {}.", data_manager.start_version);
            if starting_version < data_manager.start_version {
                return None;
            }

            if data_manager.get_data(starting_version).is_none() {
                drop(data_manager);
                self.fetch_manager.fetch_past_data(starting_version).await;
                continue;
            }

            let mut total_bytes = 0;
            let mut version = starting_version;
            let ending_version = ending_version.min(data_manager.end_version);

            let mut result = Vec::new();
            while version < ending_version
                && total_bytes < max_bytes_per_batch
                && result.len() < max_num_transactions_per_batch
            {
                if let Some(transaction) = data_manager.get_data(version).as_ref() {
                    // NOTE: We allow 1 more txn beyond the size limit here, for simplicity.
                    if filter.is_none() || filter.as_ref().unwrap().matches(transaction) {
                        total_bytes += transaction.encoded_len();
                        result.push(transaction.as_ref().clone());
                    }
                    version += 1;
                } else {
                    break;
                }
            }
            trace!("Data was sent from cache, last version: {}.", version - 1);
            return Some((result, total_bytes, version - 1));
        }
    }
}
