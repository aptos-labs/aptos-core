// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    indexer::transaction_processor::TransactionProcessor,
    processors::default_processor::DefaultTransactionProcessor,
};
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, PooledConnection},
};
use std::sync::Arc;

pub type PgPool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PgDbPool = Arc<PgPool>;
pub type PgPoolConnection = PooledConnection<ConnectionManager<PgConnection>>;

use aptos_logger::{error, info};
use aptos_moving_average::MovingAverage;
use aptos_protos::{
    datastream::v1::{
        indexer_stream_client::IndexerStreamClient, raw_datastream_response::Response,
        stream_status::StatusType, RawDatastreamRequest,
    },
    transaction::testing1::v1::Transaction as TransactionProto,
};
use futures::StreamExt;
use prost::Message;

// Will replace these with yaml config
fn get_datastream_service_address() -> String {
    std::env::var("GRPC_ADDRESS").expect("GRPC_ADDRESS is required.")
}

fn get_postgres_connection_string() -> String {
    std::env::var("DATABASE_URI").expect("DATABASE_URI is required.")
}

fn get_starting_version() -> u64 {
    std::env::var("STARTING_VERSION")
        .expect("STARTING_VERSION is required.")
        .parse::<u64>()
        .unwrap()
}

pub struct Worker {
    pub db_pool: PgDbPool,
    pub datastream_service_address: String,
    pub postgres_uri: String,
}

impl Worker {
    pub async fn new() -> Self {
        let postgres_uri = get_postgres_connection_string();
        let manager = ConnectionManager::<PgConnection>::new(postgres_uri.clone());
        let pg_pool = PgPool::builder().build(manager).map(Arc::new);
        Self {
            db_pool: pg_pool.unwrap(),
            datastream_service_address: get_datastream_service_address(),
            postgres_uri,
        }
    }

    pub async fn run(&self) {
        let mut ma = MovingAverage::new(10_000);
        // Connecting once to the RPC client.
        // TODO: What happens if the connection is lost?
        let mut rpc_client =
            match IndexerStreamClient::connect(self.datastream_service_address.clone()).await {
                Ok(client) => client,
                Err(e) => {
                    panic!(
                        "[Datastream Worker] Error connecting to grpc_stream: {}. Error: {:?}",
                        self.datastream_service_address.clone(),
                        e,
                    );
                },
            };
        info!(
            "Connected to GRPC endpoint at {}.",
            self.datastream_service_address.clone(),
        );
        let starting_version = get_starting_version();
        let request = tonic::Request::new(RawDatastreamRequest {
            // Loads from the recent successful starting version.
            starting_version,
            transactions_count: None,
        });
        let response = rpc_client.raw_datastream(request).await.unwrap();
        let mut resp_stream = response.into_inner();

        // TODO: Add code for other processors
        let default_transaction_processor: Arc<dyn TransactionProcessor> =
            Arc::new(DefaultTransactionProcessor::new(self.db_pool.clone()));
        loop {
            if let Some(received) = resp_stream.next().await {
                let received = match received {
                    Ok(r) => r,
                    Err(e) => {
                        // If the connection is lost, reconnect.
                        error!(
                            "[Datastream Worker] Error receiving datastream response: {}",
                            e
                        );
                        break;
                    },
                };
                let transactions = match received.response.unwrap() {
                    Response::Status(status) => {
                        match status.r#type() {
                            StatusType::Init => {
                                // Ensure that init matches the correct starting version
                                if status.start_version != starting_version {
                                    panic!("[Datastream Indexer] Init version mismatch. Expected: {}, got {}", starting_version, status.start_version);
                                }
                                continue;
                            },
                            StatusType::BatchEnd => {
                                // Update current version
                                // Wait will we actually have batch end? If not we should just remove this
                                continue;
                            },
                            _ => {
                                // There might be protobuf inconsistency between server and client.
                                // Panic to block running.
                                panic!("[Datastream Indexer] Unknown RawDatastreamResponse status type.");
                            },
                        }
                    },
                    Response::Data(data) => data
                        .transactions
                        .into_iter()
                        .map(|e| {
                            let txn_raw = base64::decode(e.encoded_proto_data).unwrap();
                            TransactionProto::decode(&*txn_raw).unwrap()
                        })
                        .collect::<Vec<TransactionProto>>(),
                };
                if transactions.is_empty() {
                    info!("[Datastream Indexer] Channel is empty now.");
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                    continue;
                }
                let start_version = transactions.as_slice().first().unwrap().version;
                let end_version = transactions.as_slice().last().unwrap().version;
                let batch_size = transactions.len();
                match default_transaction_processor
                    .process_transactions(transactions, start_version, end_version)
                    .await
                {
                    Ok(result) => {
                        default_transaction_processor
                            .update_last_processed_version(result.end_version)
                            .await
                            .unwrap();
                    },
                    Err(error) => {
                        panic!(
                            "[Datastream Indexer] Error processing transactions. Versions {} to {}. Error: {:?}",
                            start_version,
                            end_version,
                            error
                        );
                    },
                };
                ma.tick_now(batch_size as u64);
                info!(
                    start_version = start_version,
                    batch_size = batch_size,
                    tps = (ma.avg() * 1000.0) as u64,
                    "[Datastream Indexer] Batch inserted.",
                );
            } else {
                // If there is no next item in stream, sleep for 1 second.
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }
}
