// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    config::{
        config_optimizer::ConfigOptimizer, config_sanitizer::ConfigSanitizer,
        node_config_loader::NodeType, Error, NodeConfig,
    },
    utils,
};
use anyhow::{bail, ensure, Result};
use aptos_logger::warn;
use aptos_types::chain_id::ChainId;
use arr_macro::arr;
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
    str::FromStr,
};

// Lru cache will consume about 2G RAM based on this default value.
pub const DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD: usize = 1 << 13;

pub const BUFFERED_STATE_TARGET_ITEMS: usize = 100_000;
pub const BUFFERED_STATE_TARGET_ITEMS_FOR_TEST: usize = 10;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct DbPathConfig {
    pub ledger_db_path: Option<PathBuf>,
    pub state_kv_db_path: Option<ShardedDbPathConfig>,
    pub state_merkle_db_path: Option<ShardedDbPathConfig>,
    pub hot_state_kv_db_path: Option<ShardedDbPathConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardedDbPathConfig {
    pub metadata_path: Option<PathBuf>,
    pub shard_paths: Vec<ShardPathConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardPathConfig {
    pub shards: String,
    pub path: PathBuf,
}

impl ShardedDbPathConfig {
    pub fn get_shard_paths(&self) -> Result<HashMap<u8, PathBuf>> {
        let mut result = HashMap::new();
        for shard_path in &self.shard_paths {
            let shard_ids = Self::parse(shard_path.shards.as_str())?;
            let path = &shard_path.path;
            ensure!(
                path.is_absolute(),
                "Path ({path:?}) is not an absolute path."
            );
            for shard_id in shard_ids {
                ensure!(
                    shard_id < 16,
                    "Shard id ({shard_id}) is out of range [0, 16)."
                );
                let exist = result.insert(shard_id, path.clone()).is_some();
                ensure!(
                    !exist,
                    "Duplicated shard id ({shard_id}) is not allowed in the config."
                );
            }
        }

        Ok(result)
    }

    fn parse(path: &str) -> Result<Vec<u8>> {
        let mut shard_ids = vec![];
        for p in path.split(',') {
            let num_or_range: Vec<&str> = p.split('-').collect();
            match num_or_range.len() {
                1 => {
                    let num = u8::from_str(num_or_range[0])?;
                    ensure!(num < 16);
                    shard_ids.push(num);
                },
                2 => {
                    let range_start = u8::from_str(num_or_range[0])?;
                    let range_end = u8::from_str(num_or_range[1])?;
                    ensure!(range_start <= range_end && range_end < 16);
                    for num in range_start..=range_end {
                        shard_ids.push(num);
                    }
                },
                _ => bail!("Invalid path: {path}."),
            }
        }

        Ok(shard_ids)
    }
}

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
    // TODO(grao): Add RocksdbConfig for individual ledger DBs when necessary.
    pub ledger_db_config: RocksdbConfig,
    pub state_merkle_db_config: RocksdbConfig,
    pub state_kv_db_config: RocksdbConfig,
    pub index_db_config: RocksdbConfig,
    pub enable_storage_sharding: bool,
}

impl Default for RocksdbConfigs {
    fn default() -> Self {
        Self {
            ledger_db_config: RocksdbConfig::default(),
            state_merkle_db_config: RocksdbConfig::default(),
            state_kv_db_config: RocksdbConfig::default(),
            index_db_config: RocksdbConfig {
                max_open_files: 1000,
                ..Default::default()
            },
            enable_storage_sharding: false,
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
    /// TODO(jill): deprecate Indexer once Indexer Async V2 is ready
    pub enable_indexer: bool,
    /// Fine grained control for db paths of individal databases/shards.
    /// If not specificed, will use `dir` as default.
    /// Only allowed when sharding is enabled.
    pub db_path_overrides: Option<DbPathConfig>,
    /// ensure `ulimit -n`, set to 0 to not ensure.
    pub ensure_rlimit_nofile: u64,
    /// panic if failed to ensure `ulimit -n`
    pub assert_rlimit_nofile: bool,
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
            prune_window: 90_000_000,
            batch_size: 5_000,
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
            db_path_overrides: None,
            buffered_state_target_items: BUFFERED_STATE_TARGET_ITEMS,
            max_num_nodes_per_lru_cache_shard: DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            ensure_rlimit_nofile: 0,
            assert_rlimit_nofile: false,
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

    pub fn get_dir_paths(&self) -> StorageDirPaths {
        let default_dir = self.dir();
        let mut ledger_db_path = None;
        let mut state_kv_db_paths = ShardedDbPaths::default();
        let mut state_merkle_db_paths = ShardedDbPaths::default();
        let mut hot_state_kv_db_paths = ShardedDbPaths::default();

        if let Some(db_path_overrides) = self.db_path_overrides.as_ref() {
            db_path_overrides
                .ledger_db_path
                .clone_into(&mut ledger_db_path);

            if let Some(state_kv_db_path) = db_path_overrides.state_kv_db_path.as_ref() {
                state_kv_db_paths = ShardedDbPaths::new(state_kv_db_path);
            }

            if let Some(state_merkle_db_path) = db_path_overrides.state_merkle_db_path.as_ref() {
                state_merkle_db_paths = ShardedDbPaths::new(state_merkle_db_path);
            }

            if let Some(hot_state_kv_db_path) = db_path_overrides.hot_state_kv_db_path.as_ref() {
                hot_state_kv_db_paths = ShardedDbPaths::new(hot_state_kv_db_path);
            }
        }

        StorageDirPaths::new(
            default_dir,
            ledger_db_path,
            state_kv_db_paths,
            state_merkle_db_paths,
            hot_state_kv_db_paths,
        )
    }

    pub fn set_data_dir(&mut self, data_dir: PathBuf) {
        self.data_dir = data_dir;
    }

    pub fn randomize_ports(&mut self) {
        self.backup_service_address
            .set_port(utils::get_available_port());
    }
}

#[derive(Debug)]
pub struct StorageDirPaths {
    default_path: PathBuf,
    ledger_db_path: Option<PathBuf>,
    state_kv_db_paths: ShardedDbPaths,
    state_merkle_db_paths: ShardedDbPaths,
    hot_state_kv_db_paths: ShardedDbPaths,
}

impl StorageDirPaths {
    pub fn default_root_path(&self) -> &PathBuf {
        &self.default_path
    }

    pub fn ledger_db_root_path(&self) -> &PathBuf {
        if let Some(ledger_db_path) = self.ledger_db_path.as_ref() {
            ledger_db_path
        } else {
            &self.default_path
        }
    }

    pub fn state_kv_db_metadata_root_path(&self) -> &PathBuf {
        self.state_kv_db_paths
            .metadata_path()
            .unwrap_or(&self.default_path)
    }

    pub fn state_kv_db_shard_root_path(&self, shard_id: usize) -> &PathBuf {
        self.state_kv_db_paths
            .shard_path(shard_id)
            .unwrap_or(&self.default_path)
    }

    pub fn state_merkle_db_metadata_root_path(&self) -> &PathBuf {
        self.state_merkle_db_paths
            .metadata_path()
            .unwrap_or(&self.default_path)
    }

    pub fn state_merkle_db_shard_root_path(&self, shard_id: usize) -> &PathBuf {
        self.state_merkle_db_paths
            .shard_path(shard_id)
            .unwrap_or(&self.default_path)
    }

    pub fn hot_state_kv_db_shard_root_path(&self, shard_id: usize) -> &PathBuf {
        self.hot_state_kv_db_paths
            .shard_path(shard_id)
            .unwrap_or(&self.default_path)
    }

    pub fn from_path<P: AsRef<Path>>(path: P) -> Self {
        Self {
            default_path: path.as_ref().to_path_buf(),
            ledger_db_path: None,
            state_kv_db_paths: Default::default(),
            state_merkle_db_paths: Default::default(),
            hot_state_kv_db_paths: Default::default(),
        }
    }

    fn new(
        default_path: PathBuf,
        ledger_db_path: Option<PathBuf>,
        state_kv_db_paths: ShardedDbPaths,
        state_merkle_db_paths: ShardedDbPaths,
        hot_state_kv_db_paths: ShardedDbPaths,
    ) -> Self {
        Self {
            default_path,
            ledger_db_path,
            state_kv_db_paths,
            state_merkle_db_paths,
            hot_state_kv_db_paths,
        }
    }
}

#[derive(Debug, Default)]
struct ShardedDbPaths {
    metadata_path: Option<PathBuf>,
    shard_paths: [Option<PathBuf>; 16],
}

impl ShardedDbPaths {
    fn new(config: &ShardedDbPathConfig) -> Self {
        let mut shard_paths = arr![None; 16];
        for (shard_id, shard_path) in config.get_shard_paths().expect("Invalid config.") {
            shard_paths[shard_id as usize] = Some(shard_path);
        }

        Self {
            metadata_path: config.metadata_path.clone(),
            shard_paths,
        }
    }

    fn metadata_path(&self) -> Option<&PathBuf> {
        self.metadata_path.as_ref()
    }

    fn shard_path(&self, shard_id: usize) -> Option<&PathBuf> {
        self.shard_paths[shard_id].as_ref()
    }
}

impl ConfigOptimizer for StorageConfig {
    fn optimize(
        node_config: &mut NodeConfig,
        local_config_yaml: &Value,
        _node_type: NodeType,
        chain_id: Option<ChainId>,
    ) -> Result<bool, Error> {
        let config = &mut node_config.storage;
        let config_yaml = &local_config_yaml["storage"];

        let mut modified_config = false;
        if let Some(chain_id) = chain_id {
            if (chain_id.is_testnet() || chain_id.is_mainnet())
                && config_yaml["ensure_rlimit_nofile"].is_null()
            {
                config.ensure_rlimit_nofile = 999_999;
                modified_config = true;
            }
            if chain_id.is_testnet() && config_yaml["assert_rlimit_nofile"].is_null() {
                config.assert_rlimit_nofile = true;
                modified_config = true;
            }
        }

        Ok(modified_config)
    }
}

impl ConfigSanitizer for StorageConfig {
    fn sanitize(
        node_config: &NodeConfig,
        _node_type: NodeType,
        _chain_id: Option<ChainId>,
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

        if let Some(db_path_overrides) = config.db_path_overrides.as_ref() {
            if !config.rocksdb_configs.enable_storage_sharding {
                return Err(Error::ConfigSanitizerFailed(
                    sanitizer_name,
                    "db_path_overrides is allowed only if sharding is enabled.".to_string(),
                ));
            }

            if let Some(ledger_db_path) = db_path_overrides.ledger_db_path.as_ref() {
                if !ledger_db_path.is_absolute() {
                    return Err(Error::ConfigSanitizerFailed(
                        sanitizer_name,
                        "Path {ledger_db_path:?} in db_path_overrides is not an absolute path."
                            .to_string(),
                    ));
                }
            }

            if let Some(state_kv_db_path) = db_path_overrides.state_kv_db_path.as_ref() {
                if let Some(metadata_path) = state_kv_db_path.metadata_path.as_ref() {
                    if !metadata_path.is_absolute() {
                        return Err(Error::ConfigSanitizerFailed(
                            sanitizer_name,
                            "Path {metadata_path:?} in db_path_overrides is not an absolute path."
                                .to_string(),
                        ));
                    }
                }

                if let Err(e) = state_kv_db_path.get_shard_paths() {
                    return Err(Error::ConfigSanitizerFailed(sanitizer_name, e.to_string()));
                }
            }

            if let Some(state_merkle_db_path) = db_path_overrides.state_merkle_db_path.as_ref() {
                if let Some(metadata_path) = state_merkle_db_path.metadata_path.as_ref() {
                    if !metadata_path.is_absolute() {
                        return Err(Error::ConfigSanitizerFailed(
                            sanitizer_name,
                            "Path {metadata_path:?} in db_path_overrides is not an absolute path."
                                .to_string(),
                        ));
                    }
                }

                if let Err(e) = state_merkle_db_path.get_shard_paths() {
                    return Err(Error::ConfigSanitizerFailed(sanitizer_name, e.to_string()));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use crate::config::{
        config_optimizer::ConfigOptimizer, NodeConfig, NodeType, PrunerConfig, ShardPathConfig,
        ShardedDbPathConfig, StorageConfig,
    };
    use aptos_types::chain_id::ChainId;

    #[test]
    pub fn test_default_prune_window() {
        // These can be changed, but think twice -- make them safe for mainnet

        let config = PrunerConfig::default();
        assert!(config.ledger_pruner_config.prune_window >= 50_000_000);
        assert!(config.state_merkle_pruner_config.prune_window >= 100_000);
        assert!(config.epoch_snapshot_pruner_config.prune_window > 50_000_000);
    }

    #[test]
    pub fn test_sharded_db_path_config() {
        let path_overrides = ShardedDbPathConfig {
            metadata_path: Some("/disk0/db".into()),
            shard_paths: vec![
                ShardPathConfig {
                    shards: "2-4".into(),
                    path: "/disk1/db".into(),
                },
                ShardPathConfig {
                    shards: "8,10-11".into(),
                    path: "/disk2/db".into(),
                },
            ],
        };

        let shard_path_map = path_overrides.get_shard_paths().unwrap();
        assert_eq!(shard_path_map.len(), 6);
        assert_eq!(shard_path_map.get(&2), Some(&"/disk1/db".into()));
        assert_eq!(shard_path_map.get(&3), Some(&"/disk1/db".into()));
        assert_eq!(shard_path_map.get(&4), Some(&"/disk1/db".into()));
        assert_eq!(shard_path_map.get(&8), Some(&"/disk2/db".into()));
        assert_eq!(shard_path_map.get(&10), Some(&"/disk2/db".into()));
        assert_eq!(shard_path_map.get(&11), Some(&"/disk2/db".into()));
    }

    #[test]
    pub fn test_invalid_sharded_db_path_config() {
        let path_overrides = ShardedDbPathConfig {
            metadata_path: None,
            shard_paths: vec![ShardPathConfig {
                shards: "16".into(),
                path: "/disk1/db".into(),
            }],
        };

        assert!(path_overrides.get_shard_paths().is_err());

        let path_overrides = ShardedDbPathConfig {
            metadata_path: None,
            shard_paths: vec![ShardPathConfig {
                shards: "1".into(),
                path: "db".into(),
            }],
        };

        assert!(path_overrides.get_shard_paths().is_err());

        let path_overrides = ShardedDbPathConfig {
            metadata_path: None,
            shard_paths: vec![
                ShardPathConfig {
                    shards: "12".into(),
                    path: "/disk1/db".into(),
                },
                ShardPathConfig {
                    shards: "11-13".into(),
                    path: "/disk1/db".into(),
                },
            ],
        };

        assert!(path_overrides.get_shard_paths().is_err());
    }

    #[test]
    fn test_optimize_ensure_rlimit_nofile() {
        let mut node_config = NodeConfig::default();
        assert_eq!(node_config.storage.ensure_rlimit_nofile, 0);
        assert!(!node_config.storage.assert_rlimit_nofile);

        let modified_config = StorageConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::mainnet()),
        )
        .unwrap();
        assert!(modified_config);

        assert_eq!(node_config.storage.ensure_rlimit_nofile, 999_999);
        assert!(!node_config.storage.assert_rlimit_nofile);

        let modified_config = StorageConfig::optimize(
            &mut node_config,
            &serde_yaml::from_str("{}").unwrap(), // An empty local config,
            NodeType::Validator,
            Some(ChainId::testnet()),
        )
        .unwrap();
        assert!(modified_config);

        assert_eq!(node_config.storage.ensure_rlimit_nofile, 999_999);
        assert!(node_config.storage.assert_rlimit_nofile);
    }
}
