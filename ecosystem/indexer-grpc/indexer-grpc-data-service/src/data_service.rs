// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator, compression_util::StorageFormat, types::RedisUrl,
};
use aptos_protos::{
    indexer::v1::{GetTransactionsRequest, TransactionsResponse},
    transaction::v1::Transaction,
};
use futures::future::{BoxFuture, FutureExt, Shared};
use prost::Message;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    RwLock,
};
use tonic::Status;
use tracing::{error, info};

pub static NUM_SLOTS: usize = 200000000;
pub static SIZE_LIMIT: usize = 10000000000;
pub static DEFAULT_MAX_BATCH_SIZE: usize = 10000;

type FetchKey = u64;

struct DataClient {
    cache_operator: CacheOperator<redis::aio::ConnectionManager>,
}

impl DataClient {
    fn new(cache_operator: CacheOperator<redis::aio::ConnectionManager>) -> Self {
        Self { cache_operator }
    }

    async fn fetch_transactions(
        &self,
        starting_version: u64,
        num_transactions: usize,
    ) -> Vec<Transaction> {
        let res = self
            .cache_operator
            .clone()
            .get_transactions(starting_version, num_transactions as u64)
            .await
            .unwrap();
        res
    }
}

type FetchTask<'a> = Shared<BoxFuture<'a, usize>>;

struct FetchManager<'a> {
    data_manager: Arc<RwLock<DataManager>>,
    data_client: Arc<DataClient>,
    pending_fetches: RwLock<HashMap<FetchKey, FetchTask<'a>>>,
    fetching_latest_data_task: RwLock<Option<FetchTask<'a>>>,
}

impl<'a> FetchManager<'a> {
    fn new(
        data_manager: Arc<RwLock<DataManager>>,
        cache_operator: CacheOperator<redis::aio::ConnectionManager>,
    ) -> Self {
        Self {
            data_manager,
            data_client: Arc::new(DataClient::new(cache_operator)),
            pending_fetches: RwLock::new(HashMap::new()),
            fetching_latest_data_task: RwLock::new(None),
        }
    }

    async fn fetch_past_data(&'a self, version: u64) -> FetchTask<'a> {
        let fetch_key = version / 100 * 100;
        if let Some(fetch_task) = self.pending_fetches.read().await.get(&fetch_key) {
            return fetch_task.clone();
        }

        let fetch_task = Self::fetch_and_update_cache(
            self.data_client.clone(),
            self.data_manager.clone(),
            fetch_key,
            100,
        )
        .boxed()
        .shared();
        self.pending_fetches
            .write()
            .await
            .insert(fetch_key, fetch_task.clone());

        fetch_task
    }

    async fn fetch_and_update_cache(
        data_client: Arc<DataClient>,
        data_manager: Arc<RwLock<DataManager>>,
        version: u64,
        num_transactions: usize,
    ) -> usize {
        let transactions = data_client
            .fetch_transactions(version, num_transactions)
            .await;
        let len = transactions.len();

        if len > 0 {
            data_manager
                .write()
                .await
                .update_data(version, transactions);
        }

        len
    }

    async fn fetch_latest_data(&'a self) -> usize {
        let version = self.data_manager.read().await.end_version;
        info!("Fetching latest data starting from version {version}.");
        loop {
            let num_transactions = Self::fetch_and_update_cache(
                self.data_client.clone(),
                self.data_manager.clone(),
                version,
                100,
            )
            .await;
            if num_transactions != 0 {
                info!("Finished fetching latest data, got {num_transactions} num_transactions starting from version {version}.");
                return num_transactions;
            }
            tokio::time::sleep(Duration::from_millis(200)).await;
        }
    }

    async fn continuously_fetch_latest_data(&'a self) {
        loop {
            let task = self.fetch_latest_data().boxed().shared();
            *self.fetching_latest_data_task.write().await = Some(task.clone());
            let _ = task.await;
        }
    }
}

struct DataManager {
    start_version: u64,
    end_version: u64,
    data: Vec<Option<Box<Transaction>>>,

    soft_limit_for_eviction: usize,
    eviction_target: usize,
    total_size: usize,
    num_slots: usize,
}

impl DataManager {
    fn new(end_version: u64, num_slots: usize, size_limit_bytes: usize) -> Self {
        Self {
            start_version: end_version.saturating_sub(num_slots as u64),
            end_version,
            data: vec![None; num_slots],
            soft_limit_for_eviction: size_limit_bytes,
            eviction_target: size_limit_bytes,
            total_size: 0,
            num_slots,
        }
    }

    fn update_data(&mut self, start_version: u64, transactions: Vec<Transaction>) {
        if start_version > self.end_version {
            // TODO(grao): unexpected
            return;
        }

        let end_version = start_version + transactions.len() as u64;
        if end_version <= self.start_version {
            return;
        }

        let num_to_skip = self.start_version.saturating_sub(start_version);
        let start_version = start_version.max(self.start_version);

        let mut size_increased = 0;
        let mut size_decreased = 0;

        for (i, transaction) in transactions
            .into_iter()
            .enumerate()
            .skip(num_to_skip as usize)
        {
            let version = start_version + i as u64;
            if version % 100 == 0 {
                info!("version: {version}, transaction: {transaction:?}");
            }
            let slot_index = version as usize % self.num_slots;
            if let Some(transaction) = self.data[slot_index].take() {
                size_decreased += transaction.encoded_len();
            }
            size_increased += transaction.encoded_len();
            self.data[version as usize % self.num_slots] = Some(Box::new(transaction));
        }

        if end_version > self.end_version {
            self.end_version = end_version;
            if self.start_version + (self.num_slots as u64) < end_version {
                self.start_version = end_version - self.num_slots as u64;
            }
        }

        self.total_size += size_increased;
        self.total_size -= size_decreased;

        if self.total_size >= self.soft_limit_for_eviction {
            while self.total_size >= self.eviction_target {
                if let Some(transaction) =
                    self.data[self.start_version as usize % self.num_slots].take()
                {
                    self.total_size -= transaction.encoded_len();
                    drop(transaction);
                }
                self.start_version += 1;
            }
        }
    }
}

pub struct InMemoryCache<'a> {
    data_manager: Arc<RwLock<DataManager>>,
    fetch_manager: Arc<FetchManager<'a>>,
}

impl<'a> InMemoryCache<'a> {
    pub fn new(
        cache_operator: CacheOperator<redis::aio::ConnectionManager>,
        known_latest_version: u64,
        num_slots: usize,
        size_limit_bytes: usize,
    ) -> Self {
        let data_manager = Arc::new(RwLock::new(DataManager::new(
            known_latest_version + 1,
            num_slots,
            size_limit_bytes,
        )));
        let fetch_manager = Arc::new(FetchManager::new(data_manager.clone(), cache_operator));
        Self {
            data_manager,
            fetch_manager,
        }
    }

    async fn get_data(
        &'a self,
        starting_version: u64,
        ending_version: u64,
        max_batch_size: usize,
    ) -> Option<Vec<Transaction>> {
        while starting_version >= self.data_manager.read().await.end_version {
            info!("Reached head, wait...");
            let num_transactions = self
                .fetch_manager
                .fetching_latest_data_task
                .read()
                .await
                .as_ref()
                .unwrap()
                .clone()
                .await;

            info!("Done waiting, {num_transactions}");
        }

        loop {
            let data_manager = self.data_manager.read().await;

            if starting_version < data_manager.start_version {
                info!(
                    "requested_version: {starting_version}, oldest available version: {}",
                    data_manager.start_version
                );
                return None;
            }

            let start_index = starting_version as usize % data_manager.num_slots;

            if data_manager.data[start_index].is_none() {
                drop(data_manager);
                self.fetch_manager
                    .fetch_past_data(starting_version)
                    .await
                    .await;
                continue;
            }

            let mut total_bytes = 0;
            let mut version = starting_version;
            let ending_version = ending_version.min(data_manager.end_version);

            if let Some(_) = data_manager.data[version as usize % data_manager.num_slots].as_ref() {
                let mut result = Vec::new();
                while version < ending_version {
                    if let Some(transaction) =
                        data_manager.data[version as usize % data_manager.num_slots].as_ref()
                    {
                        result.push(transaction.as_ref().clone());
                        version += 1;
                    } else {
                        break;
                    }
                }
                info!("version {} is sent", version - 1);
                return Some(result);
            } else {
                unreachable!("Data cannot be None.");
            }
        }
    }
}

pub struct DataService<'a> {
    cache_operator: CacheOperator<redis::aio::ConnectionManager>,
    in_memory_cache: InMemoryCache<'a>,
    known_latest_version: AtomicU64,
}

impl<'a> DataService<'a> {
    pub fn new(mut cache_operator: CacheOperator<redis::aio::ConnectionManager>) -> Self {
        let known_latest_version = futures::executor::block_on(cache_operator.get_latest_version())
            .unwrap()
            .unwrap();
        Self {
            cache_operator: cache_operator.clone(),
            in_memory_cache: InMemoryCache::new(
                cache_operator,
                known_latest_version,
                NUM_SLOTS,
                SIZE_LIMIT,
            ),
            known_latest_version: AtomicU64::new(known_latest_version),
        }
    }

    pub fn run(
        &'a self,
        mut handler_rx: Receiver<(
            GetTransactionsRequest,
            Sender<Result<TransactionsResponse, Status>>,
        )>,
    ) {
        tokio_scoped::scope(|scope| {
            scope.spawn(async move {
                let _ = self
                    .in_memory_cache
                    .fetch_manager
                    .continuously_fetch_latest_data()
                    .await;
            });
            scope.spawn(async move {
                loop {
                    let result = self.cache_operator.clone().get_latest_version().await;
                    if let Ok(Some(known_latest_version)) = result {
                        self.set_known_latest_version(known_latest_version);
                    } else {
                        error!("Failed to fetch known latest version: {result:?}.");
                    }
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
            });
            while let Some((request, response_sender)) = handler_rx.blocking_recv() {
                let known_latest_version = self.get_known_latest_version();
                let starting_version = request.starting_version.unwrap_or(known_latest_version);

                info!("Received request: {request:?}.");
                if starting_version > known_latest_version + 10000 {
                    let err = Err(Status::failed_precondition(
                        "starting_version cannot be set to a far future version.",
                    ));
                    info!("Client error: {err:?}.");
                    let _ = response_sender.blocking_send(err);
                    continue;
                }

                let max_batch_size = if let Some(batch_size) = request.batch_size {
                    batch_size as usize
                } else {
                    DEFAULT_MAX_BATCH_SIZE
                };

                let ending_version = request
                    .transactions_count
                    .map(|count| starting_version + count);

                scope.spawn(async move {
                    self.start_streaming(
                        starting_version,
                        ending_version,
                        max_batch_size,
                        response_sender,
                    )
                    .await
                });
            }
        });
    }

    async fn start_streaming(
        &'a self,
        starting_version: u64,
        ending_version: Option<u64>,
        max_batch_size: usize,
        response_sender: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
    ) {
        info!("Start streaming, starting_version: {starting_version}, ending_version: {ending_version:?}.");
        let mut next_version = starting_version;
        let ending_version = ending_version.unwrap_or(u64::MAX);
        loop {
            if next_version >= ending_version {
                break;
            }
            let known_latest_version = self.get_known_latest_version();
            if next_version > known_latest_version {
                info!("next_version {next_version} is larger than known_latest_version {known_latest_version}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            if let Some(transactions) = self
                .in_memory_cache
                .get_data(next_version, ending_version, max_batch_size)
                .await
            {
                next_version += transactions.len() as u64;
                let response = TransactionsResponse {
                    transactions,
                    // TODO(grao): Fix chain id.
                    chain_id: Some(0),
                };
                if let Err(e) = response_sender.send(Ok(response)).await {
                    info!("Client dropped.");
                    break;
                }
            } else {
                let err = Err(Status::not_found("Requested data is too old."));
                info!("Client error: {err:?}.");
                let _ = response_sender.send(err).await;
                break;
            }
        }
    }

    fn get_known_latest_version(&self) -> u64 {
        self.known_latest_version.load(Ordering::SeqCst)
    }

    fn set_known_latest_version(&self, version: u64) {
        self.known_latest_version.store(version, Ordering::SeqCst);
        //info!("Updated known_latest_version to {version}.");
    }
}
