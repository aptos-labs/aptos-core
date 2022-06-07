// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::utils;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

/// Port selected RocksDB options for tuning underlying rocksdb instance of AptosDB.
/// see <https://github.com/facebook/rocksdb/blob/master/include/rocksdb/options.h>
/// for detailed explanations.
#[derive(Copy, Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RocksdbConfig {
    pub max_open_files: i32,
    pub max_total_wal_size: u64,
}

impl Default for RocksdbConfig {
    fn default() -> Self {
        Self {
            // Set max_open_files to 10k instead of -1 to avoid keep-growing memory in accordance
            // with the number of files.
            max_open_files: 10_000,
            // For now we set the max total WAL size to be 1G. This config can be useful when column
            // families are updated at non-uniform frequencies.
            #[allow(clippy::integer_arithmetic)] // TODO: remove once clippy lint fixed
            max_total_wal_size: 1u64 << 30,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageConfig {
    pub address: SocketAddr,
    pub backup_service_address: SocketAddr,
    pub dir: PathBuf,
    pub grpc_max_receive_len: Option<i32>,
    pub storage_pruner_config: StoragePrunerConfig,
    #[serde(skip)]
    data_dir: PathBuf,
    /// Read, Write, Connect timeout for network operations in milliseconds
    pub timeout_ms: u64,
    /// Rocksdb-specific configurations
    pub rocksdb_config: RocksdbConfig,
}

pub const NO_OP_STORAGE_PRUNER_CONFIG: StoragePrunerConfig = StoragePrunerConfig {
    state_store_prune_window: None,
    ledger_prune_window: None,
    pruning_batch_size: 10_000,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StoragePrunerConfig {
    /// None disables pruning. The size of the window should be calculated based on disk space
    /// availability and system TPS.
    pub state_store_prune_window: Option<u64>,
    /// This is the default pruning window for any other store except for state store. State store
    /// being big in size, we might want to configure a smaller window for state store vs other
    /// store.
    pub ledger_prune_window: Option<u64>,
    /// Batch size of the versions to be sent to the pruner - this is to avoid slowdown due to
    /// issuing too many DB calls and batch prune instead.
    pub pruning_batch_size: usize,
}

impl StoragePrunerConfig {
    pub fn new(
        state_store_prune_window: Option<u64>,
        ledger_store_prune_window: Option<u64>,
        pruning_batch_size: usize,
    ) -> Self {
        StoragePrunerConfig {
            state_store_prune_window,
            ledger_prune_window: ledger_store_prune_window,
            pruning_batch_size,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> StorageConfig {
        StorageConfig {
            address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6666),
            backup_service_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6186),
            dir: PathBuf::from("db"),
            grpc_max_receive_len: Some(100_000_000),
            // The prune window must at least out live a RPC request because its sub requests are
            // to return a consistent view of the DB at exactly same version. Considering a few
            // thousand TPS we are potentially going to achieve, and a few minutes a consistent view
            // of the DB might require, 10k (TPS)  * 100 (seconds)  =  1 Million might be a
            // conservatively safe minimal prune window. It'll take a few Gigabytes of disk space
            // depending on the size of an average account blob.
            storage_pruner_config: StoragePrunerConfig {
                state_store_prune_window: Some(10_000_000),
                ledger_prune_window: Some(10_000_000),
                pruning_batch_size: 500,
            },
            data_dir: PathBuf::from("/opt/aptos/data"),
            // Default read/write/connection timeout, in milliseconds
            timeout_ms: 30_000,
            rocksdb_config: RocksdbConfig::default(),
        }
    }
}

impl StorageConfig {
    pub fn dir(&self) -> PathBuf {
        if self.dir.is_relative() {
            self.data_dir.join(&self.dir)
        } else {
            self.dir.clone()
        }
    }

    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.data_dir = data_dir;
    }

    pub fn randomize_ports(&mut self) {
        self.address.set_port(utils::get_available_port());
        self.backup_service_address
            .set_port(utils::get_available_port());
    }
}
