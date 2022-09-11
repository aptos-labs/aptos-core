// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION};
use aptos_logger::prelude::*;
use aptos_rest_client::{retriable, retriable_with_404, Client as RestClient, State, Transaction};
use futures::channel::mpsc;
use futures::{SinkExt, StreamExt};
use serde_json::Value;
use std::time::Duration;
use tokio::task::JoinHandle;
use url::Url;

// Default Values
const RETRY_TIME_MILLIS: u64 = 300;
const MAX_RETRY_TIME_MILLIS: u64 = 120000;
const TRANSACTION_FETCH_BATCH_SIZE: u16 = 500;
const TRANSACTION_CHANNEL_SIZE: usize = 35;
const MAX_TASKS: usize = 10;

#[derive(Debug)]
pub struct Fetcher {
    client: RestClient,
    options: TransactionFetcherOptions,
    chain_id: u8,
    current_version: u64,
    highest_known_version: u64,
    transactions_sender: mpsc::Sender<Vec<Transaction>>,
}

impl Fetcher {
    pub fn new(
        client: RestClient,
        starting_version: u64,
        options: TransactionFetcherOptions,
        transactions_sender: mpsc::Sender<Vec<Transaction>>,
    ) -> Self {
        Self {
            client,
            options,
            chain_id: 0,
            current_version: starting_version,
            highest_known_version: starting_version,
            transactions_sender,
        }
    }

    pub async fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let res = RestClient::try_until_ok(
            Some(self.options.max_retry_time),
            Some(self.options.starting_retry_time),
            retriable,
            || self.client.get_ledger_information(),
        )
        .await?;
        let state = res.state();
        self.highest_known_version = state.version;
        self.chain_id = state.chain_id;
        Ok(())
    }

    pub async fn run(&mut self) {
        loop {
            if self.current_version >= self.highest_known_version {
                tokio::time::sleep(self.options.starting_retry_time).await;
                if let Err(err) = self.set_highest_known_version().await {
                    error!(
                        error = format!("{:?}", err),
                        "Failed to set highest known version"
                    );
                    continue;
                } else {
                    sample!(
                        SampleRate::Frequency(10),
                        aptos_logger::info!(
                            highest_known_version = self.highest_known_version,
                            "Found new highest known version",
                        )
                    );
                }
            }

            let num_missing = self.highest_known_version - self.current_version;

            let num_batches = std::cmp::min(
                (num_missing as f64 / self.options.transaction_fetch_batch_size as f64).ceil()
                    as u64,
                self.options.max_tasks as u64,
            ) as usize;

            info!(
                num_missing = num_missing,
                num_batches = num_batches,
                current_version = self.current_version,
                highest_known_version = self.highest_known_version,
                "Preparing to fetch transactions"
            );

            let fetch_start = chrono::Utc::now().naive_utc();
            let mut futures = vec![];
            for i in 0..num_batches {
                let starting_version = self.current_version
                    + (i as u64 * self.options.transaction_fetch_batch_size as u64);
                futures.push(fetch_nexts(
                    self.client.clone(),
                    starting_version,
                    &self.options,
                ));
            }

            let mut res: Vec<Vec<Transaction>> = futures::future::join_all(futures).await;
            let total_fetched = res.iter().fold(0, |acc, v| acc + v.len());
            let fetch_millis = (chrono::Utc::now().naive_utc() - fetch_start).num_milliseconds();

            // Sort by first transaction of batch's version
            res.sort_by(|a, b| {
                a.first()
                    .unwrap()
                    .version()
                    .unwrap()
                    .cmp(&b.first().unwrap().version().unwrap())
            });

            info!(
                total_fetched = total_fetched,
                fetch_millis = fetch_millis,
                num_batches = num_batches,
                "Finished fetching transactions"
            );

            let send_start = chrono::Utc::now().naive_utc();
            // Send keeping track of the last version sent by the batch
            for batch in res {
                self.current_version = batch.last().unwrap().version().unwrap();
                self.transactions_sender
                    .send(batch)
                    .await
                    .expect("Should be able to send transaction on channel");
            }

            let send_millis = (chrono::Utc::now().naive_utc() - send_start).num_milliseconds();
            info!(
                total_sent = total_fetched,
                send_millis = send_millis,
                num_batches = num_batches,
                "Finished sending transactions"
            );
        }
    }
}

/// Fetches the next version based on its internal version counter
/// Under the hood, it fetches TRANSACTION_FETCH_BATCH_SIZE versions in bulk (when needed), and uses that buffer to feed out
/// In the event it can't fetch, it will keep retrying every RETRY_TIME_MILLIS ms
async fn fetch_nexts(
    client: RestClient,
    starting_version: u64,
    options: &TransactionFetcherOptions,
) -> Vec<Transaction> {
    let res = RestClient::try_until_ok(
        Some(options.max_retry_time),
        Some(options.starting_retry_time),
        retriable_with_404,
        || {
            client.get_transactions(
                Some(starting_version),
                Some(options.transaction_fetch_batch_size),
            )
        },
    )
    .await;
    match res {
        Ok(response) => {
            FETCHED_TRANSACTION.inc();
            remove_null_bytes_from_txns(response.into_inner())
        }
        Err(err) => {
            UNABLE_TO_FETCH_TRANSACTION.inc();
            error!(
                "Could not fetch {} transactions starting at {}. Err: {:?}",
                options.transaction_fetch_batch_size, starting_version, err
            );
            panic!(
                "Could not fetch {} transactions starting at {} in {}ms!",
                options.transaction_fetch_batch_size,
                starting_version,
                options.max_retry_time_millis
            );
        }
    }
}

#[derive(Clone, Debug)]
pub struct TransactionFetcherOptions {
    pub starting_retry_time_millis: u64,
    pub starting_retry_time: Duration,
    pub max_retry_time_millis: u64,
    pub max_retry_time: Duration,
    pub transaction_fetch_batch_size: u16,
    pub max_pending_batches: usize,
    pub max_tasks: usize,
}

impl TransactionFetcherOptions {
    pub fn new(
        starting_retry_time_millis: Option<u64>,
        max_retry_time_millis: Option<u64>,
        transaction_fetch_batch_size: Option<u16>,
        max_pending_batches: Option<usize>,
        max_tasks: Option<usize>,
    ) -> Self {
        let starting_retry_time_millis = starting_retry_time_millis.unwrap_or(RETRY_TIME_MILLIS);
        let max_retry_time_millis = max_retry_time_millis.unwrap_or(MAX_RETRY_TIME_MILLIS);

        TransactionFetcherOptions {
            starting_retry_time_millis,
            starting_retry_time: Duration::from_millis(starting_retry_time_millis),
            max_retry_time_millis,
            max_retry_time: Duration::from_millis(max_retry_time_millis),
            transaction_fetch_batch_size: transaction_fetch_batch_size
                .unwrap_or(TRANSACTION_FETCH_BATCH_SIZE),
            max_pending_batches: max_pending_batches.unwrap_or(TRANSACTION_CHANNEL_SIZE),
            max_tasks: max_tasks.unwrap_or(MAX_TASKS),
        }
    }
}

impl Default for TransactionFetcherOptions {
    fn default() -> Self {
        TransactionFetcherOptions::new(None, None, None, None, None)
    }
}

#[derive(Debug)]
pub struct TransactionFetcher {
    starting_version: u64,
    options: TransactionFetcherOptions,
    client: RestClient,
    fetcher_handle: Option<JoinHandle<()>>,
    transactions_sender: Option<mpsc::Sender<Vec<Transaction>>>,
    transaction_receiver: mpsc::Receiver<Vec<Transaction>>,
}

impl TransactionFetcher {
    pub fn new(node_url: Url, starting_version: u64, options: TransactionFetcherOptions) -> Self {
        let (transactions_sender, transaction_receiver) =
            mpsc::channel::<Vec<Transaction>>(options.max_pending_batches);

        let client = RestClient::new(node_url);

        Self {
            starting_version,
            options,
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
            let res = RestClient::try_until_ok(None, None, retriable_with_404, || {
                self.client.get_transaction_by_version(version)
            })
            .await;

            match res {
                Ok(response) => {
                    FETCHED_TRANSACTION.inc();
                    return response.into_inner();
                }
                Err(err) => {
                    UNABLE_TO_FETCH_TRANSACTION.inc();
                    error!(
                        version = version,
                        error = format!("{:?}", err),
                        "Could not fetch version, will retry. in {}ms. Err: {:?}",
                        self.options.starting_retry_time_millis,
                        err
                    );
                    tokio::time::sleep(self.options.starting_retry_time).await;
                }
            };
        }
    }

    async fn fetch_ledger_info(&mut self) -> State {
        let res =
            RestClient::try_until_ok(Some(self.options.max_retry_time), None, retriable, || {
                self.client.get_ledger_information()
            })
            .await;
        match res {
            Ok(inner) => inner.into_inner(),
            Err(err) => panic!(
                "Failed to get ledger info in {}ms: {:?}",
                self.options.max_retry_time.as_millis(),
                err
            ),
        }
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
        let options2 = self.options.clone();
        let fetcher_handle = tokio::spawn(async move {
            let mut fetcher = Fetcher::new(client, starting_version, options2, transactions_sender);
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
