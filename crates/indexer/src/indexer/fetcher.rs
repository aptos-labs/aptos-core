// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::counters::{FETCHED_TRANSACTION, UNABLE_TO_FETCH_TRANSACTION};
use aptos_api::Context;
use aptos_api_types::{AsConverter, LedgerInfo, Transaction, TransactionOnChainData};
use aptos_logger::prelude::*;
use futures::{channel::mpsc, SinkExt};
use std::{sync::Arc, time::Duration};
use tokio::task::JoinHandle;

// Default Values
const RETRY_TIME_MILLIS: u64 = 300;
const MAX_RETRY_TIME_MILLIS: u64 = 120000;
const TRANSACTION_FETCH_BATCH_SIZE: u16 = 500;
const TRANSACTION_CHANNEL_SIZE: usize = 35;

#[derive(Debug)]
pub struct Fetcher {
    pub context: Arc<Context>,
    options: TransactionFetcherOptions,
    chain_id: u8,
    current_version: u64,
    highest_known_version: u64,
    transactions_sender: mpsc::Sender<Vec<Transaction>>,
}

impl Fetcher {
    pub fn new(
        context: Arc<Context>,
        starting_version: u64,
        options: TransactionFetcherOptions,
        transactions_sender: mpsc::Sender<Vec<Transaction>>,
    ) -> Self {
        Self {
            context,
            options,
            chain_id: 0,
            current_version: starting_version,
            highest_known_version: 0,
            transactions_sender,
        }
    }

    pub fn set_highest_known_version(&mut self) -> anyhow::Result<()> {
        let info = self.context.get_latest_ledger_info_wrapped()?;
        self.highest_known_version = info.ledger_version.0;
        self.chain_id = info.chain_id;
        Ok(())
    }

    /// Will keep looping and checking the latest ledger info to see if there are new transactions
    /// If there are, it will set the highest known version
    async fn ensure_highest_known_version(&mut self) {
        let mut empty_loops = 0;
        while self.highest_known_version == 0 || self.current_version > self.highest_known_version {
            if empty_loops > 0 {
                tokio::time::sleep(self.options.starting_retry_time).await;
            }
            empty_loops += 1;
            if let Err(err) = self.set_highest_known_version() {
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
    }

    /// Main loop for fetching transactions
    /// Fetches transactions in batches of `options.transaction_fetch_batch_size` and sends them to the processor channel.
    /// If the processor channel is full, it will wait for the processor to catch up.
    /// 1. Get the latest ledger info, and set the highest known version (if we've caught up)
    /// 2. Determine how many batches of size `options.transaction_fetch_batch_size` we need to catch up
    /// 3. Spawn tasks which fetch 'raw' `OnChainTransactions` from storage, and convert them to `Transaction`s. We spawn at most `options.max_tasks` tasks.
    /// 4. We wait for all the tasks to complete, then send the `Transaction`s to the processor, via the `transactions_sender` channel.
    pub async fn run(&mut self) {
        let transaction_fetch_batch_size = self.options.transaction_fetch_batch_size;
        loop {
            self.ensure_highest_known_version().await;

            info!(
                current_version = self.current_version,
                highest_known_version = self.highest_known_version,
                max_batch_size = transaction_fetch_batch_size,
                "Preparing to fetch transactions"
            );

            let fetch_start = chrono::Utc::now().naive_utc();
            let mut tasks = vec![];
            let mut starting_version = self.current_version;
            let mut num_fetches = 0;

            while num_fetches < self.options.max_tasks
                && starting_version <= self.highest_known_version
            {
                let num_transactions_to_fetch = std::cmp::min(
                    transaction_fetch_batch_size as u64,
                    self.highest_known_version - starting_version + 1,
                ) as u16;

                let context = self.context.clone();
                let highest_known_version = self.highest_known_version;
                let task = tokio::spawn(async move {
                    fetch_nexts(
                        context,
                        starting_version,
                        highest_known_version,
                        num_transactions_to_fetch,
                    )
                    .await
                });
                tasks.push(task);
                starting_version += num_transactions_to_fetch as u64;
                num_fetches += 1;
            }

            let batches = match futures::future::try_join_all(tasks).await {
                Ok(res) => res,
                Err(err) => panic!("Error fetching transaction batches: {:?}", err),
            };

            let versions_fetched = batches.iter().fold(0, |acc, v| acc + v.len());
            let fetch_millis = (chrono::Utc::now().naive_utc() - fetch_start).num_milliseconds();
            info!(
                versions_fetched = versions_fetched,
                fetch_millis = fetch_millis,
                num_batches = batches.len(),
                "Finished fetching transaction batches"
            );
            self.send_transaction_batches(batches).await;
        }
    }

    /// Sends the transaction batches to the processor via the `transactions_sender` channel
    async fn send_transaction_batches(&mut self, transaction_batches: Vec<Vec<Transaction>>) {
        let send_start = chrono::Utc::now().naive_utc();
        let num_batches = transaction_batches.len();
        let mut versions_sent: usize = 0;
        // Send keeping track of the last version sent by the batch
        for batch in transaction_batches {
            versions_sent += batch.len();
            self.current_version = std::cmp::max(
                batch.last().unwrap().version().unwrap() + 1,
                self.current_version,
            );
            self.transactions_sender
                .send(batch)
                .await
                .expect("Should be able to send transaction on channel");
        }

        let send_millis = (chrono::Utc::now().naive_utc() - send_start).num_milliseconds();
        info!(
            versions_sent = versions_sent,
            send_millis = send_millis,
            num_batches = num_batches,
            "Finished sending transaction batches"
        );
    }
}

async fn fetch_raw_txns_with_retries(
    context: Arc<Context>,
    starting_version: u64,
    ledger_version: u64,
    num_transactions_to_fetch: u16,
    max_retries: u8,
) -> Vec<TransactionOnChainData> {
    let mut retries = 0;
    loop {
        match context.get_transactions(starting_version, num_transactions_to_fetch, ledger_version)
        {
            Ok(raw_txns) => return raw_txns,
            Err(err) => {
                UNABLE_TO_FETCH_TRANSACTION.inc();
                retries += 1;
                if retries >= max_retries {
                    error!(
                        starting_version = starting_version,
                        num_transactions = num_transactions_to_fetch,
                        error = format!("{:?}", err),
                        "Could not fetch transactions: retries exhausted",
                    );
                    panic!(
                        "Could not fetch {} transactions after {} retries, starting at {}: {:?}",
                        num_transactions_to_fetch, retries, starting_version, err
                    );
                } else {
                    error!(
                        starting_version = starting_version,
                        num_transactions = num_transactions_to_fetch,
                        error = format!("{:?}", err),
                        "Could not fetch transactions: will retry",
                    );
                }
                tokio::time::sleep(Duration::from_millis(300)).await;
            },
        }
    }
}

async fn fetch_nexts(
    context: Arc<Context>,
    starting_version: u64,
    ledger_version: u64,
    num_transactions_to_fetch: u16,
) -> Vec<Transaction> {
    let start_millis = chrono::Utc::now().naive_utc();

    let raw_txns = fetch_raw_txns_with_retries(
        context.clone(),
        starting_version,
        ledger_version,
        num_transactions_to_fetch,
        3,
    )
    .await;

    let (_, _, block_event) = context
        .db
        .get_block_info_by_version(starting_version)
        .unwrap_or_else(|_| {
            panic!(
                "Could not get block_info for start version {}",
                starting_version,
            )
        });
    let mut timestamp = block_event.proposed_time();
    let mut epoch = block_event.epoch();
    let mut epoch_bcs = aptos_api_types::U64::from(epoch);
    let mut block_height = block_event.height();
    let mut block_height_bcs = aptos_api_types::U64::from(block_height);

    let state_view = context.latest_state_view().unwrap();
    let converter = state_view.as_converter(context.db.clone(), context.indexer_reader.clone());

    let mut transactions = vec![];
    for (ind, raw_txn) in raw_txns.into_iter().enumerate() {
        let txn_version = raw_txn.version;
        // Do not update block_height if first block is block metadata
        if ind > 0 {
            // Update the timestamp if the next block occurs
            if let Some(txn) = raw_txn.transaction.try_as_block_metadata_ext() {
                timestamp = txn.timestamp_usecs();
                epoch = txn.epoch();
                epoch_bcs = aptos_api_types::U64::from(epoch);
                block_height += 1;
                block_height_bcs = aptos_api_types::U64::from(block_height);
            } else if let Some(txn) = raw_txn.transaction.try_as_block_metadata() {
                timestamp = txn.timestamp_usecs();
                epoch = txn.epoch();
                epoch_bcs = aptos_api_types::U64::from(epoch);
                block_height += 1;
                block_height_bcs = aptos_api_types::U64::from(block_height);
            }
        }
        let res = converter
            .try_into_onchain_transaction(timestamp, raw_txn)
            .map(|mut txn| {
                match txn {
                    Transaction::PendingTransaction(_) => {
                        unreachable!("Indexer should never see pending transactions")
                    },
                    Transaction::UserTransaction(ref mut ut) => {
                        ut.info.block_height = Some(block_height_bcs);
                        ut.info.epoch = Some(epoch_bcs);
                    },
                    Transaction::GenesisTransaction(ref mut gt) => {
                        gt.info.block_height = Some(block_height_bcs);
                        gt.info.epoch = Some(epoch_bcs);
                    },
                    Transaction::BlockMetadataTransaction(ref mut bmt) => {
                        bmt.info.block_height = Some(block_height_bcs);
                        bmt.info.epoch = Some(epoch_bcs);
                    },
                    Transaction::StateCheckpointTransaction(ref mut sct) => {
                        sct.info.block_height = Some(block_height_bcs);
                        sct.info.epoch = Some(epoch_bcs);
                    },
                    Transaction::BlockEpilogueTransaction(ref mut bet) => {
                        bet.info.block_height = Some(block_height_bcs);
                        bet.info.epoch = Some(epoch_bcs);
                    },
                    Transaction::ValidatorTransaction(ref mut st) => {
                        let info = st.transaction_info_mut();
                        info.block_height = Some(block_height_bcs);
                        info.epoch = Some(epoch_bcs);
                    },
                    Transaction::ScheduledTransaction(ref mut st) => {
                        st.info.block_height = Some(block_height_bcs);
                        st.info.epoch = Some(epoch_bcs);
                    },
                };
                txn
            });
        match res {
            Ok(transaction) => transactions.push(transaction),
            Err(err) => {
                UNABLE_TO_FETCH_TRANSACTION.inc();
                error!(
                    version = txn_version,
                    error = format!("{:?}", err),
                    "Could not convert from OnChainTransactions",
                );
                // IN CASE WE NEED TO SKIP BAD TXNS
                // continue;
                panic!(
                    "Could not convert txn {} from OnChainTransactions: {:?}",
                    txn_version, err
                );
            },
        }
    }

    if transactions.is_empty() {
        panic!("No transactions!");
    }

    let fetch_millis = (chrono::Utc::now().naive_utc() - start_millis).num_milliseconds();

    info!(
        starting_version = starting_version,
        num_transactions = transactions.len(),
        time_millis = fetch_millis,
        actual_last_version = transactions
            .last()
            .map(|txn| txn.version().unwrap())
            .unwrap_or(0),
        "Fetched transactions",
    );

    FETCHED_TRANSACTION.inc();

    transactions
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

fn default_if_zero<T>(value: Option<T>, default: T) -> T
where
    T: PartialEq + Copy + Default,
{
    match value {
        Some(v) => {
            if v == T::default() {
                default
            } else {
                v
            }
        },
        None => default,
    }
}

impl TransactionFetcherOptions {
    pub fn new(
        starting_retry_time_millis: Option<u64>,
        max_retry_time_millis: Option<u64>,
        transaction_fetch_batch_size: Option<u16>,
        max_pending_batches: Option<usize>,
        max_tasks: usize,
    ) -> Self {
        let starting_retry_time_millis =
            default_if_zero(starting_retry_time_millis, RETRY_TIME_MILLIS);

        let max_retry_time_millis = default_if_zero(max_retry_time_millis, MAX_RETRY_TIME_MILLIS);

        let transaction_fetch_batch_size =
            default_if_zero(transaction_fetch_batch_size, TRANSACTION_FETCH_BATCH_SIZE);

        let max_pending_batches = default_if_zero(max_pending_batches, TRANSACTION_CHANNEL_SIZE);

        TransactionFetcherOptions {
            starting_retry_time_millis,
            starting_retry_time: Duration::from_millis(starting_retry_time_millis),
            max_retry_time_millis,
            max_retry_time: Duration::from_millis(max_retry_time_millis),
            transaction_fetch_batch_size,
            max_pending_batches,
            max_tasks: std::cmp::max(max_tasks, 1),
        }
    }
}

impl Default for TransactionFetcherOptions {
    fn default() -> Self {
        TransactionFetcherOptions::new(None, None, None, None, 5)
    }
}

pub struct TransactionFetcher {
    starting_version: u64,
    options: TransactionFetcherOptions,
    pub context: Arc<Context>,
    fetcher_handle: Option<JoinHandle<()>>,
    transactions_sender: Option<mpsc::Sender<Vec<Transaction>>>,
    transaction_receiver: mpsc::Receiver<Vec<Transaction>>,
}

impl TransactionFetcher {
    pub fn new(
        context: Arc<Context>,
        starting_version: u64,
        options: TransactionFetcherOptions,
    ) -> Self {
        let (transactions_sender, transaction_receiver) =
            mpsc::channel::<Vec<Transaction>>(options.max_pending_batches);

        Self {
            starting_version,
            options,
            context,
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
        // try_next is nonblocking unlike next. It'll try to fetch the next one and return immediately.
        match self.transaction_receiver.try_next() {
            Ok(Some(transactions)) => transactions,
            Ok(None) => {
                // We never close the channel, so this should never happen
                panic!("Transaction fetcher channel closed");
            },
            // The error here is when the channel is empty which we definitely expect.
            Err(_) => vec![],
        }
    }

    fn fetch_ledger_info(&mut self) -> LedgerInfo {
        self.context
            .get_latest_ledger_info_wrapped()
            .unwrap_or_else(|err| panic!("Failed to get ledger info: {}", err))
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
        let context = self.context.clone();
        let transactions_sender = self.transactions_sender.take().unwrap();
        let starting_version = self.starting_version;

        let options2 = self.options.clone();
        let fetcher_handle = tokio::spawn(async move {
            let mut fetcher =
                Fetcher::new(context, starting_version, options2, transactions_sender);
            fetcher.run().await;
        });
        self.fetcher_handle = Some(fetcher_handle);
    }
}

/// For mocking TransactionFetcher in tests
#[async_trait::async_trait]
pub trait TransactionFetcherTrait: Send + Sync {
    async fn fetch_next_batch(&mut self) -> Vec<Transaction>;

    fn fetch_ledger_info(&mut self) -> LedgerInfo;

    async fn set_version(&mut self, version: u64);

    async fn start(&mut self);
}
