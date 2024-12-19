// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{config::LiveDataServiceConfig, connection_manager::ConnectionManager};
use aptos_protos::{
    indexer::v1::{GetTransactionsRequest, TransactionsResponse},
    transaction::v1::Transaction,
};
use futures::future::{BoxFuture, FutureExt, Shared};
use prost::Message;
use std::{sync::Arc, time::Duration};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    RwLock,
};
use tonic::{Request, Status};
use tracing::{info, trace};
use uuid::Uuid;

static MAX_BYTES_PER_BATCH: usize = 20 * (1 << 20);

struct DataClient {
    connection_manager: Arc<ConnectionManager>,
}

impl DataClient {
    fn new(connection_manager: Arc<ConnectionManager>) -> Self {
        Self { connection_manager }
    }

    async fn fetch_transactions(&self, starting_version: u64) -> Vec<Transaction> {
        trace!("Fetching transactions from GrpcManager, start_version: {starting_version}.");

        let request = GetTransactionsRequest {
            starting_version: Some(starting_version),
            transactions_count: None,
            batch_size: None,
        };
        loop {
            let mut client = self
                .connection_manager
                .get_grpc_manager_client_for_request();
            let response = client.get_transactions(request).await;
            if let Ok(response) = response {
                return response.into_inner().transactions;
            }
            // TODO(grao): Error handling.
        }
    }
}

type FetchTask<'a> = Shared<BoxFuture<'a, usize>>;

struct FetchManager<'a> {
    data_manager: Arc<RwLock<DataManager>>,
    data_client: Arc<DataClient>,
    fetching_latest_data_task: RwLock<Option<FetchTask<'a>>>,
}

impl<'a> FetchManager<'a> {
    fn new(
        data_manager: Arc<RwLock<DataManager>>,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        Self {
            data_manager,
            data_client: Arc::new(DataClient::new(connection_manager)),
            fetching_latest_data_task: RwLock::new(None),
        }
    }

    async fn fetch_past_data(&self, version: u64) -> usize {
        Self::fetch_and_update_cache(self.data_client.clone(), self.data_manager.clone(), version)
            .await
    }

    async fn fetch_and_update_cache(
        data_client: Arc<DataClient>,
        data_manager: Arc<RwLock<DataManager>>,
        version: u64,
    ) -> usize {
        let transactions = data_client.fetch_transactions(version).await;
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

    size_limit_bytes: usize,
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
            size_limit_bytes,
            eviction_target: size_limit_bytes,
            total_size: 0,
            num_slots,
        }
    }

    fn update_data(&mut self, start_version: u64, transactions: Vec<Transaction>) {
        let end_version = start_version + transactions.len() as u64;

        trace!(
            "Updating data for {} transactions in range [{start_version}, {end_version}).",
            transactions.len(),
        );
        if start_version > self.end_version {
            // TODO(grao): unexpected
            return;
        }

        if end_version <= self.start_version {
            // TODO(grao): Log and counter.
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

        if self.total_size >= self.size_limit_bytes {
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

    async fn get_data(
        &'a self,
        starting_version: u64,
        ending_version: u64,
        max_num_transactions_per_batch: usize,
        max_bytes_per_batch: usize,
    ) -> Option<(Vec<Transaction>, usize)> {
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

            let start_index = starting_version as usize % data_manager.num_slots;

            if data_manager.data[start_index].is_none() {
                drop(data_manager);
                self.fetch_manager.fetch_past_data(starting_version).await;
                continue;
            }

            let mut total_bytes = 0;
            let mut version = starting_version;
            let ending_version = ending_version.min(data_manager.end_version);

            if let Some(_) = data_manager.data[version as usize % data_manager.num_slots].as_ref() {
                let mut result = Vec::new();
                while version < ending_version
                    && total_bytes < max_bytes_per_batch
                    && result.len() < max_num_transactions_per_batch
                {
                    if let Some(transaction) =
                        data_manager.data[version as usize % data_manager.num_slots].as_ref()
                    {
                        // NOTE: We allow 1 more txn beyond the size limit here, for simplicity.
                        total_bytes += transaction.encoded_len();
                        result.push(transaction.as_ref().clone());
                        version += 1;
                    } else {
                        break;
                    }
                }
                trace!("Data was sent from cache, last version: {}.", version - 1);
                return Some((result, total_bytes));
            } else {
                unreachable!("Data cannot be None.");
            }
        }
    }
}

pub struct LiveDataService<'a> {
    chain_id: u64,
    in_memory_cache: InMemoryCache<'a>,
    connection_manager: Arc<ConnectionManager>,
}

impl<'a> LiveDataService<'a> {
    pub fn new(
        chain_id: u64,
        config: LiveDataServiceConfig,
        connection_manager: Arc<ConnectionManager>,
    ) -> Self {
        let known_latest_version = connection_manager.known_latest_version();
        Self {
            chain_id,
            connection_manager: connection_manager.clone(),
            in_memory_cache: InMemoryCache::new(
                connection_manager,
                known_latest_version,
                config.num_slots,
                config.size_limit_bytes,
            ),
        }
    }

    pub fn run(
        &'a self,
        mut handler_rx: Receiver<(
            Request<GetTransactionsRequest>,
            Sender<Result<TransactionsResponse, Status>>,
        )>,
    ) {
        info!("Running LiveDataService...");
        tokio_scoped::scope(|scope| {
            scope.spawn(async move {
                let _ = self
                    .in_memory_cache
                    .fetch_manager
                    .continuously_fetch_latest_data()
                    .await;
            });
            while let Some((request, response_sender)) = handler_rx.blocking_recv() {
                // TODO(grao): Store request metadata.
                let request = request.into_inner();
                let id = Uuid::new_v4().to_string();
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

                let max_num_transactions_per_batch = if let Some(batch_size) = request.batch_size {
                    batch_size as usize
                } else {
                    10000
                };

                let ending_version = request
                    .transactions_count
                    .map(|count| starting_version + count);

                scope.spawn(async move {
                    self.start_streaming(
                        id,
                        starting_version,
                        ending_version,
                        max_num_transactions_per_batch,
                        MAX_BYTES_PER_BATCH,
                        response_sender,
                    )
                    .await
                });
            }
        });
    }

    pub(crate) fn get_connection_manager(&self) -> &ConnectionManager {
        &self.connection_manager
    }

    pub(crate) async fn get_min_servable_version(&self) -> u64 {
        self.in_memory_cache.data_manager.read().await.start_version
    }

    async fn start_streaming(
        &'a self,
        id: String,
        starting_version: u64,
        ending_version: Option<u64>,
        max_num_transactions_per_batch: usize,
        max_bytes_per_batch: usize,
        response_sender: tokio::sync::mpsc::Sender<Result<TransactionsResponse, Status>>,
    ) {
        info!(stream_id = id, "Start streaming, starting_version: {starting_version}, ending_version: {ending_version:?}.");
        self.connection_manager
            .insert_active_stream(&id, starting_version, ending_version);
        let mut next_version = starting_version;
        let mut size_bytes = 0;
        let ending_version = ending_version.unwrap_or(u64::MAX);
        loop {
            if next_version >= ending_version {
                break;
            }
            self.connection_manager
                .update_stream_progress(&id, next_version, size_bytes);
            let known_latest_version = self.get_known_latest_version();
            if next_version > known_latest_version {
                info!(stream_id = id, "next_version {next_version} is larger than known_latest_version {known_latest_version}");
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            if let Some((transactions, batch_size_bytes)) = self
                .in_memory_cache
                .get_data(
                    next_version,
                    ending_version,
                    max_num_transactions_per_batch,
                    max_bytes_per_batch,
                )
                .await
            {
                next_version += transactions.len() as u64;
                size_bytes += batch_size_bytes as u64;
                let response = TransactionsResponse {
                    transactions,
                    chain_id: Some(self.chain_id),
                };
                if let Err(_) = response_sender.send(Ok(response)).await {
                    info!(stream_id = id, "Client dropped.");
                    break;
                }
            } else {
                let err = Err(Status::not_found("Requested data is too old."));
                info!(stream_id = id, "Client error: {err:?}.");
                let _ = response_sender.send(err).await;
                break;
            }
        }

        self.connection_manager
            .update_stream_progress(&id, next_version, size_bytes);
        self.connection_manager.remove_active_stream(&id);
    }

    fn get_known_latest_version(&self) -> u64 {
        self.connection_manager.known_latest_version()
    }
}
