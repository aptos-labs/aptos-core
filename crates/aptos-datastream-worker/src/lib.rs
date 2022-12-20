// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Serialize};
use std::{fs::File, io::Read, path::PathBuf};

pub mod constants;
pub mod worker;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DatastreamWorkerConfig {
    /// Indexer GRPC address.
    pub indexer_address: String,

    /// Indexer GRPC port.
    pub indexer_port: u64,

    /// Chain ID
    pub chain_id: u64,

    /// Starting version
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub starting_version: Option<u64>,

    /// Number of workers for processing data. Default is 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_task_count: Option<u64>,

    /// Number of transactions received for each streaming Response. Default is 10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processor_batch_size: Option<u64>,

    /// Redis address. Default is 127.0.0.1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redis_address: Option<String>,

    /// Redis port. Default is 6379.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redis_port: Option<u64>,

    /// Output transaction batch size; default to 100.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_transaction_batch_size: Option<u64>,
}

impl DatastreamWorkerConfig {
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
