// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{config_sanitizer::ConfigSanitizer, node_config_loader::NodeType, Error, NodeConfig},
    utils,
};
use aptos_logger::warn;
use aptos_types::chain_id::ChainId;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::PathBuf,
};

// Lru cache will consume about 2G RAM based on this default value.
pub const DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD: usize = 1 << 13;

pub const BUFFERED_STATE_TARGET_ITEMS: usize = 100_000;

/// Port selected RocksDB options for tuning underlying rocksdb instance of AptosDB.
/// see <https://github.com/facebook/rocksdb/blob/master/include/rocksdb/options.h>
/// for detailed explanations.
#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RocksdbConfig {
    /// Maximum number of files open by RocksDB at one time
    pub max_open_files: i32,
    /// Maximum size of the RocksDB write ahead log (WAL)
    pub max_total_wal_size: u64,
    /// Maximum number of background threads for Rocks DB
    pub max_background_jobs: i32,
    /// Block cache size for Rocks DB
    pub block_cache_size: u64,
    /// Block size for Rocks DB
    pub block_size: u64,
    /// Whether cache index and filter blocks into block cache.
    pub cache_index_and_filter_blocks: bool,
}

impl Default for RocksdbConfig {
    fn default() -> Self {
        Self {
            // Allow db to close old sst files, saving memory.
            max_open_files: 5000,
            // For now we set the max total WAL size to be 1G. This config can be useful when column
            // families are updated at non-uniform frequencies.
            max_total_wal_size: 1u64 << 30,
            // This includes threads for flashing and compaction. Rocksdb will decide the # of
            // threads to use internally.
            max_background_jobs: 16,
            // Default block cache size is 8MB,
            block_cache_size: 8 * (1u64 << 20),
            // Default block cache size is 4KB,
            block_size: 4 * (1u64 << 10),
            // Whether cache index and filter blocks into block cache.
            cache_index_and_filter_blocks: false,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct RocksdbConfigs {
    pub ledger_db_config: RocksdbConfig,
    pub state_merkle_db_config: RocksdbConfig,
    // Note: Not ready for production use yet.
    pub use_sharded_state_merkle_db: bool,
    // Note: Not ready for production use yet.
    // TODO(grao): Add RocksdbConfig for individual DBs when necessary.
    pub split_ledger_db: bool,
    // Note: Not ready for production use yet.
    pub skip_index_and_usage: bool,
    pub state_kv_db_config: RocksdbConfig,
    pub index_db_config: RocksdbConfig,
}

impl Default for RocksdbConfigs {
    fn default() -> Self {
        Self {
            ledger_db_config: RocksdbConfig::default(),
            state_merkle_db_config: RocksdbConfig::default(),
            use_sharded_state_merkle_db: false,
            split_ledger_db: false,
            skip_index_and_usage: false,
            state_kv_db_config: RocksdbConfig::default(),
            index_db_config: RocksdbConfig {
                max_open_files: 1000,
                ..Default::default()
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StorageConfig {
    pub backup_service_address: SocketAddr,
    /// Top level directory to store the RocksDB
    pub dir: PathBuf,
    /// Storage pruning configuration
    pub storage_pruner_config: PrunerConfig,
    /// Subdirectory for storage in tests only
    #[serde(skip)]
    data_dir: PathBuf,
    /// AptosDB persists the state authentication structure off the critical path
    /// of transaction execution and batch up recent changes for performance. Once
    /// the number of buffered state updates exceeds this config, a dump of all
    /// buffered values into a snapshot is triggered. (Alternatively, if too many
    /// transactions have been processed since last dump, a new dump is processed
    /// as well.)
    pub buffered_state_target_items: usize,
    /// The max # of nodes for a lru cache shard.
    pub max_num_nodes_per_lru_cache_shard: usize,
    /// Rocksdb-specific configurations
    pub rocksdb_configs: RocksdbConfigs,
    /// Try to enable the internal indexer. The indexer expects to have seen all transactions
    /// since genesis. To recover operation after data loss, or to bootstrap a node in fast sync
    /// mode, the indexer db needs to be copied in from another node.
    pub enable_indexer: bool,
}

pub const NO_OP_STORAGE_PRUNER_CONFIG: PrunerConfig = PrunerConfig {
    ledger_pruner_config: LedgerPrunerConfig {
        enable: false,
        prune_window: 0,
        batch_size: 0,
        user_pruning_window_offset: 0,
    },
    state_merkle_pruner_config: StateMerklePrunerConfig {
        enable: false,
        prune_window: 0,
        batch_size: 0,
    },
    epoch_snapshot_pruner_config: EpochSnapshotPrunerConfig {
        enable: false,
        prune_window: 0,
        batch_size: 0,
    },
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct LedgerPrunerConfig {
    /// Boolean to enable/disable the ledger pruner. The ledger pruner is responsible for pruning
    /// everything else except for states (e.g. transactions, events etc.)
    pub enable: bool,
    /// This is the default pruning window for any other store except for state store. State store
    /// being big in size, we might want to configure a smaller window for state store vs other
    /// store.
    pub prune_window: u64,
    /// Batch size of the versions to be sent to the ledger pruner - this is to avoid slowdown due to
    /// issuing too many DB calls and batch prune instead. For ledger pruner, this means the number
    /// of versions to prune a time.
    pub batch_size: usize,
    /// The offset for user pruning window to adjust
    pub user_pruning_window_offset: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct StateMerklePrunerConfig {
    /// Boolean to enable/disable the state merkle pruner. The state merkle pruner is responsible
    /// for pruning state tree nodes.
    pub enable: bool,
    /// Window size in versions.
    pub prune_window: u64,
    /// Number of stale nodes to prune a time.
    pub batch_size: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct EpochSnapshotPrunerConfig {
    pub enable: bool,
    /// Window size in versions, but only the snapshots at epoch ending versions are kept, because
    /// other snapshots are pruned by the state merkle pruner.
    pub prune_window: u64,
    /// Number of stale nodes to prune a time.
    pub batch_size: usize,
}

// Config for the epoch ending state pruner is actually in the same format as the state merkle
// pruner, but it has it's own type hence separate default values. This converts it to the same
// type, to use the same pruner implementation (but parameterized on the stale node index DB schema).
impl From<EpochSnapshotPrunerConfig> for StateMerklePrunerConfig {
    fn from(config: EpochSnapshotPrunerConfig) -> Self {
        Self {
            enable: config.enable,
            prune_window: config.prune_window,
            batch_size: config.batch_size,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, Default)]
#[serde(default, deny_unknown_fields)]
pub struct PrunerConfig {
    pub ledger_pruner_config: LedgerPrunerConfig,
    pub state_merkle_pruner_config: StateMerklePrunerConfig,
    pub epoch_snapshot_pruner_config: EpochSnapshotPrunerConfig,
}

impl Default for LedgerPrunerConfig {
    fn default() -> Self {
        LedgerPrunerConfig {
            enable: true,
            // This assumes we have 1T disk, minus the space needed by state merkle db and the
            // overhead in storage.
            prune_window: 150_000_000,
            batch_size: 500,
            user_pruning_window_offset: 200_000,
        }
    }
}

impl Default for StateMerklePrunerConfig {
    fn default() -> Self {
        StateMerklePrunerConfig {
            enable: true,
            // This allows a block / chunk being executed to have access to a non-latest state tree.
            // It needs to be greater than the number of versions the state committing thread is
            // able to commit during the execution of the block / chunk. If the bad case indeed
            // happens due to this being too small, a node restart should recover it.
            // Still, defaulting to 1M to be super safe.
            prune_window: 1_000_000,
            // A 10k transaction block (touching 60k state values, in the case of the account
            // creation benchmark) on a 4B items DB (or 1.33B accounts) yields 300k JMT nodes
            batch_size: 1_000,
        }
    }
}

impl Default for EpochSnapshotPrunerConfig {
    fn default() -> Self {
        Self {
            enable: true,
            // This is based on ~5K TPS * 2h/epoch * 2 epochs. -- epoch ending snapshots are used
            // by state sync in fast sync mode.
            // The setting is in versions, not epochs, because this makes it behave more like other
            // pruners: a slower network will have longer history in db with the same pruner
            // settings, but the disk space take will be similar.
            // settings.
            prune_window: 80_000_000,
            // A 10k transaction block (touching 60k state values, in the case of the account
            // creation benchmark) on a 4B items DB (or 1.33B accounts) yields 300k JMT nodes
            batch_size: 1_000,
        }
    }
}

impl Default for StorageConfig {
    fn default() -> StorageConfig {
        StorageConfig {
            backup_service_address: SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 6186),
            dir: PathBuf::from("db"),
            // The prune window must at least out live a RPC request because its sub requests are
            // to return a consistent view of the DB at exactly same version. Considering a few
            // thousand TPS we are potentially going to achieve, and a few minutes a consistent view
            // of the DB might require, 10k (TPS)  * 100 (seconds)  =  1 Million might be a
            // conservatively safe minimal prune window. It'll take a few Gigabytes of disk space
            // depending on the size of an average account blob.
            storage_pruner_config: PrunerConfig::default(),
            data_dir: PathBuf::from("/opt/aptos/data"),
            rocksdb_configs: RocksdbConfigs::default(),
            enable_indexer: false,
            buffered_state_target_items: BUFFERED_STATE_TARGET_ITEMS,
            max_num_nodes_per_lru_cache_shard: DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
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
        self.backup_service_address
            .set_port(utils::get_available_port());
    }
}

impl ConfigSanitizer for StorageConfig {
    fn sanitize(
        node_config: &mut NodeConfig,
        _node_type: NodeType,
        _chain_id: ChainId,
    ) -> Result<(), Error> {
        let sanitizer_name = Self::get_sanitizer_name();
        let config = &node_config.storage;

        let ledger_prune_window = config
            .storage_pruner_config
            .ledger_pruner_config
            .prune_window;
        let state_merkle_prune_window = config
            .storage_pruner_config
            .state_merkle_pruner_config
            .prune_window;
        let epoch_snapshot_prune_window = config
            .storage_pruner_config
            .epoch_snapshot_pruner_config
            .prune_window;
        let user_pruning_window_offset = config
            .storage_pruner_config
            .ledger_pruner_config
            .user_pruning_window_offset;

        if ledger_prune_window < 50_000_000 {
            warn!("Ledger prune_window is too small, harming network data availability.");
        }
        if state_merkle_prune_window < 100_000 {
            warn!("State Merkle prune_window is too small, node might stop functioning.");
        }
        if epoch_snapshot_prune_window < 50_000_000 {
            warn!("Epoch snapshot prune_window is too small, harming network data availability.");
        }
        if user_pruning_window_offset > 1_000_000 {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "user_pruning_window_offset too large, so big a buffer is unlikely necessary. Set something < 1 million.".to_string(),
            ));
        }
        if user_pruning_window_offset > ledger_prune_window {
            return Err(Error::ConfigSanitizerFailed(
                sanitizer_name,
                "user_pruning_window_offset is larger than the ledger prune window, the API will refuse to return any data.".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::config::PrunerConfig;

    #[test]
    pub fn test_default_prune_window() {
        // These can be changed, but think twice -- make them safe for mainnet

        let config = PrunerConfig::default();
        assert!(config.ledger_pruner_config.prune_window >= 50_000_000);
        assert!(config.state_merkle_pruner_config.prune_window >= 100_000);
        assert!(config.epoch_snapshot_pruner_config.prune_window > 50_000_000);
    }
}
