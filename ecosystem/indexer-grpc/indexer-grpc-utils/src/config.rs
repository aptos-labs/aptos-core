// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
/// Common configuration for Indexer GRPC Store.
use std::{fs::File, io::Read, path::PathBuf};

/// Indexer GRPC configuration. This is to configure the Indexer GRPC server.
/// This configuration is intende to share between Indexer GRPC(cache, file store, etc.).
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

/// Indexer GRPC Processor configuration. This is to configure the processors,
/// e.g., `default_processor` to process data from Indexer GRPC.
#[derive(Clone, Debug, Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields)]
pub struct IndexerGrpcProcessorConfig {
    /// Address of Indexer grpc, e.g. "34.70.26.67:50051".
    pub indexer_grpc_address: String,
    /// Postgres connection string, e.g. "postgres://postgres@localhost/indexer_v3".
    pub postgres_connection_string: String,
    /// Number of concurrent processing tasks, e.g., tasks to receive, transform and save data into postgres.
    pub number_concurrent_processing_tasks: usize,
    /// Name of the processor, e.g., "default_processor".
    pub processor_name: String,
    /// Aptos Name Service address, only used by `token_processor`.
    pub ans_address: Option<String>,
    /// Health check port.
    pub health_check_port: u16,
    /// Starting version.
    pub starting_version: Option<u64>,
}

impl IndexerGrpcConfig {
    pub fn load(path: PathBuf) -> Result<Self, anyhow::Error> {
        load::<Self>(path)
    }
}

impl IndexerGrpcProcessorConfig {
    pub fn load(path: PathBuf) -> Result<Self, anyhow::Error> {
        load::<Self>(path)
    }
}

pub fn load<T: for<'de> serde::Deserialize<'de>>(path: PathBuf) -> Result<T, anyhow::Error> {
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

    serde_yaml::from_str::<T>(&contents).map_err(|e| {
        anyhow::anyhow!(
            "Unable to read yaml {}. Error: {}",
            path.to_str().unwrap(),
            e
        )
    })
}
