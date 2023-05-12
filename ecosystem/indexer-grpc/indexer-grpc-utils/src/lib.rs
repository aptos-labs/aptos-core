// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod cache_operator;
pub mod config;
pub mod constants;
pub mod file_store_operator;

use aptos_inspection_service::utils::get_encoded_metrics;
use aptos_protos::{
    internal::fullnode::v1::fullnode_data_client::FullnodeDataClient, util::timestamp::Timestamp,
};
use prometheus::TextEncoder;
use warp::{http::Response, Filter};

pub type GrpcClientType = FullnodeDataClient<tonic::transport::Channel>;

/// Create a gRPC client with exponential backoff.
pub async fn create_grpc_client(address: String) -> GrpcClientType {
    backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
        match FullnodeDataClient::connect(address.clone()).await {
            Ok(client) => {
                aptos_logger::info!(
                    address = address.clone(),
                    "[Indexer Cache] Connected to indexer gRPC server."
                );
                Ok(client)
            },
            Err(e) => {
                aptos_logger::error!(
                    address = address.clone(),
                    "[Indexer Cache] Failed to connect to indexer gRPC server: {}",
                    e
                );
                Err(backoff::Error::transient(e))
            },
        }
    })
    .await
    .unwrap()
}

// (Protobuf encoded transaction, version)
pub type EncodedTransactionWithVersion = (String, u64);
/// Build the EncodedTransactionWithVersion from the encoded transactions and starting version.
#[inline]
pub fn build_protobuf_encoded_transaction_wrappers(
    encoded_transactions: Vec<String>,
    starting_version: u64,
) -> Vec<EncodedTransactionWithVersion> {
    encoded_transactions
        .into_iter()
        .enumerate()
        .map(|(ind, encoded_transaction)| (encoded_transaction, starting_version + ind as u64))
        .collect()
}

fn metrics() -> Vec<u8> {
    get_encoded_metrics(TextEncoder)
}

pub async fn register_probes_and_metrics_handler(port: u16) {
    let readiness = warp::path("readiness")
        .map(move || warp::reply::with_status("ready", warp::http::StatusCode::OK));
    let metrics_endpoint = warp::path("metrics").map(|| {
        Response::builder()
            .header("Content-Type", "text/plain")
            .body(metrics())
    });
    warp::serve(readiness.or(metrics_endpoint))
        .run(([0, 0, 0, 0], port))
        .await;
}

pub fn time_diff_since_pb_timestamp_in_secs(timestamp: &Timestamp) -> f64 {
    let current_timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("SystemTime before UNIX EPOCH!")
        .as_secs_f64();
    let transaction_time = timestamp.seconds as f64 + timestamp.nanos as f64 * 1e-9;
    current_timestamp - transaction_time
}
