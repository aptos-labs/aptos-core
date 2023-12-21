// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{ERROR_COUNT, WAIT_FOR_FILE_STORE_COUNTER};
use anyhow::{bail, Context, Result};
use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator,
    config::IndexerGrpcFileStoreConfig,
    counters::{log_grpc_step, IndexerGrpcStep},
    create_grpc_client,
    file_store_operator::FileStoreOperator,
    storage_format::{CacheEntry, CacheEntryBuilder, FileStoreMetadata, StorageFormat},
    types::RedisUrl,
};
use aptos_protos::internal::fullnode::v1::{
    stream_status::StatusType, transactions_from_node_response::Response,
    GetTransactionsFromNodeRequest, TransactionsFromNodeResponse,
};
use core::panic;
use futures::{self, StreamExt};
use prost::Message;
use tracing::{error, info};
use url::Url;

type ChainID = u32;
type StartingVersion = u64;

const FILE_STORE_VERSIONS_RESERVED: u64 = 150_000;
// Cache worker will wait if filestore is behind by
// `FILE_STORE_VERSIONS_RESERVED` versions
// This is pinging the cache so it's OK to be more aggressive
const CACHE_WORKER_WAIT_FOR_FILE_STORE_MS: u64 = 100;
// This is the time we wait for the file store to be ready. It should only be
// kicked off when there's no metadata in the file store.
const FILE_STORE_METADATA_WAIT_MS: u64 = 2000;

const SERVICE_TYPE: &str = "cache_worker";

pub struct Worker {
    /// Redis client.
    redis_client: redis::Client,
    /// Fullnode grpc address.
    fullnode_grpc_address: Url,
    /// File store config
    file_store: IndexerGrpcFileStoreConfig,
    /// Cache storage format.
    cache_storage_format: StorageFormat,
}

/// GRPC data status enum is to identify the data frame.
/// One stream may contain multiple batches and one batch may contain multiple data chunks.
pub(crate) enum _GrpcDataStatus {
    /// Ok status with processed count.
    /// Each batch may contain multiple data chunks(like 1000 transactions).
    /// These data chunks may be out of order.
    ChunkDataOk { num_of_transactions: u64 },
    /// Init signal received with start version of current stream.
    /// No two `Init` signals will be sent in the same stream.
    StreamInit(u64),
    /// End signal received with batch end version(inclusive).
    /// Start version and its number of transactions are included for current batch.
    BatchEnd {
        start_version: u64,
        num_of_transactions: u64,
    },
}

/// These are direct cache operation requests from the cache worker.
enum CacheTaskItem {
    /// Update the cache with the given transactions.
    UpdateCacheTransactions((Vec<Vec<u8>>, u64)),
    /// Update the cache with the given latest version.
    UpdateCacheLatestVersion((u64, u64)),
}

impl Worker {
    pub async fn new(
        fullnode_grpc_address: Url,
        redis_main_instance_address: RedisUrl,
        file_store: IndexerGrpcFileStoreConfig,
        enable_cache_compression: bool,
    ) -> Result<Self> {
        let cache_storage_format = if enable_cache_compression {
            StorageFormat::GzipCompressedProto
        } else {
            StorageFormat::Base64UncompressedProto
        };
        let redis_client = redis::Client::open(redis_main_instance_address.0.clone())
            .with_context(|| {
                format!(
                    "[Indexer Cache] Failed to create redis client for {}",
                    redis_main_instance_address
                )
            })?;
        Ok(Self {
            redis_client,
            file_store,
            fullnode_grpc_address,
            cache_storage_format,
        })
    }

    /// The main loop of the worker is:
    /// 1. Fetch metadata from file store; if not present, exit after 1 minute.
    /// 2. Start the streaming RPC with version from file store or 0 if not present.
    /// 3. Handle the INIT frame from TransactionsFromNodeResponse:
    ///    * If metadata is not present and cache is empty, start from 0.
    ///    * If metadata is not present and cache is not empty, crash.
    ///    * If metadata is present, start from file store version.
    /// 4. Process the streaming response.
    /// TODO: Use the ! return type when it is stable.
    /// TODO: Rewrite logic to actually conform to this description
    pub async fn run(&mut self) -> Result<()> {
        // Re-connect if lost.
        loop {
            let conn = self
                .redis_client
                .get_tokio_connection_manager()
                .await
                .context("Get redis connection failed.")?;
            let mut rpc_client = create_grpc_client(self.fullnode_grpc_address.clone()).await;
            // 1. Fetch metadata.
            let file_store_operator: Box<dyn FileStoreOperator> = self.file_store.create();
            // TODO: move chain id check somewhere around here
            // This ensures that metadata is created before we start the cache worker
            let mut starting_version = file_store_operator.get_latest_version().await;
            while starting_version.is_none() {
                starting_version = file_store_operator.get_latest_version().await;
                tracing::warn!(
                    "[Indexer Cache] File store metadata not found. Waiting for {} ms.",
                    FILE_STORE_METADATA_WAIT_MS
                );
                tokio::time::sleep(std::time::Duration::from_millis(
                    FILE_STORE_METADATA_WAIT_MS,
                ))
                .await;
            }

            // There's a guarantee at this point that starting_version is not null
            let starting_version = starting_version.unwrap();

            let file_store_metadata = file_store_operator.get_file_store_metadata().await.unwrap();

            tracing::info!(
                service_type = SERVICE_TYPE,
                "[Indexer Cache] Starting cache worker with version {}",
                starting_version
            );

            // 2. Start streaming RPC.
            let request = tonic::Request::new(GetTransactionsFromNodeRequest {
                starting_version: Some(starting_version),
                ..Default::default()
            });

            let response = rpc_client
                .get_transactions_from_node(request)
                .await
                .with_context(|| {
                    format!(
                        "Failed to get transactions from node at starting version {}",
                        starting_version
                    )
                })?;
            info!(
                service_type = SERVICE_TYPE,
                "[Indexer Cache] Streaming RPC started."
            );
            // 3&4. Infinite streaming until error happens. Either stream ends or worker crashes.
            process_streaming_response(
                conn,
                self.cache_storage_format,
                file_store_metadata,
                response.into_inner(),
            )
            .await?;

            info!(
                service_type = SERVICE_TYPE,
                "[Indexer Cache] Streaming RPC ended."
            );
        }
    }
}

async fn process_transactions_from_node_response(
    response: CacheTaskItem,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
) -> Result<()> {
    match response {
        CacheTaskItem::UpdateCacheLatestVersion((num_of_transactions, version)) => cache_operator
            .update_cache_latest_version(num_of_transactions, version)
            .await
            .context("update failure"),
        CacheTaskItem::UpdateCacheTransactions((data, first_version)) => {
            // Push to cache.
            cache_operator
                .update_cache_transactions_with_bytes(data, first_version)
                .await
                .context("update failure")
        },
    }
}

//// Setup the cache operator with init signal, includeing chain id and starting version from fullnode.
async fn verify_fullnode_init_signal(
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    init_signal: TransactionsFromNodeResponse,
    file_store_metadata: FileStoreMetadata,
) -> Result<(ChainID, StartingVersion)> {
    let (fullnode_chain_id, starting_version) = match init_signal
        .response
        .expect("[Indexer Cache] Response type does not exist.")
    {
        Response::Status(status_frame) => {
            match StatusType::try_from(status_frame.r#type)
                .expect("[Indexer Cache] Invalid status type.")
            {
                StatusType::Init => (init_signal.chain_id, status_frame.start_version),
                _ => {
                    bail!("[Indexer Cache] Streaming error: first frame is not INIT signal.");
                },
            }
        },
        _ => {
            bail!("[Indexer Cache] Streaming error: first frame is not siganl frame.");
        },
    };

    // Guaranteed that chain id is here at this point because we already ensure that fileworker did the set up
    let chain_id = cache_operator.get_chain_id().await?.unwrap();
    if chain_id != fullnode_chain_id as u64 {
        bail!("[Indexer Cache] Chain ID mismatch between fullnode init signal and cache.");
    }

    // It's required to start the worker with the same version as file store.
    if file_store_metadata.version != starting_version {
        bail!("[Indexer Cache] Starting version mismatch between filestore metadata and fullnode init signal.");
    }
    if file_store_metadata.chain_id != fullnode_chain_id as u64 {
        bail!("[Indexer Cache] Chain id mismatch between filestore metadata and fullnode.");
    }

    Ok((fullnode_chain_id, starting_version))
}

/// Infinite streaming processing. Retry if error happens; crash if fatal.
async fn process_streaming_response(
    conn: redis::aio::ConnectionManager,
    cache_storage_format: StorageFormat,
    file_store_metadata: FileStoreMetadata,
    mut resp_stream: impl futures_core::Stream<Item = Result<TransactionsFromNodeResponse, tonic::Status>>
        + std::marker::Unpin,
) -> Result<()> {
    // 3. Set up the cache operator with init signal.
    let init_signal = match resp_stream.next().await {
        Some(Ok(r)) => r,
        _ => {
            bail!("[Indexer Cache] Streaming error: no response.");
        },
    };
    let mut cache_operator = CacheOperator::new(conn, cache_storage_format);

    let (fullnode_chain_id, starting_version) =
        verify_fullnode_init_signal(&mut cache_operator, init_signal, file_store_metadata)
            .await
            .context("[Indexer Cache] Failed to verify init signal")?;

    let current_version = starting_version;
    let batch_start_time = std::time::Instant::now();

    let (tx, mut rx) = tokio::sync::mpsc::channel::<CacheTaskItem>(1000);

    // process the cache items.
    tokio::spawn({
        let mut cache_operator_clone = cache_operator.clone();
        async move {
            while let Some(item) = rx.recv().await {
                process_transactions_from_node_response(item, &mut cache_operator_clone)
                    .await
                    .expect("process failure");
            }
            panic!("Cache worker channel closed.");
        }
    });
    // 4. Process the streaming response.
    loop {
        let received = match resp_stream.next().await {
            Some(r) => r,
            _ => {
                error!(
                    service_type = SERVICE_TYPE,
                    "[Indexer Cache] Streaming error: no response."
                );
                ERROR_COUNT.with_label_values(&["streaming_error"]).inc();
                break;
            },
        };
        let received: TransactionsFromNodeResponse = match received {
            Ok(r) => r,
            Err(err) => {
                error!(
                    service_type = SERVICE_TYPE,
                    "[Indexer Cache] Streaming error: {}", err
                );
                ERROR_COUNT.with_label_values(&["streaming_error"]).inc();
                break;
            },
        };

        if received.chain_id as u64 != fullnode_chain_id as u64 {
            panic!("[Indexer Cache] Chain id mismatch happens during data streaming.");
        }

        let size_in_bytes = received.encoded_len();

        match received.response.unwrap() {
            Response::Status(status) => {
                match StatusType::try_from(status.r#type)
                    .expect("[Indexer Cache] Invalid status type.")
                {
                    StatusType::Init => panic!("Init signal received twice."),
                    StatusType::BatchEnd => {
                        let start_version = status.start_version;
                        let num_of_transactions = status
                            .end_version
                            .expect("TransactionsFromNodeResponse status end_version is None")
                            + 1
                            - start_version;
                        tx.send(CacheTaskItem::UpdateCacheLatestVersion((
                            num_of_transactions,
                            start_version,
                        )))
                        .await
                        .expect("Failed to send cache task item.");
                    },
                    StatusType::Unspecified => unreachable!("Unspecified status type."),
                }
            },
            Response::Data(transactions) => {
                let transactions = transactions.transactions;
                let first_transaction = transactions
                    .first()
                    .context("There were unexpectedly no transactions in the response")?;
                let first_transaction_version = first_transaction.version;
                let last_transaction = transactions
                    .last()
                    .context("There were unexpectedly no transactions in the response")?;
                let last_transaction_version = last_transaction.version;
                let start_version = first_transaction.version;
                let first_transaction_pb_timestamp = first_transaction.timestamp.clone();
                let last_transaction_pb_timestamp = last_transaction.timestamp.clone();

                log_grpc_step(
                    SERVICE_TYPE,
                    IndexerGrpcStep::CacheWorkerReceivedTxns,
                    Some(start_version as i64),
                    Some(last_transaction_version as i64),
                    first_transaction_pb_timestamp.as_ref(),
                    last_transaction_pb_timestamp.as_ref(),
                    Some(batch_start_time.elapsed().as_secs_f64()),
                    Some(size_in_bytes),
                    Some((last_transaction_version + 1 - first_transaction_version) as i64),
                    None,
                );
                let cache_encoding_start_time = std::time::Instant::now();
                let cache_entries: Vec<Vec<u8>> = transactions
                    .into_iter()
                    .map(|t| {
                        let cache_entry: CacheEntry =
                            CacheEntryBuilder::new(t, cache_storage_format)
                                .try_into()
                                .expect("Failed to convert transaction to cache entry");
                        cache_entry.into_inner()
                    })
                    .collect();

                log_grpc_step(
                    SERVICE_TYPE,
                    IndexerGrpcStep::CacheWorkerTxnsProcessed,
                    Some(first_transaction_version as i64),
                    Some(last_transaction_version as i64),
                    first_transaction_pb_timestamp.as_ref(),
                    last_transaction_pb_timestamp.as_ref(),
                    Some(cache_encoding_start_time.elapsed().as_secs_f64()),
                    Some(size_in_bytes),
                    Some((last_transaction_version + 1 - first_transaction_version) as i64),
                    None,
                );
                // Push to cache.
                tx.send(CacheTaskItem::UpdateCacheTransactions((
                    cache_entries,
                    start_version,
                )))
                .await
                .expect("task send failure");
                // Channel size.
                info!("[Cache worker] Channel size: {}", 100 - tx.capacity());
            },
        }

        // Check if the file store isn't too far away
        loop {
            let file_store_version = cache_operator
                .get_file_store_latest_version()
                .await?
                .unwrap();
            if file_store_version + FILE_STORE_VERSIONS_RESERVED < current_version {
                tokio::time::sleep(std::time::Duration::from_millis(
                    CACHE_WORKER_WAIT_FOR_FILE_STORE_MS,
                ))
                .await;
                tracing::warn!(
                    current_version = current_version,
                    file_store_version = file_store_version,
                    "[Indexer Cache] File store version is behind current version too much."
                );
                WAIT_FOR_FILE_STORE_COUNTER.inc();
            } else {
                // File store is up to date, continue cache update.
                break;
            }
        }
    }

    // It is expected that we get to this point, the upstream server disconnects
    // clients after 5 minutes.
    Ok(())
}
