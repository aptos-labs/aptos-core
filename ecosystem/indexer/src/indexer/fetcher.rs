// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION};
use aptos_rest_client::{Client as RestClient, Transaction};
use std::time::Duration;
use url::Url;

#[derive(Debug)]
pub struct TransactionFetcher {
    client: RestClient,
    version: u64,
}

impl TransactionFetcher {
    pub fn new(node_url: Url, starting_version: Option<u64>) -> Self {
        let client = RestClient::new(node_url);

        Self {
            client,
            version: starting_version.unwrap_or(0),
        }
    }

    pub fn set_version(&mut self, version: u64) {
        self.version = version;
    }

    pub async fn fetch_next(&mut self) -> Transaction {
        let transaction = self.fetch_version(self.version).await;
        self.version += 1;
        transaction
    }

    /// Fetches the next version based on its internal version counter
    /// In the event it can't, it will keep retrying every second.
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
                        "Could not fetch version {}, will retry in 5s. Err: {:?}",
                        version,
                        err
                    );
                    tokio::time::sleep(Duration::from_millis(5000)).await;
                }
            };
        }
    }
}
