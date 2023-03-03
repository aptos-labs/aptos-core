// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
/// Common configuration for Indexer GRPC Store.
use std::{fs::File, io::Read, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcConfig {
    /// GRPC address of Indexer, e.g. "127.0.0.1:50051".
    pub fullnode_grpc_address: Option<String>,
    // GRPC address listening to, e.g., "0.0.0.0:50051"
    pub data_service_grpc_listen_address: Option<String>,
    /// Redis address, e.g. "127.0.0.1:6379".
    pub redis_address: String,
    /// File store bucket name, e.g., "indexer-grpc-file-store".
    pub file_store_bucket_name: String,
    /// Health check port.
    pub health_check_port: u16,
}

impl IndexerGrpcConfig {
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
