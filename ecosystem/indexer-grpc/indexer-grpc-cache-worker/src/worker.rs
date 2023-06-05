// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    ERROR_COUNT, LATEST_PROCESSED_VERSION, PROCESSED_BATCH_SIZE, PROCESSED_LATENCY_IN_SECS,
    PROCESSED_VERSIONS_COUNT,
};
use aptos_indexer_grpc_utils::{
    cache_operator::CacheOperator,
    config::IndexerGrpcFileStoreConfig,
    create_grpc_client,
    file_store_operator::{
        FileStoreMetadata, FileStoreOperator, GcsFileStoreOperator, LocalFileStoreOperator,
    },
    time_diff_since_pb_timestamp_in_secs,
};
use aptos_moving_average::MovingAverage;
use aptos_protos::internal::fullnode::v1::{
    stream_status::StatusType, transactions_from_node_response::Response,
    GetTransactionsFromNodeRequest, TransactionsFromNodeResponse,
};
use futures::{self, StreamExt};
use prost::Message;
use tracing::{error, info};

type ChainID = u32;
type StartingVersion = u64;

pub struct Worker {
    /// Redis client.
    redis_client: redis::Client,
    /// Fullnode grpc address.
    fullnode_grpc_address: String,
    /// File store config
    file_store: IndexerGrpcFileStoreConfig,
}

/// GRPC data status enum is to identify the data frame.
/// One stream may contain multiple batches and one batch may contain multiple data chunks.
pub(crate) enum GrpcDataStatus {
    /// Ok status with processed count.
    /// Each batch may contain multiple data chunks(like 1000 transactions).
    /// These data chunks may be out of order.
    ChunkDataOk {
        start_version: u64,
        num_of_transactions: u64,
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
        fullnode_grpc_address: String,
        redis_main_instance_address: String,
        file_store: IndexerGrpcFileStoreConfig,
    ) -> Self {
        let redis_client = redis::Client::open(format!("redis://{}", redis_main_instance_address))
            .expect("Create redis client failed.");
        Self {
            redis_client,
            file_store,
            fullnode_grpc_address: format!("http://{}", fullnode_grpc_address),
        }
    }

    /// The main loop of the worker is:
    /// 1. Fetch metadata from file store; if not present, exit after 1 minute.
    /// 2. Start the streaming RPC with version from file store or 0 if not present.
    /// 3. Handle the INIT frame from TransactionsFromNodeResponse:
    ///    * If metadata is not present and cache is empty, start from 0.
    ///    * If metadata is not present and cache is not empty, crash.
    ///    * If metadata is present, start from file store version.
    /// 4. Process the streaming response.
    pub async fn run(&mut self) {
        // Re-connect if lost.
        loop {
            let conn = self
                .redis_client
                .get_tokio_connection()
                .await
                .expect("Get redis connection failed.");

            let mut rpc_client = create_grpc_client(self.fullnode_grpc_address.clone()).await;

            // 1. Fetch metadata.
            let file_store_operator: Box<dyn FileStoreOperator> = match &self.file_store {
                IndexerGrpcFileStoreConfig::GcsFileStore(gcs_file_store) => Box::new(
                    GcsFileStoreOperator::new(gcs_file_store.gcs_file_store_bucket_name.clone()),
                ),
                IndexerGrpcFileStoreConfig::LocalFileStore(local_file_store) => Box::new(
                    LocalFileStoreOperator::new(local_file_store.local_file_store_path.clone()),
                ),
            };

            file_store_operator.verify_storage_bucket_existence().await;
            let starting_version = file_store_operator
                .get_starting_version()
                .await
                .unwrap_or(0);

            let file_store_metadata = file_store_operator.get_file_store_metadata().await;

            // 2. Start streaming RPC.
            let request = tonic::Request::new(GetTransactionsFromNodeRequest {
                starting_version: Some(starting_version),
                ..Default::default()
            });

            let response = rpc_client
                .get_transactions_from_node(request)
                .await
                .unwrap();

            // 3&4. Infinite streaming until error happens. Either stream ends or worker crashes.
            process_streaming_response(conn, file_store_metadata, response.into_inner()).await;
        }
    }
}

async fn process_transactions_from_node_response(
    response: TransactionsFromNodeResponse,
    cache_operator: &mut CacheOperator<redis::aio::Connection>,
) -> anyhow::Result<GrpcDataStatus> {
    match response.response.unwrap() {
        Response::Status(status) => {
            match StatusType::from_i32(status.r#type).expect("[Indexer Cache] Invalid status type.")
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
            let start_version = data.transactions.first().unwrap().version;
            let first_transaction_pb_timestamp =
                data.transactions.first().unwrap().timestamp.clone();
            let transactions = data
                .transactions
                .into_iter()
                .map(|tx| {
                    let timestamp_in_seconds = match tx.timestamp {
                        Some(ref timestamp) => timestamp.seconds as u64,
                        None => 0,
                    };
                    let mut encoded_proto_data = vec![];
                    tx.encode(&mut encoded_proto_data)
                        .expect("Encode transaction failed.");
                    let base64_encoded_proto_data = base64::encode(encoded_proto_data);
                    (tx.version, base64_encoded_proto_data, timestamp_in_seconds)
                })
                .collect::<Vec<(u64, String, u64)>>();

            // Push to cache.
            match cache_operator.update_cache_transactions(transactions).await {
                Ok(_) => {},
                Err(e) => {
                    ERROR_COUNT
                        .with_label_values(&["failed_to_update_cache_version"])
                        .inc();
                    anyhow::bail!("Update cache with version failed: {}", e);
                },
            }
            if let Some(ref txn_time) = first_transaction_pb_timestamp {
                PROCESSED_LATENCY_IN_SECS.set(time_diff_since_pb_timestamp_in_secs(txn_time));
            }
            Ok(GrpcDataStatus::ChunkDataOk {
                start_version,
                num_of_transactions: transaction_len as u64,
            })
        },
    }
}

/// Setup the cache operator with init signal, includeing chain id and starting version from fullnode.
async fn setup_cache_with_init_signal(
    conn: redis::aio::Connection,
    init_signal: TransactionsFromNodeResponse,
) -> (
    CacheOperator<redis::aio::Connection>,
    ChainID,
    StartingVersion,
) {
    let (fullnode_chain_id, starting_version) =
        match init_signal.response.expect("Response type not exists.") {
            Response::Status(status_frame) => {
                match StatusType::from_i32(status_frame.r#type).expect("Invalid status type.") {
                    StatusType::Init => (init_signal.chain_id, status_frame.start_version),
                    _ => {
                        panic!("[Indexer Cache] Streaming error: first frame is not INIT signal.");
                    },
                }
            },
            _ => {
                panic!("[Indexer Cache] Streaming error: first frame is not siganl frame.");
            },
        };

    let mut cache_operator = CacheOperator::new(conn);
    cache_operator.cache_setup_if_needed().await;
    cache_operator
        .update_or_verify_chain_id(fullnode_chain_id as u64)
        .await
        .expect("[Indexer Cache] Chain id mismatch between cache and fullnode.");

    (cache_operator, fullnode_chain_id, starting_version)
}

// Infinite streaming processing. Retry if error happens; crash if fatal.
async fn process_streaming_response(
    conn: redis::aio::Connection,
    file_store_metadata: Option<FileStoreMetadata>,
    mut resp_stream: impl futures_core::Stream<Item = Result<TransactionsFromNodeResponse, tonic::Status>>
        + std::marker::Unpin,
) {
    let mut tps_calculator = MovingAverage::new(10_000);
    let mut transaction_count = 0;
    // 3. Set up the cache operator with init signal.
    let init_signal = match resp_stream.next().await {
        Some(Ok(r)) => r,
        _ => {
            panic!("[Indexer Cache] Streaming error: no response.");
        },
    };
    let (mut cache_operator, fullnode_chain_id, starting_version) =
        setup_cache_with_init_signal(conn, init_signal).await;
    // It's required to start the worker with the same version as file store.
    if let Some(file_store_metadata) = file_store_metadata {
        if file_store_metadata.version != starting_version {
            panic!("[Indexer Cache] File store version mismatch with fullnode.");
        }
        if file_store_metadata.chain_id != fullnode_chain_id as u64 {
            panic!("[Indexer Cache] Chain id mismatch between file store and fullnode.");
        }
    }
    let mut current_version = starting_version;

    // 4. Process the streaming response.
    while let Some(received) = resp_stream.next().await {
        let received: TransactionsFromNodeResponse = match received {
            Ok(r) => r,
            Err(err) => {
                error!("[Indexer Cache] Streaming error: {}", err);
                ERROR_COUNT.with_label_values(&["streaming_error"]).inc();
                break;
            },
        };

        if received.chain_id as u64 != fullnode_chain_id as u64 {
            panic!("[Indexer Cache] Chain id mismatch happens during data streaming.");
        }

        match process_transactions_from_node_response(received, &mut cache_operator).await {
            Ok(status) => match status {
                GrpcDataStatus::ChunkDataOk {
                    start_version,
                    num_of_transactions,
                } => {
                    current_version += num_of_transactions;
                    transaction_count += num_of_transactions;
                    tps_calculator.tick_now(num_of_transactions);

                    PROCESSED_VERSIONS_COUNT.inc_by(num_of_transactions);
                    LATEST_PROCESSED_VERSION.set(current_version as i64);
                    PROCESSED_BATCH_SIZE.set(num_of_transactions as i64);
                    info!(
                        start_version = start_version,
                        num_of_transactions = num_of_transactions,
                        "[Indexer Cache] Data chunk received.",
                    );
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
                    info!(
                        start_version = start_version,
                        num_of_transactions = num_of_transactions,
                        "[Indexer Cache] End signal received for current batch.",
                    );
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
                        .unwrap();
                    transaction_count = 0;
                    info!(
                        current_version = current_version,
                        chain_id = fullnode_chain_id,
                        tps = (tps_calculator.avg() * 1000.0) as u64,
                        "[Indexer Cache] Successfully process current batch."
                    );
                },
            },
            Err(e) => {
                error!(
                    "[Indexer Cache] Process transactions from fullnode failed: {}",
                    e
                );
                ERROR_COUNT.with_label_values(&["response_error"]).inc();
                break;
            },
        }
    }
}
