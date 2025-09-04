// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{ERROR_COUNT, WAIT_FOR_FILE_STORE_COUNTER};
use anyhow::{bail, Context, Result};
use velor_indexer_grpc_utils::{
    cache_operator::CacheOperator,
    compression_util::{FileStoreMetadata, StorageFormat},
    config::IndexerGrpcFileStoreConfig,
    counters::{log_grpc_step, IndexerGrpcStep},
    create_grpc_client,
    file_store_operator::FileStoreOperator,
    types::RedisUrl,
};
use velor_moving_average::MovingAverage;
use velor_protos::internal::fullnode::v1::{
    stream_status::StatusType, transactions_from_node_response::Response,
    GetTransactionsFromNodeRequest, TransactionsFromNodeResponse,
};
use futures::{self, future::join_all, StreamExt};
use prost::Message;
use tokio::task::JoinHandle;
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
pub(crate) enum GrpcDataStatus {
    /// Ok status with processed count.
    /// Each batch may contain multiple data chunks(like 1000 transactions).
    /// These data chunks may be out of order.
    ChunkDataOk {
        num_of_transactions: u64,
        task: tokio::task::JoinHandle<anyhow::Result<()>>,
    },
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

impl Worker {
    pub async fn new(
        fullnode_grpc_address: Url,
        redis_main_instance_address: RedisUrl,
        file_store: IndexerGrpcFileStoreConfig,
        enable_cache_compression: bool,
    ) -> Result<Self> {
        let cache_storage_format = if enable_cache_compression {
            StorageFormat::Lz4CompressedProto
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
    response: TransactionsFromNodeResponse,
    cache_operator: &mut CacheOperator<redis::aio::ConnectionManager>,
    download_start_time: std::time::Instant,
) -> Result<GrpcDataStatus> {
    let size_in_bytes = response.encoded_len();
    match response.response.unwrap() {
        Response::Status(status) => {
            match StatusType::try_from(status.r#type).expect("[Indexer Cache] Invalid status type.")
            {
                StatusType::Init => Ok(GrpcDataStatus::StreamInit(status.start_version)),
                StatusType::BatchEnd => {
                    let start_version = status.start_version;
                    let num_of_transactions = status
                        .end_version
                        .expect("TransactionsFromNodeResponse status end_version is None")
                        - start_version
                        + 1;
                    Ok(GrpcDataStatus::BatchEnd {
                        start_version,
                        num_of_transactions,
                    })
                },
                StatusType::Unspecified => unreachable!("Unspecified status type."),
            }
        },
        Response::Data(data) => {
            let transaction_len = data.transactions.len();
            let data_download_duration_in_secs = download_start_time.elapsed().as_secs_f64();
            let mut cache_operator_clone = cache_operator.clone();
            let task: JoinHandle<anyhow::Result<()>> = tokio::spawn({
                let first_transaction = data
                    .transactions
                    .first()
                    .context("There were unexpectedly no transactions in the response")?;
                let first_transaction_version = first_transaction.version;
                let last_transaction = data
                    .transactions
                    .last()
                    .context("There were unexpectedly no transactions in the response")?;
                let last_transaction_version = last_transaction.version;
                let start_version = first_transaction.version;
                let first_transaction_pb_timestamp = first_transaction.timestamp;
                let last_transaction_pb_timestamp = last_transaction.timestamp;

                log_grpc_step(
                    SERVICE_TYPE,
                    IndexerGrpcStep::CacheWorkerReceivedTxns,
                    Some(start_version as i64),
                    Some(last_transaction_version as i64),
                    first_transaction_pb_timestamp.as_ref(),
                    last_transaction_pb_timestamp.as_ref(),
                    Some(data_download_duration_in_secs),
                    Some(size_in_bytes),
                    Some((last_transaction_version + 1 - first_transaction_version) as i64),
                    None,
                );

                let cache_update_start_time = std::time::Instant::now();

                async move {
                    // Push to cache.
                    match cache_operator_clone
                        .update_cache_transactions(data.transactions)
                        .await
                    {
                        Ok(_) => {
                            log_grpc_step(
                                SERVICE_TYPE,
                                IndexerGrpcStep::CacheWorkerTxnsProcessed,
                                Some(first_transaction_version as i64),
                                Some(last_transaction_version as i64),
                                first_transaction_pb_timestamp.as_ref(),
                                last_transaction_pb_timestamp.as_ref(),
                                Some(cache_update_start_time.elapsed().as_secs_f64()),
                                Some(size_in_bytes),
                                Some(
                                    (last_transaction_version + 1 - first_transaction_version)
                                        as i64,
                                ),
                                None,
                            );
                            Ok(())
                        },
                        Err(e) => {
                            ERROR_COUNT
                                .with_label_values(&["failed_to_update_cache_version"])
                                .inc();
                            bail!("Update cache with version failed: {}", e);
                        },
                    }
                }
            });

            Ok(GrpcDataStatus::ChunkDataOk {
                num_of_transactions: transaction_len as u64,
                task,
            })
        },
    }
}

// Setup the cache operator with init signal, including chain id and starting version from fullnode.
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
    let mut tps_calculator = MovingAverage::new(10_000);
    let mut transaction_count = 0;
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

    let mut current_version = starting_version;
    let mut batch_start_time = std::time::Instant::now();

    let mut tasks_to_run = vec![];
    // 4. Process the streaming response.
    loop {
        let download_start_time = std::time::Instant::now();
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
        // 10 batches doewnload + slowest processing& uploading task
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
        match process_transactions_from_node_response(
            received,
            &mut cache_operator,
            download_start_time,
        )
        .await
        {
            Ok(status) => match status {
                GrpcDataStatus::ChunkDataOk {
                    num_of_transactions,
                    task,
                } => {
                    current_version += num_of_transactions;
                    transaction_count += num_of_transactions;
                    tps_calculator.tick_now(num_of_transactions);

                    tasks_to_run.push(task);
                },
                GrpcDataStatus::StreamInit(new_version) => {
                    error!(
                        current_version = new_version,
                        "[Indexer Cache] Init signal received twice."
                    );
                    ERROR_COUNT.with_label_values(&["data_init_twice"]).inc();
                    break;
                },
                GrpcDataStatus::BatchEnd {
                    start_version,
                    num_of_transactions,
                } => {
                    // Handle the data multithreading.
                    let result = join_all(tasks_to_run).await;
                    if result
                        .iter()
                        .any(|r| (r.is_err() || r.as_ref().unwrap().is_err()))
                    {
                        error!(
                            start_version = start_version,
                            num_of_transactions = num_of_transactions,
                            "[Indexer Cache] Process transactions from fullnode failed."
                        );
                        ERROR_COUNT.with_label_values(&["response_error"]).inc();
                        panic!("Error happens when processing transactions from fullnode.");
                    }
                    // Cleanup.
                    tasks_to_run = vec![];
                    if current_version != start_version + num_of_transactions {
                        error!(
                            current_version = current_version,
                            actual_current_version = start_version + num_of_transactions,
                            "[Indexer Cache] End signal received with wrong version."
                        );
                        ERROR_COUNT
                            .with_label_values(&["data_end_wrong_version"])
                            .inc();
                        break;
                    }
                    cache_operator
                        .update_cache_latest_version(transaction_count, current_version)
                        .await
                        .context("Failed to update the latest version in the cache")?;
                    transaction_count = 0;

                    log_grpc_step(
                        SERVICE_TYPE,
                        IndexerGrpcStep::CacheWorkerBatchProcessed,
                        Some(start_version as i64),
                        Some((start_version + num_of_transactions - 1) as i64),
                        None,
                        None,
                        Some(batch_start_time.elapsed().as_secs_f64()),
                        Some(size_in_bytes),
                        Some(num_of_transactions as i64),
                        None,
                    );
                    batch_start_time = std::time::Instant::now();
                },
            },
            Err(e) => {
                error!(
                    start_version = current_version,
                    chain_id = fullnode_chain_id,
                    service_type = SERVICE_TYPE,
                    "[Indexer Cache] Process transactions from fullnode failed: {}",
                    e
                );
                ERROR_COUNT.with_label_values(&["response_error"]).inc();
                break;
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
