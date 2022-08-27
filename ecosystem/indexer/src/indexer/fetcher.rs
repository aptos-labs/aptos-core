// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION};
use aptos_rest_client::{Client as RestClient, State, Transaction};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::task::JoinHandle;
use url::Url;

// TODO: make this configurable
const RETRY_TIME_MILLIS: u64 = 1000;
const TRANSACTION_FETCH_BATCH_SIZE: u16 = 500;
const TRANSACTION_CHANNEL_SIZE: usize = 35;
const MAX_THREADS: usize = 10;
const MAX_RETRIES: usize = 10;

#[derive(Debug)]
pub struct Fetcher {
    client: RestClient,
    chain_id: u8,
    current_version: u64,
    highest_known_version: u64,
    transactions_sender: mpsc::Sender<Vec<Transaction>>,
}

impl Fetcher {
    pub fn new(
        client: RestClient,
        current_version: u64,
        transactions_sender: mpsc::Sender<Vec<Transaction>>,
    ) -> Self {
        Self {
            client,
            chain_id: 0,
            current_version,
            highest_known_version: current_version,
            transactions_sender,
        }
    }

    pub async fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let res = self.client.get_ledger_information().await?;
        let state = res.state();
        self.highest_known_version = state.version;
        self.chain_id = state.chain_id;
        Ok(())
    }

    pub async fn run(&mut self) {
        loop {
            if self.current_version >= self.highest_known_version {
                tokio::time::sleep(Duration::from_millis(200)).await;
                if let Err(err) = self.set_highest_known_version().await {
                    aptos_logger::error!("Failed to set highest known version. Err: {:?}", err);
                    continue;
                }
            }

            let num_missing = self.highest_known_version - self.current_version;
            let num_batches = std::cmp::min(
                (num_missing as f64 / TRANSACTION_FETCH_BATCH_SIZE as f64).ceil() as u64,
                MAX_THREADS as u64,
            ) as usize;
            let mut futures = vec![];
            for i in 0..num_batches {
                futures.push(fetch_nexts(
                    self.client.clone(),
                    self.current_version + (i as u64 * TRANSACTION_FETCH_BATCH_SIZE as u64),
                ));
            }
            let mut res: Vec<Vec<Transaction>> = futures::future::join_all(futures).await;
            // Sort by first transaction of batch's version
            res.sort_by(|a, b| {
                a.first()
                    .unwrap()
                    .version()
                    .unwrap()
                    .cmp(&b.first().unwrap().version().unwrap())
            });

            // Send keeping track of the last version sent by the batch
            for batch in res {
                self.current_version = batch.last().unwrap().version().unwrap();
                self.transactions_sender
                    .send(batch)
                    .await
                    .expect("Should be able to send transaction on channel");
            }
        }
    }
}

/// Fetches the next version based on its internal version counter
/// Under the hood, it fetches TRANSACTION_FETCH_BATCH_SIZE versions in bulk (when needed), and uses that buffer to feed out
/// In the event it can't fetch, it will keep retrying every RETRY_TIME_MILLIS ms
async fn fetch_nexts(client: RestClient, starting_version: u64) -> Vec<Transaction> {
    let mut retries = 0;
    let mut backoff_time = Duration::from_millis(RETRY_TIME_MILLIS);
    while retries < MAX_RETRIES {
        retries += 1;

        let res = client
            .get_transactions(Some(starting_version), Some(TRANSACTION_FETCH_BATCH_SIZE))
            .await;

        match res {
            Ok(response) => {
                FETCHED_TRANSACTION.inc();
                return remove_null_bytes_from_txns(response.into_inner());
            }
            Err(err) => {
                let err_str = err.to_string();
                // If it's a 404, then we're all caught up; no need to increment the `UNABLE_TO_FETCH_TRANSACTION` counter
                if err_str.contains("404") {
                    aptos_logger::debug!(
                            "Could not fetch {} transactions starting at {}: all caught up. Will check again in {}ms.",
                            TRANSACTION_FETCH_BATCH_SIZE,
                            starting_version,
                            backoff_time.as_millis(),
                        );
                }
                UNABLE_TO_FETCH_TRANSACTION.inc();
                aptos_logger::error!(
                    "Could not fetch {} transactions starting at {}, will retry in {}ms. Err: {:?}",
                    TRANSACTION_FETCH_BATCH_SIZE,
                    starting_version,
                    backoff_time.as_millis(),
                    err
                );
            }
        }
        tokio::time::sleep(backoff_time).await;
        backoff_time = backoff_time * 3 / 2;
    }

    panic!(
        "Could not fetch {} transactions starting at {} after {} retries!",
        TRANSACTION_FETCH_BATCH_SIZE, starting_version, retries
    );
}

#[derive(Debug)]
pub struct TransactionFetcher {
    starting_version: u64,
    client: RestClient,
    fetcher_handle: Option<JoinHandle<()>>,
    transactions_sender: Option<mpsc::Sender<Vec<Transaction>>>,
    transaction_receiver: mpsc::Receiver<Vec<Transaction>>,
}

impl TransactionFetcher {
    pub fn new(node_url: Url, starting_version: Option<u64>) -> Self {
        let (transactions_sender, transaction_receiver) =
            mpsc::channel::<Vec<Transaction>>(TRANSACTION_CHANNEL_SIZE);

        let client = RestClient::new(node_url);

        Self {
            starting_version: starting_version.unwrap_or(0),
            client,
            fetcher_handle: None,
            transactions_sender: Some(transactions_sender),
            transaction_receiver,
        }
    }
}

#[async_trait::async_trait]
impl TransactionFetcherTrait for TransactionFetcher {
    /// Fetches the next batch based on its internal version counter
    async fn fetch_next_batch(&mut self) -> Vec<Transaction> {
        self.transaction_receiver
            .next()
            .await
            .expect("No transactions, producer of batches died")
    }

    /// fetches one version; this used for error checking/repair/etc
    /// In the event it can't, it will keep retrying every RETRY_TIME_MILLIS ms
    async fn fetch_version(&self, version: u64) -> Transaction {
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

    async fn fetch_ledger_info(&mut self) -> State {
        let mut retries = 0;
        while retries < MAX_RETRIES {
            retries += 1;
            match self.client.get_ledger_information().await {
                Ok(inner) => return inner.into_inner(),
                Err(err) => {
                    aptos_logger::warn!(
                        "Failed to get ledger info, will retry in {}ms. Err: {:?}",
                        RETRY_TIME_MILLIS,
                        err
                    );
                    tokio::time::sleep(Duration::from_secs(RETRY_TIME_MILLIS)).await
                }
            }
        }
        panic!("Failed to get ledger info after {} retries", retries);
    }

    async fn set_version(&mut self, version: u64) {
        if self.fetcher_handle.is_some() {
            panic!("TransactionFetcher already started!");
        }
        self.starting_version = version;
    }

    async fn start(&mut self) {
        if self.fetcher_handle.is_some() {
            panic!("TransactionFetcher already started!");
        }
        let client = self.client.clone();
        let transactions_sender = self.transactions_sender.take().unwrap();
        let starting_version = self.starting_version;
        let fetcher_handle = tokio::spawn(async move {
            let mut fetcher = Fetcher::new(client, starting_version, transactions_sender);
            fetcher.run().await;
        });
        self.fetcher_handle = Some(fetcher_handle);
    }
}

pub fn string_null_byte_replacement(value: &mut str) -> String {
    value.replace('\u{0000}', "").replace("\\u0000", "")
}

pub fn recurse_remove_null_bytes_from_json(sub_json: &mut Value) {
    match sub_json {
        Value::Array(array) => {
            for item in array {
                recurse_remove_null_bytes_from_json(item);
            }
        }
        Value::Object(object) => {
            for (_key, value) in object {
                recurse_remove_null_bytes_from_json(value);
            }
        }
        Value::String(str) => {
            if !str.is_empty() {
                let replacement = string_null_byte_replacement(str);
                *str = replacement;
            }
        }
        _ => {}
    }
}

pub fn remove_null_bytes_from_txns(txns: Vec<Transaction>) -> Vec<Transaction> {
    txns.iter()
        .map(|txn| {
            let mut txn_json = serde_json::to_value(txn).unwrap();
            recurse_remove_null_bytes_from_json(&mut txn_json);
            serde_json::from_value::<Transaction>(txn_json).unwrap()
        })
        .collect::<Vec<Transaction>>()
}

/// For mocking TransactionFetcher in tests
#[async_trait::async_trait]
pub trait TransactionFetcherTrait: Send + Sync {
    async fn fetch_next_batch(&mut self) -> Vec<Transaction>;

    async fn fetch_version(&self, version: u64) -> Transaction;

    async fn fetch_ledger_info(&mut self) -> State;

    async fn set_version(&mut self, version: u64);

    async fn start(&mut self);
}
