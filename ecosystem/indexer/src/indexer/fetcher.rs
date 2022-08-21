// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_BLOCK, UNABLE_TO_FETCH_BLOCK};

use aptos_rest_client::{Client as RestClient, State, Transaction};
use std::time::Duration;
use url::Url;

// TODO: make this configurable
const RETRY_TIME_MILLIS: u64 = 5000;

#[derive(Debug)]
pub struct TransactionFetcher {
    client: RestClient,
    block_height: u64,
}

impl TransactionFetcher {
    pub fn new(node_url: Url, starting_block: Option<u64>) -> Self {
        let client = RestClient::new(node_url);

        Self {
            client,
            block_height: starting_block.unwrap_or(0),
        }
    }
}

#[async_trait::async_trait]
impl TransactionFetcherTrait for TransactionFetcher {
    fn get_block_height(&mut self) -> u64 {
        self.block_height
    }

    fn set_block_height(&mut self, block_height: u64) {
        self.block_height = block_height;
    }

    async fn fetch_ledger_info(&mut self) -> State {
        self.client
            .get_ledger_information()
            .await
            .expect("ledger info must be present")
            .into_inner()
    }

    /// Fetches all transactions within a block. If block height is not available, set block height using
    /// the starting version
    async fn fetch_block(&mut self) -> Vec<Transaction> {
        loop {
            let res = self.client.get_block(self.block_height, true).await;
            match res {
                Ok(response) => {
                    FETCHED_BLOCK.inc();
                    return response.into_inner().transactions.unwrap_or_else(|| {
                        panic!("Block {} missing transactions", self.block_height)
                    });
                }
                Err(err) => {
                    let err_str = err.to_string();
                    // If it's a 404, then we're all caught up; no need to increment the `UNABLE_TO_FETCH_TRANSACTION` counter
                    if err_str.contains("404") {
                        aptos_logger::debug!(
                            "Could not fetch block {}: all caught up. Will check again in {}ms.",
                            self.block_height,
                            RETRY_TIME_MILLIS,
                        );
                        tokio::time::sleep(Duration::from_millis(RETRY_TIME_MILLIS)).await;
                    }
                    UNABLE_TO_FETCH_BLOCK.inc();
                    aptos_logger::error!(
                        "Could not fetch block {}: all caught up. Will check again in {}ms. Error: {:?}",
                        self.block_height,
                        RETRY_TIME_MILLIS,
                        err
                    );
                    tokio::time::sleep(Duration::from_millis(RETRY_TIME_MILLIS)).await;
                }
            };
        }
    }

    async fn fetch_block_height_from_version(&self, version: u64) -> u64 {
        *self
            .client
            .get_block_info(version)
            .await
            .unwrap_or_else(|err| {
                panic!(
                    "Could not fetch block info for version {}. Error: {:?}",
                    version, err
                )
            })
            .inner()
            .block_height
            .inner()
    }
}

/// For mocking TransactionFetcher in tests
#[async_trait::async_trait]
pub trait TransactionFetcherTrait: Send + Sync {
    fn set_block_height(&mut self, block_height: u64);

    fn get_block_height(&mut self) -> u64;

    async fn fetch_block(&mut self) -> Vec<Transaction>;

    async fn fetch_ledger_info(&mut self) -> State;

    async fn fetch_block_height_from_version(&self, version: u64) -> u64;
}
