// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

pub mod backup_service_client;
pub(crate) mod error_notes;
pub mod read_record_bytes;
pub mod storage_ext;
pub(crate) mod stream;

#[cfg(test)]
pub mod test_utils;

use anyhow::{anyhow, Result};
use aptos_config::config::{
    RocksdbConfig, RocksdbConfigs, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    NO_OP_STORAGE_PRUNER_CONFIG, TARGET_SNAPSHOT_SIZE,
};
use aptos_crypto::HashValue;
use aptos_infallible::duration_since_epoch;
use aptos_jellyfish_merkle::{NodeBatch, TreeWriter};
use aptos_logger::info;
use aptos_types::state_store::state_storage_usage::StateStorageUsage;
use aptos_types::{
    state_store::{state_key::StateKey, state_value::StateValue},
    transaction::Version,
    waypoint::Waypoint,
};
use aptosdb::state_restore::StateSnapshotProgress;
use aptosdb::{
    backup::restore_handler::RestoreHandler,
    state_restore::{StateSnapshotRestore, StateValueBatch, StateValueWriter},
    AptosDB, GetRestoreHandler,
};
use clap::Parser;
use std::{
    collections::HashMap,
    convert::TryFrom,
    mem::size_of,
    path::{Path, PathBuf},
    sync::Arc,
};
use tokio::fs::metadata;

#[derive(Clone, Parser)]
pub struct GlobalBackupOpt {
    // Defaults to 128MB, so concurrent chunk downloads won't take up too much memory.
    #[clap(
        long = "max-chunk-size",
        default_value = "134217728",
        help = "Maximum chunk file size in bytes."
    )]
    pub max_chunk_size: usize,
}

#[derive(Clone, Parser)]
pub struct RocksdbOpt {
    #[clap(long, default_value = "5000")]
    ledger_db_max_open_files: i32,
    #[clap(long, default_value = "1073741824")] // 1GB
    ledger_db_max_total_wal_size: u64,
    #[clap(long, default_value = "5000")]
    state_merkle_db_max_open_files: i32,
    #[clap(long, default_value = "1073741824")] // 1GB
    state_merkle_db_max_total_wal_size: u64,
    #[clap(long, default_value = "1000")]
    index_db_max_open_files: i32,
    #[clap(long, default_value = "1073741824")] // 1GB
    index_db_max_total_wal_size: u64,
    #[clap(long, default_value = "16")]
    max_background_jobs: i32,
}

impl From<RocksdbOpt> for RocksdbConfigs {
    fn from(opt: RocksdbOpt) -> Self {
        Self {
            ledger_db_config: RocksdbConfig {
                max_open_files: opt.ledger_db_max_open_files,
                max_total_wal_size: opt.ledger_db_max_total_wal_size,
                max_background_jobs: opt.max_background_jobs,
                ..Default::default()
            },
            state_merkle_db_config: RocksdbConfig {
                max_open_files: opt.state_merkle_db_max_open_files,
                max_total_wal_size: opt.state_merkle_db_max_total_wal_size,
                max_background_jobs: opt.max_background_jobs,
                ..Default::default()
            },
            index_db_config: RocksdbConfig {
                max_open_files: opt.index_db_max_open_files,
                max_total_wal_size: opt.index_db_max_total_wal_size,
                max_background_jobs: opt.max_background_jobs,
                ..Default::default()
            },
        }
    }
}

impl Default for RocksdbOpt {
    fn default() -> Self {
        Self::from_iter(vec!["exe"])
    }
}

#[derive(Clone, Parser)]
pub struct GlobalRestoreOpt {
    #[clap(long, help = "Dry run without writing data to DB.")]
    pub dry_run: bool,

    #[clap(
        long = "target-db-dir",
        parse(from_os_str),
        conflicts_with = "dry-run",
        required_unless = "dry-run"
    )]
    pub db_dir: Option<PathBuf>,

    #[clap(
        long,
        help = "Content newer than this version will not be recovered to DB, \
        defaulting to the largest version possible, meaning recover everything in the backups."
    )]
    pub target_version: Option<Version>,

    #[clap(flatten)]
    pub trusted_waypoints: TrustedWaypointOpt,

    #[clap(flatten)]
    pub rocksdb_opt: RocksdbOpt,

    #[clap(flatten)]
    pub concurrent_downloads: ConcurrentDownloadsOpt,

    #[clap(flatten)]
    pub replay_concurrency_level: ReplayConcurrencyLevelOpt,
}

pub enum RestoreRunMode {
    Restore { restore_handler: RestoreHandler },
    Verify,
}

struct MockStore;

impl TreeWriter<StateKey> for MockStore {
    fn write_node_batch(&self, _node_batch: &NodeBatch<StateKey>) -> Result<()> {
        Ok(())
    }
}

impl StateValueWriter<StateKey, StateValue> for MockStore {
    fn write_kv_batch(
        &self,
        _version: Version,
        _kv_batch: &StateValueBatch<StateKey, Option<StateValue>>,
        _progress: StateSnapshotProgress,
    ) -> Result<()> {
        Ok(())
    }

    fn write_usage(&self, _version: Version, _usage: StateStorageUsage) -> Result<()> {
        Ok(())
    }

    fn get_progress(&self, _version: Version) -> Result<Option<StateSnapshotProgress>> {
        Ok(None)
    }
}

impl RestoreRunMode {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Restore { restore_handler: _ } => "restore",
            Self::Verify => "verify",
        }
    }

    pub fn is_verify(&self) -> bool {
        match self {
            Self::Restore { restore_handler: _ } => false,
            Self::Verify => true,
        }
    }

    pub fn get_state_restore_receiver(
        &self,
        version: Version,
        expected_root_hash: HashValue,
    ) -> Result<StateSnapshotRestore<StateKey, StateValue>> {
        match self {
            Self::Restore { restore_handler } => {
                restore_handler.get_state_restore_receiver(version, expected_root_hash)
            }
            Self::Verify => {
                let mock_store = Arc::new(MockStore);
                StateSnapshotRestore::new_overwrite(
                    &mock_store,
                    &mock_store,
                    version,
                    expected_root_hash,
                )
            }
        }
    }

    pub fn finish(&self) {
        match self {
            Self::Restore { restore_handler } => {
                restore_handler.reset_state_store();
            }
            Self::Verify => (),
        }
    }

    pub fn get_next_expected_transaction_version(&self) -> Result<Version> {
        match self {
            RestoreRunMode::Restore { restore_handler } => {
                restore_handler.get_next_expected_transaction_version()
            }
            RestoreRunMode::Verify => {
                info!("This is a dry run. Assuming resuming point at version 0.");
                Ok(0)
            }
        }
    }

    pub fn get_in_progress_state_snapshot(&self) -> Result<Option<Version>> {
        match self {
            RestoreRunMode::Restore { restore_handler } => {
                restore_handler.get_in_progress_state_snapshot_version()
            }
            RestoreRunMode::Verify => Ok(None),
        }
    }
}

#[derive(Clone)]
pub struct GlobalRestoreOptions {
    pub target_version: Version,
    pub trusted_waypoints: Arc<HashMap<Version, Waypoint>>,
    pub run_mode: Arc<RestoreRunMode>,
    pub concurrent_downloads: usize,
    pub replay_concurrency_level: usize,
}

impl TryFrom<GlobalRestoreOpt> for GlobalRestoreOptions {
    type Error = anyhow::Error;

    fn try_from(opt: GlobalRestoreOpt) -> Result<Self> {
        let target_version = opt.target_version.unwrap_or(Version::max_value());
        let concurrent_downloads = opt.concurrent_downloads.get();
        let replay_concurrency_level = opt.replay_concurrency_level.get();
        let run_mode = if let Some(db_dir) = &opt.db_dir {
            let restore_handler = Arc::new(AptosDB::open(
                db_dir,
                false,                       /* read_only */
                NO_OP_STORAGE_PRUNER_CONFIG, /* pruner config */
                opt.rocksdb_opt.into(),
                false,
                TARGET_SNAPSHOT_SIZE,
                DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            )?)
            .get_restore_handler();
            RestoreRunMode::Restore { restore_handler }
        } else {
            RestoreRunMode::Verify
        };
        Ok(Self {
            target_version,
            trusted_waypoints: Arc::new(opt.trusted_waypoints.verify()?),
            run_mode: Arc::new(run_mode),
            concurrent_downloads,
            replay_concurrency_level,
        })
    }
}

#[derive(Clone, Default, Parser)]
pub struct TrustedWaypointOpt {
    #[clap(
        long,
        help = "(multiple) When provided, an epoch ending LedgerInfo at the waypoint version will be \
        checked against the hash in the waypoint, but signatures on it are NOT checked. \
        Use this for two purposes: \
        1. set the genesis or the latest waypoint to confirm the backup is compatible. \
        2. set waypoints at versions where writeset transactions were used to overwrite the \
        validator set, so that the signature check is skipped. \
        N.B. LedgerInfos are verified only when restoring / verifying the epoch ending backups, \
        i.e. they are NOT checked at all when doing one-shot restoring of the transaction \
        and state backups."
    )]
    pub trust_waypoint: Vec<Waypoint>,
}

impl TrustedWaypointOpt {
    pub fn verify(self) -> Result<HashMap<Version, Waypoint>> {
        let mut trusted_waypoints = HashMap::new();
        for w in self.trust_waypoint {
            trusted_waypoints
                .insert(w.version(), w)
                .map_or(Ok(()), |w| {
                    Err(anyhow!("Duplicated waypoints at version {}", w.version()))
                })?;
        }
        Ok(trusted_waypoints)
    }
}

#[derive(Clone, Copy, Default, Parser)]
pub struct ConcurrentDownloadsOpt {
    #[clap(
        long,
        help = "Number of concurrent downloads from the backup storage. This covers the initial \
        metadata downloads as well. Speeds up remote backup access. [Defaults to number of CPUs]"
    )]
    concurrent_downloads: Option<usize>,
}

impl ConcurrentDownloadsOpt {
    pub fn get(&self) -> usize {
        let ret = self.concurrent_downloads.unwrap_or_else(num_cpus::get);
        info!(
            concurrent_downloads = ret,
            "Determined concurrency level for downloading."
        );
        ret
    }
}

#[derive(Clone, Copy, Default, Parser)]
pub struct ReplayConcurrencyLevelOpt {
    /// AptosVM::set_concurrency_level_once() is called with this
    #[clap(
        long,
        help = "concurrency_level used by the transaction executor, applicable when replaying transactions \
        after a state snapshot. [Defaults to number of CPUs]"
    )]
    replay_concurrency_level: Option<usize>,
}

impl ReplayConcurrencyLevelOpt {
    pub fn get(&self) -> usize {
        let ret = self.replay_concurrency_level.unwrap_or_else(num_cpus::get);
        info!(
            concurrency = ret,
            "Determined concurrency level for transaction replaying."
        );
        ret
    }
}

pub(crate) fn should_cut_chunk(chunk: &[u8], record: &[u8], max_chunk_size: usize) -> bool {
    !chunk.is_empty() && chunk.len() + record.len() + size_of::<u32>() > max_chunk_size
}

// TODO: use Path::exists() when Rust 1.5 stabilizes.
pub(crate) async fn path_exists(path: &Path) -> bool {
    metadata(&path).await.is_ok()
}

pub(crate) trait PathToString {
    fn path_to_string(&self) -> Result<String>;
}

impl<T: AsRef<Path>> PathToString for T {
    fn path_to_string(&self) -> Result<String> {
        self.as_ref()
            .to_path_buf()
            .into_os_string()
            .into_string()
            .map_err(|s| anyhow!("into_string failed for OsString '{:?}'", s))
    }
}

pub(crate) fn unix_timestamp_sec() -> i64 {
    duration_since_epoch().as_secs() as i64
}
