// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::CacheConfig,
    metadata_manager::MetadataManager,
    metrics::{
        CACHE_END_VERSION, CACHE_SIZE, CACHE_START_VERSION, FILE_STORE_VERSION_IN_CACHE,
        IS_FILE_STORE_LAGGING, MAX_CACHE_SIZE, TARGET_CACHE_SIZE, TIMER,
    },
};
use anyhow::{bail, ensure, Result};
use velor_indexer_grpc_utils::{
    config::IndexerGrpcFileStoreConfig, file_store_operator_v2::file_store_reader::FileStoreReader,
};
use velor_protos::{
    internal::fullnode::v1::{
        transactions_from_node_response::Response, GetTransactionsFromNodeRequest,
    },
    transaction::v1::Transaction,
};
use futures::StreamExt;
use prost::Message;
use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};
use tokio::sync::{mpsc::channel, oneshot::Receiver, RwLock, RwLockReadGuard};
use tracing::{debug, error, info, trace, warn};

struct Cache {
    start_version: u64,
    file_store_version: AtomicU64,
    transactions: VecDeque<Transaction>,
    cache_size: usize,

    max_cache_size: usize,
    target_cache_size: usize,
}

impl Cache {
    fn new(cache_config: CacheConfig, file_store_version: u64) -> Self {
        MAX_CACHE_SIZE.set(cache_config.max_cache_size as i64);
        TARGET_CACHE_SIZE.set(cache_config.target_cache_size as i64);
        CACHE_START_VERSION.set(file_store_version as i64);
        CACHE_END_VERSION.set(file_store_version as i64);

        Self {
            start_version: file_store_version,
            file_store_version: AtomicU64::new(file_store_version),
            transactions: VecDeque::new(),
            cache_size: 0,
            max_cache_size: cache_config.max_cache_size,
            target_cache_size: cache_config.target_cache_size,
        }
    }

    // NOTE: This will only gc data up to the file store version.
    fn maybe_gc(&mut self) -> bool {
        if self.cache_size <= self.max_cache_size {
            return true;
        }

        while self.start_version < self.file_store_version.load(Ordering::SeqCst)
            && self.cache_size > self.target_cache_size
        {
            let transaction = self.transactions.pop_front().unwrap();
            self.cache_size -= transaction.encoded_len();
            self.start_version += 1;
        }

        CACHE_SIZE.set(self.cache_size as i64);
        CACHE_START_VERSION.set(self.start_version as i64);

        self.cache_size <= self.max_cache_size
    }

    fn put_transactions(&mut self, transactions: Vec<Transaction>) {
        self.cache_size += transactions
            .iter()
            .map(|transaction| transaction.encoded_len())
            .sum::<usize>();
        self.transactions.extend(transactions);
        CACHE_SIZE.set(self.cache_size as i64);
        CACHE_END_VERSION.set(self.start_version as i64 + self.transactions.len() as i64);
    }

    fn get_transactions(
        &self,
        start_version: u64,
        max_size_bytes: usize,
        update_file_store_version: bool,
    ) -> Vec<Transaction> {
        if !update_file_store_version {
            trace!(
            "Requesting version {start_version} from cache, update_file_store_version = {update_file_store_version}.",
        );
            trace!(
                "Current data range in cache: [{}, {}).",
                self.start_version,
                self.start_version + self.transactions.len() as u64
            );
        }
        if start_version < self.start_version {
            return vec![];
        }

        let mut transactions = vec![];
        let mut size_bytes = 0;
        for transaction in self
            .transactions
            .iter()
            .skip((start_version - self.start_version) as usize)
        {
            size_bytes += transaction.encoded_len();
            transactions.push(transaction.clone());
            if size_bytes > max_size_bytes {
                // Note: We choose to not pop the last transaction here, so the size could be
                // slightly larger than the `max_size_bytes`. This is fine.
                break;
            }
        }
        if update_file_store_version {
            let old_version = self
                .file_store_version
                .fetch_add(transactions.len() as u64, Ordering::SeqCst);
            let new_version = old_version + transactions.len() as u64;
            FILE_STORE_VERSION_IN_CACHE.set(new_version as i64);
            info!("Updated file_store_version in cache to {new_version}.");
        } else {
            trace!(
                "Returned {} transactions from Cache, total {size_bytes} bytes.",
                transactions.len()
            );
        }
        transactions
    }
}

pub(crate) struct DataManager {
    // TODO(grao): Putting a big lock for now, if necessary we can explore some solution with less
    // locking / lock-free.
    cache: RwLock<Cache>,
    file_store_reader: FileStoreReader,
    metadata_manager: Arc<MetadataManager>,
}

impl DataManager {
    pub(crate) async fn new(
        chain_id: u64,
        file_store_config: IndexerGrpcFileStoreConfig,
        cache_config: CacheConfig,
        metadata_manager: Arc<MetadataManager>,
    ) -> Self {
        let file_store = file_store_config.create_filestore().await;
        let file_store_reader = FileStoreReader::new(chain_id, file_store).await;
        let file_store_version = file_store_reader.get_latest_version().await.unwrap();
        Self {
            cache: RwLock::new(Cache::new(cache_config, file_store_version)),
            file_store_reader,
            metadata_manager,
        }
    }

    pub(crate) async fn start(
        &self,
        is_master: bool,
        file_store_uploader_recover_rx: Receiver<()>,
    ) {
        let watch_file_store_version = !is_master;

        if is_master {
            // For master, we need to wait for the FileStoreUploader to finish the recover to get
            // the true file_store_version.
            info!("Waiting for FileStoreUploader recovering.");
            match file_store_uploader_recover_rx.await {
                Ok(_) => {},
                Err(_) => panic!("Should not happen!"),
            };
            let cache = self.cache.read().await;
            self.update_file_store_version_in_cache(&cache, /*version_can_go_backward=*/ true)
                .await;
        }

        info!("Starting DataManager loop.");

        'out: loop {
            let _timer = TIMER
                .with_label_values(&["data_manager_main_loop"])
                .start_timer();
            let cache = self.cache.read().await;
            if watch_file_store_version {
                self.update_file_store_version_in_cache(
                    &cache, /*version_can_go_backward=*/ false,
                )
                .await;
            }
            let request = GetTransactionsFromNodeRequest {
                starting_version: Some(cache.start_version + cache.transactions.len() as u64),
                transactions_count: Some(100000),
            };
            drop(cache);

            debug!(
                "Requesting transactions from fullnodes, starting_version: {}.",
                request.starting_version.unwrap()
            );
            let (address, mut fullnode_client) =
                self.metadata_manager.get_fullnode_for_request(&request);
            let response = fullnode_client.get_transactions_from_node(request).await;
            if response.is_err() {
                warn!(
                    "Error when getting transactions from fullnode ({address}): {}",
                    response.err().unwrap()
                );
                tokio::time::sleep(Duration::from_millis(100)).await;
                continue;
            }

            let mut response = response.unwrap().into_inner();
            while let Some(response_item) = response.next().await {
                loop {
                    if self.cache.write().await.maybe_gc() {
                        IS_FILE_STORE_LAGGING.set(0);
                        break;
                    }
                    IS_FILE_STORE_LAGGING.set(1);
                    // If file store is lagging, we are not inserting more data.
                    let cache = self.cache.read().await;
                    warn!("Filestore is lagging behind, cache is full [{}, {}), known_latest_version ({}).",
                          cache.start_version,
                          cache.start_version + cache.transactions.len() as u64,
                          self.metadata_manager.get_known_latest_version());
                    tokio::time::sleep(Duration::from_millis(100)).await;
                    if watch_file_store_version {
                        self.update_file_store_version_in_cache(
                            &cache, /*version_can_go_backward=*/ false,
                        )
                        .await;
                    }
                }
                match response_item {
                    Ok(r) => {
                        if let Some(response) = r.response {
                            match response {
                                Response::Data(data) => {
                                    self.cache.write().await.put_transactions(data.transactions);
                                },
                                Response::Status(_) => continue,
                            }
                        } else {
                            warn!("Error when getting transactions from fullnode: no data.");
                            continue 'out;
                        }
                    },
                    Err(e) => {
                        warn!("Error when getting transactions from fullnode: {}", e);
                        continue 'out;
                    },
                }
            }
        }
    }

    pub(crate) fn lagging(&self, cache_next_version: u64) -> bool {
        // TODO(grao): Need a better way, we can use the information in the metadata_manager.
        cache_next_version + 20000 < self.metadata_manager.get_known_latest_version()
    }

    pub(crate) async fn get_transactions(
        &self,
        start_version: u64,
        max_size_bytes_from_cache: usize,
    ) -> Result<Vec<Transaction>> {
        let cache = self.cache.read().await;
        let cache_start_version = cache.start_version;
        let cache_next_version = cache_start_version + cache.transactions.len() as u64;
        drop(cache);

        if start_version >= cache_start_version {
            if start_version >= cache_next_version {
                // If lagging, try to fetch the data from FN.
                if self.lagging(cache_next_version) {
                    debug!("GrpcManager is lagging, getting data from FN, requested_version: {start_version}, cache_next_version: {cache_next_version}.");
                    let request = GetTransactionsFromNodeRequest {
                        starting_version: Some(start_version),
                        transactions_count: Some(5000),
                    };

                    let (_, mut fullnode_client) =
                        self.metadata_manager.get_fullnode_for_request(&request);
                    let response = fullnode_client.get_transactions_from_node(request).await?;
                    let mut response = response.into_inner();
                    while let Some(Ok(response_item)) = response.next().await {
                        if let Some(response) = response_item.response {
                            match response {
                                Response::Data(data) => {
                                    return Ok(data.transactions);
                                },
                                Response::Status(_) => continue,
                            }
                        }
                    }
                }

                // Let client side to retry.
                return Ok(vec![]);
            }
            // NOTE: We are not holding the read lock for cache here. Therefore it's possible that
            // the start_version becomes older than the cache.start_version. In that case the
            // following function will return empty return, and let the client to retry.
            return Ok(self
                .get_transactions_from_cache(
                    start_version,
                    max_size_bytes_from_cache,
                    /*update_file_store_version=*/ false,
                )
                .await);
        }

        let (tx, mut rx) = channel(1);
        self.file_store_reader
            .get_transaction_batch(
                start_version,
                /*retries=*/ 3,
                /*max_files=*/ Some(1),
                /*filter=*/ None,
                /*ending_version=*/ None,
                tx,
            )
            .await;

        if let Some((transactions, _, _, range)) = rx.recv().await {
            debug!(
                "Transactions returned from filestore: [{}, {}].",
                range.0, range.1
            );
            let first_version = transactions.first().unwrap().version;
            ensure!(
                first_version == start_version,
                "Version doesn't match, something is wrong."
            );
            Ok(transactions)
        } else {
            let error_msg = "Failed to fetch transactions from filestore, either filestore is not available, or data is corrupted.";
            // TODO(grao): Consider downgrade this to warn! if this happens too frequently when
            // filestore is unavailable.
            error!(error_msg);
            bail!(error_msg);
        }
    }

    pub(crate) async fn get_transactions_from_cache(
        &self,
        start_version: u64,
        max_size: usize,
        update_file_store_version: bool,
    ) -> Vec<Transaction> {
        self.cache
            .read()
            .await
            .get_transactions(start_version, max_size, update_file_store_version)
    }

    pub(crate) async fn get_file_store_version(&self) -> u64 {
        self.file_store_reader.get_latest_version().await.unwrap()
    }

    pub(crate) async fn cache_stats(&self) -> String {
        let cache = self.cache.read().await;
        let len = cache.transactions.len() as u64;
        format!(
            "cache version: [{}, {}), # of txns: {}, file store version: {}, cache size: {}",
            cache.start_version,
            cache.start_version + len,
            len,
            cache.file_store_version.load(Ordering::SeqCst),
            cache.cache_size
        )
    }

    async fn update_file_store_version_in_cache(
        &self,
        cache: &RwLockReadGuard<'_, Cache>,
        version_can_go_backward: bool,
    ) {
        let file_store_version = self.file_store_reader.get_latest_version().await;
        if let Some(file_store_version) = file_store_version {
            let file_store_version_before_update = cache
                .file_store_version
                .fetch_max(file_store_version, Ordering::SeqCst);
            FILE_STORE_VERSION_IN_CACHE.set(file_store_version as i64);
            info!("Updated file_store_version in cache to {file_store_version}.");
            if !version_can_go_backward && file_store_version_before_update > file_store_version {
                panic!("File store version is going backward, data might be corrupted. {file_store_version_before_update} v.s. {file_store_version}");
            };
        }
    }
}
