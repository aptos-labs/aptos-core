// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION};
use aptos_rest_client::{Client as RestClient, Transaction};
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;

// TODO: make this configurable
const RETRY_TIME_MILLIS: u64 = 5000;
const TRANSACTION_FETCH_BATCH_SIZE: u64 = 500;

#[derive(Debug)]
pub struct TransactionFetcher {
    client: RestClient,
    version: u64,
    transactions_buffer: Mutex<Vec<Transaction>>,
}

impl TransactionFetcher {
    pub fn new(node_url: Url, starting_version: Option<u64>) -> Self {
        let client = RestClient::new(node_url);

        Self {
            client,
            version: starting_version.unwrap_or(0),
            transactions_buffer: Default::default(),
        }
    }

    pub fn set_version(&mut self, version: u64) {
        self.version = version;
    }

    /// Fetches the next version based on its internal version counter
    /// Under the hood, it fetches TRANSACTION_FETCH_BATCH_SIZE versions in bulk (when needed), and uses that buffer to feed out
    /// In the event it can't fetch, it will keep retrying every RETRY_TIME_MILLIS ms
    pub async fn fetch_next(&mut self) -> Transaction {
        let mut transactions_buffer = self.transactions_buffer.lock().await;
        if transactions_buffer.is_empty() {
            // Fill it up!
            loop {
                let res = self
                    .client
                    .get_transactions(Some(self.version), Some(TRANSACTION_FETCH_BATCH_SIZE))
                    .await;
                match res {
                    Ok(response) => {
                        FETCHED_TRANSACTION.inc();
                        let mut transactions = response.into_inner();
                        transactions.reverse();
                        *transactions_buffer = transactions;
                        break;
                    }
                    Err(err) => {
                        let err_str = err.to_string();
                        // If it's a 404, then we're all caught up; no need to increment the `UNABLE_TO_FETCH_TRANSACTION` counter
                        if err_str.contains("404") {
                            aptos_logger::debug!(
                            "Could not fetch {} transactions starting at {}: all caught up. Will check again in {}ms.",
                            TRANSACTION_FETCH_BATCH_SIZE,
                            self.version,
                            RETRY_TIME_MILLIS,
                        );
                            tokio::time::sleep(Duration::from_millis(RETRY_TIME_MILLIS)).await;
                            continue;
                        }
                        UNABLE_TO_FETCH_TRANSACTION.inc();
                        aptos_logger::error!(
                            "Could not fetch {} transactions starting at {}, will retry in {}ms. Err: {:?}",
                            TRANSACTION_FETCH_BATCH_SIZE,
                            self.version,
                            RETRY_TIME_MILLIS,
                            err
                        );
                        tokio::time::sleep(Duration::from_millis(RETRY_TIME_MILLIS)).await;
                    }
                };
            }
        }
        // At this point we're guaranteed to have something in the buffer
        let transaction = transactions_buffer.pop().unwrap();
        self.version += 1;
        transaction
    }

    /// fetches one version; this used for error checking/repair/etc
    /// In the event it can't, it will keep retrying every RETRY_TIME_MILLIS ms
    pub async fn fetch_version(&self, version: u64) -> Transaction {
        loop {
            let res = self.client.get_transaction_by_version(version).await;
            match res {
                Ok(response) => {
                    FETCHED_TRANSACTION.inc();
                    return response.into_inner();
                }
                Err(err) => {
                    UNABLE_TO_FETCH_TRANSACTION.inc();
                    aptos_logger::error!(
                        "Could not fetch version {}, will retry in {}ms. Err: {:?}",
                        version,
                        RETRY_TIME_MILLIS,
                        err
                    );
                    tokio::time::sleep(Duration::from_millis(RETRY_TIME_MILLIS)).await;
                }
            };
        }
    }
}
