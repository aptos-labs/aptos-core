// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_protos::datastream::v1::{self as datastream};
use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

pub mod worker;

pub type GrpcClientType =
    datastream::indexer_stream_client::IndexerStreamClient<tonic::transport::Channel>;

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcCacheWorkerConfig {
    /// Indexer GRPC address, i.e., `127.0.0.1:50051`.
    pub indexer_address: String,

    /// Redis address, i.e., `127.0.0.1:6379`.
    pub redis_address: String,

    /// Chain ID
    pub chain_id: u32,

    /// Starting version; if not provided, will start from the latest version in the cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_version: Option<u64>,
}

impl IndexerGrpcCacheWorkerConfig {
    pub fn load(path: PathBuf) -> Result<Self, anyhow::Error> {
        let mut file = File::open(&path).map_err(|e| {
            anyhow::anyhow!(
                "Unable to open file {}. Error: {}",
                path.to_str().unwrap(),
                e
            )
        })?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).map_err(|e| {
            anyhow::anyhow!(
                "Unable to read file {}. Error: {}",
                path.to_str().unwrap(),
                e
            )
        })?;

        serde_yaml::from_str(&contents).map_err(|e| {
            anyhow::anyhow!(
                "Unable to read yaml {}. Error: {}",
                path.to_str().unwrap(),
                e
            )
        })
    }
}

// 9999-12-31 23:59:59. UTC.
pub const BASE_EXPIRATION_EPOCH_TIME_IN_SECONDS: u64 = 253_402_300_799;

/// Get the TTL in seconds for a given timestamp.
pub fn get_ttl_in_seconds(timestamp_in_seconds: u64) -> u64 {
    let current_time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    BASE_EXPIRATION_EPOCH_TIME_IN_SECONDS - (current_time - timestamp_in_seconds)
}

/// Create a gRPC client with exponential backoff.
pub async fn create_grpc_client(address: String) -> GrpcClientType {
    backoff::future::retry(backoff::ExponentialBackoff::default(), || async {
        Ok(
            datastream::indexer_stream_client::IndexerStreamClient::connect(address.clone())
                .await?,
        )
    })
    .await
    .unwrap()
}
