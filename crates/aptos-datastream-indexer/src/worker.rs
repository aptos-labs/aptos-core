// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::models::transactions::TransactionModel;
use diesel::{
    pg::PgConnection,
    r2d2::{ConnectionManager, PooledConnection},
    result::Error,
    RunQueryDsl,
};
use std::sync::Arc;

pub type PgPool = diesel::r2d2::Pool<ConnectionManager<PgConnection>>;
pub type PgDbPool = Arc<PgPool>;
pub type PgPoolConnection = PooledConnection<ConnectionManager<PgConnection>>;

use aptos_logger::{error, info};
use aptos_protos::{
    datastream::v1::{self as datastream, RawDatastreamRequest},
    transaction::v1::Transaction as TransactionProto,
};
use futures::StreamExt;
use moving_average::MovingAverage;
use prost::Message;
use tokio::sync::mpsc;

fn get_datastream_service_address() -> String {
    std::env::var("APTOS_DATASTREAM_SERVICE_ADDRESS_VAR")
        .expect("DATASTREAM_SERVICE_ADDRESS is required.")
}

fn get_postgres_connection_string() -> String {
    std::env::var("APTOS_POSTGRES_CONNECTION_STRING_VAR")
        .expect("POSTGRES_CONNECTION_STRING is required.")
}

fn get_starting_version() -> u64 {
    std::env::var("STARTING_VERSION")
        .expect("STARTING_VERSION is required.")
        .parse::<u64>()
        .unwrap()
}

fn get_chain_id() -> u32 {
    std::env::var("CHAIN_ID")
        .expect("CHAIN_ID is required.")
        .parse::<u32>()
        .unwrap()
}

pub struct Worker {
    pub db_pool: Arc<PgDbPool>,
    pub datastream_service_address: String,
    pub postgres_uri: String,
}

impl Worker {
    pub async fn new() -> Self {
        let postgres_uri = get_postgres_connection_string();
        let manager = ConnectionManager::<PgConnection>::new(postgres_uri.clone());
        let pg_pool = PgPool::builder().build(manager).map(Arc::new);
        Self {
            db_pool: Arc::new(pg_pool.unwrap()),
            datastream_service_address: get_datastream_service_address(),
            postgres_uri,
        }
    }

    pub async fn run(&self) {
        let (tx, mut rx) = mpsc::channel::<TransactionProto>(50_000);
        let mut ma = MovingAverage::new(10_000);
        let mut conn = get_conn(self.db_pool.clone());
        // Re-connect if lost.
        tokio::spawn(async move {
            // Nothing speicial.
            let batch_size = 5000;
            loop {
                let mut current_transactions = vec![];
                for _ in 0..batch_size {
                    let transaction = rx.recv().await.unwrap();
                    current_transactions.push(transaction);
                }

                // nothing speicial.
                let vec_transaction_model = current_transactions
                    .iter()
                    .map(TransactionModel::from_transaction)
                    .collect::<Vec<TransactionModel>>();
                insert_to_db(&mut conn, vec_transaction_model).expect("inertion failed");
                ma.tick_now(5000);
                info!(
                    starting_version = current_transactions.as_slice().first().unwrap().version,
                    batch_size = 5000,
                    tps = (ma.avg() * 1000.0) as u64,
                    "[Datastream Indexer] Batch inserted.",
                );
            }
        });

        loop {
            let mut rpc_client =
                match datastream::indexer_stream_client::IndexerStreamClient::connect(format!(
                    "http://{}:50051",
                    self.datastream_service_address,
                ))
                .await
                {
                    Ok(client) => client,
                    Err(e) => {
                        error!(
                            "[Datasteram Worker]Error connecting to indexer address {}, port {}",
                            self.datastream_service_address, self.postgres_uri
                        );
                        panic!("[Datastream Worker] Error connecting to indexer: {}", e);
                    },
                };
            let request = tonic::Request::new(datastream::RawDatastreamRequest {
                // Loads from the recent successful starting version.
                starting_version: get_starting_version(),
                chain_id: get_chain_id(),
                output_batch_size: 100,
                ..RawDatastreamRequest::default()
            });

            let response = rpc_client.raw_datastream(request).await.unwrap();
            let mut resp_stream = response.into_inner();
            let mut init_signal_received = false;

            while let Some(received) = resp_stream.next().await {
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
                match received.response.unwrap() {
                    datastream::raw_datastream_response::Response::Status(status) => {
                        match status.r#type {
                            0 => {
                                if init_signal_received {
                                    error!("[Datastream Indexer] No signal is expected; panic.");
                                    panic!("[Datastream Indexer] No signal is expected; panic.");
                                } else {
                                    // The first signal is the initialization signal.
                                    init_signal_received = true;
                                }
                            },
                            1 => {
                                // No BATCH_END signal is expected.
                                error!("[Datastream Indexer] No signal is expected; panic.");
                                panic!("[Datastream Indexer] No signal is expected; panic.");
                            },
                            _ => {
                                // There might be protobuf inconsistency between server and client.
                                // Panic to block running.
                                panic!("[Datastream Worker] Unknown RawDatastreamResponse status type.");
                            },
                        }
                    },
                    datastream::raw_datastream_response::Response::Data(data) => {
                        let transaction_sender = tx.clone();

                        let transactions: Vec<TransactionProto> = data
                            .transactions
                            .into_iter()
                            .map(|e| {
                                let txn_raw = base64::decode(e.encoded_proto_data).unwrap();
                                TransactionProto::decode(&*txn_raw).unwrap()
                            })
                            .collect();

                        for txn in transactions {
                            transaction_sender.send(txn).await.unwrap();
                        }
                    },
                };
            }
        }
    }
}

fn get_conn(pool: Arc<PgDbPool>) -> PgPoolConnection {
    loop {
        match pool.get() {
            Ok(conn) => {
                return conn;
            },
            Err(err) => {
                error!("Error getting connection from pool: {}", err);
            },
        };
    }
}
fn insert_transactions(
    conn: &mut PgConnection,
    txns: &[TransactionModel],
) -> Result<(), diesel::result::Error> {
    use crate::schema::transactions::dsl::*;
    diesel::insert_into(crate::schema::transactions::table)
        .values(txns)
        .on_conflict(version)
        .do_nothing()
        .execute(conn)
        .expect("insertion failed");
    Ok(())
}

fn insert_to_db(
    conn: &mut PgPoolConnection,
    txns: Vec<TransactionModel>,
) -> Result<(), diesel::result::Error> {
    match conn
        .build_transaction()
        .read_write()
        .run::<_, Error, _>(|pg_conn| {
            insert_transactions(pg_conn, &txns)?;
            Ok(())
        }) {
        Ok(_) => Ok(()),
        Err(_) => Ok(()),
    }
}
