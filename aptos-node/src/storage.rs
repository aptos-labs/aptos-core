// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::genesis_state::try_new_genesis_state_reader;
use anyhow::{anyhow, ensure, Result};
use aptos_backup_service::start_backup_service;
use aptos_config::{config::NodeConfig, utils::get_genesis_txn};
use aptos_db::AptosDB;
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_executor::db_bootstrapper::{calculate_genesis, maybe_bootstrap, GenesisCommitter};
use aptos_indexer_grpc_table_info::internal_indexer_db_service::InternalIndexerDBService;
use aptos_logger::{debug, info};
use aptos_state_sync_driver::LocalGenesis;
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::{
    ledger_info::LedgerInfoWithSignatures, transaction::Version, waypoint::Waypoint,
};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use std::{fs, path::Path, sync::Arc, time::Instant};
use tokio::{
    runtime::Runtime,
    sync::watch::{channel, Receiver as WatchReceiver},
};
pub(crate) fn maybe_apply_genesis(
    db_rw: &DbReaderWriter,
    node_config: &NodeConfig,
) -> Result<Option<LedgerInfoWithSignatures>> {
    // We read from the storage genesis waypoint and fallback to the node config one if it is none
    let genesis_waypoint = node_config
        .execution
        .genesis_waypoint
        .as_ref()
        .unwrap_or(&node_config.base.waypoint)
        .genesis_waypoint();
    if let Some(genesis) = get_genesis_txn(node_config) {
        let ledger_info_opt =
            maybe_bootstrap::<AptosVMBlockExecutor>(db_rw, genesis, genesis_waypoint)
                .map_err(|err| anyhow!("DB failed to bootstrap {}", err))?;
        Ok(ledger_info_opt)
    } else {
        info ! ("Genesis txn not provided! This is fine only if you don't expect to apply it. Otherwise, the config is incorrect!");
        Ok(None)
    }
}

/// The storage handles the rest of the node is built on
pub struct StorageHandles {
    /// The opened DB, held so the backup service can be started against it
    /// once the node has finished bootstrapping. Absent in consensus-only
    /// mode, which runs no backup service.
    pub db: Option<Arc<AptosDB>>,
    pub db_rw: DbReaderWriter,
    pub internal_indexer_db: Option<InternalIndexerDB>,
    pub update_receiver: Option<WatchReceiver<(Instant, Version)>>,
    /// Present only when the node is about to fast sync and still holds a
    /// usable genesis blob.
    pub local_genesis: Option<LocalGenesis>,
}

#[cfg(not(feature = "consensus-only-perf-test"))]
pub(crate) fn bootstrap_db(node_config: &NodeConfig) -> Result<StorageHandles> {
    let internal_indexer_db = InternalIndexerDBService::get_indexer_db(node_config);
    let (update_sender, update_receiver) = if internal_indexer_db.is_some() {
        let (sender, receiver) = channel::<(Instant, Version)>((Instant::now(), 0 as Version));
        (Some(sender), Some(receiver))
    } else {
        (None, None)
    };

    let mut db = AptosDB::open(
        node_config.storage.get_dir_paths(),
        /*readonly=*/ false,
        node_config.storage.storage_pruner_config,
        node_config.storage.rocksdb_configs,
        node_config.storage.buffered_state_target_items,
        node_config.storage.max_num_nodes_per_lru_cache_shard,
        internal_indexer_db.clone(),
        node_config.storage.hot_state_config,
    )
    .map_err(|err| anyhow!("DB failed to open {}", err))?;
    if let Some(sender) = update_sender {
        db.add_version_update_subscriber(sender)?;
    }

    // A node configured for fast sync that has never committed a transaction
    // gets genesis computed but not persisted: the state snapshot it is about
    // to restore supersedes it, and genesis rows left at version 0 would
    // resurrect keys that were deleted before the snapshot version.
    let fast_syncing = node_config
        .state_sync
        .state_sync_driver
        .bootstrapping_mode
        .is_fast_sync()
        && db.get_synced_version()?.is_none();

    let (db_arc, db_rw) = DbReaderWriter::wrap(db);
    let local_genesis = if fast_syncing {
        commit_genesis_ledger_info_only(node_config, &db_arc, &db_rw)?;
        build_local_genesis(node_config, &db_rw)
    } else {
        maybe_apply_genesis(&db_rw, node_config)?;
        None
    };

    Ok(StorageHandles {
        db: Some(db_arc),
        db_rw,
        internal_indexer_db,
        update_receiver,
        local_genesis,
    })
}

/// Starts the backup service.
///
/// Deferred until the node has bootstrapped: until then there is nothing worth
/// backing up, and on a fast syncing node the DB is mid-restore.
pub(crate) fn start_backup_service_for(node_config: &NodeConfig, db: Arc<AptosDB>) -> Runtime {
    start_backup_service(
        node_config.storage.backup_service_address,
        db,
        node_config.storage.backup_service_runtime_threads,
    )
}

/// Executes genesis and verifies it against the configured genesis waypoint.
///
/// Returns `None` when the node has no genesis blob, which is only workable if
/// it expects to get everything, genesis included, from its peers.
fn calculate_verified_genesis(
    node_config: &NodeConfig,
    db_rw: &DbReaderWriter,
) -> Result<Option<GenesisCommitter>> {
    let Some(genesis_txn) = get_genesis_txn(node_config) else {
        info!("Genesis txn not provided! This is fine only if you don't expect to apply it. Otherwise, the config is incorrect!");
        return Ok(None);
    };

    let ledger_summary = db_rw.reader.get_pre_committed_ledger_summary()?;
    let committer = calculate_genesis::<AptosVMBlockExecutor>(db_rw, ledger_summary, genesis_txn)
        .map_err(|err| anyhow!("Failed to calculate genesis: {}", err))?;

    let genesis_waypoint = node_config
        .execution
        .genesis_waypoint
        .as_ref()
        .unwrap_or(&node_config.base.waypoint)
        .genesis_waypoint();
    ensure!(
        committer.waypoint() == genesis_waypoint,
        "Waypoint verification failed. Expected {:?}, got {:?}.",
        genesis_waypoint,
        committer.waypoint(),
    );

    Ok(Some(committer))
}

/// Executes genesis and persists only the resulting epoch-0 ledger info.
///
/// State sync needs that ledger info as its trust root: its `next_epoch_state`
/// is what verifies the epoch-ending ledger infos fetched from peers, which in
/// turn verify the configured waypoint. The genesis state itself is discarded,
/// since fast sync is about to replace it wholesale.
fn commit_genesis_ledger_info_only(
    node_config: &NodeConfig,
    db: &Arc<AptosDB>,
    db_rw: &DbReaderWriter,
) -> Result<()> {
    if db.get_latest_ledger_info_option()?.is_some() {
        return Ok(()); // Already recorded on an earlier run
    }
    let Some(committer) = calculate_verified_genesis(node_config, db_rw)? else {
        return Ok(());
    };

    let genesis_li = committer
        .ledger_info()
        .ok_or_else(|| anyhow!("Genesis execution produced no ledger info!"))?;
    db.commit_genesis_ledger_info(genesis_li)?;

    // `committer` is dropped without committing, so the genesis state never
    // reaches disk.
    Ok(())
}

/// Commits genesis in full, for a node whose peers turned out to have nothing
/// beyond it.
///
/// The epoch-0 ledger info is already durable, recorded by
/// `commit_genesis_ledger_info_only` before the fast-sync target was known, so
/// only the transaction and state are written here. Committing the ledger info
/// again would be rejected as a gap in epoch history, since it is already the
/// latest.
fn commit_local_genesis(node_config: &NodeConfig, db_rw: &DbReaderWriter) -> Result<()> {
    if db_rw.reader.get_pre_committed_version()?.is_some() {
        return Ok(()); // Already committed on an earlier run
    }
    let Some(committer) = calculate_verified_genesis(node_config, db_rw)? else {
        return Ok(());
    };

    committer
        .commit_without_ledger_info()
        .map_err(|err| anyhow!("Failed to commit genesis: {}", err))
}

/// Bundles up what a fast syncing node can still get out of its genesis blob:
/// the on-chain configs its subscribers need before any state has landed, and a
/// way to commit genesis outright if the network has nothing beyond it.
fn build_local_genesis(node_config: &NodeConfig, db_rw: &DbReaderWriter) -> Option<LocalGenesis> {
    // Without a usable genesis blob everything, genesis included, has to come
    // from peers.
    let state_reader = try_new_genesis_state_reader(node_config)?;

    let commit_node_config = node_config.clone();
    let commit_db_rw = db_rw.clone();
    Some(LocalGenesis {
        state_reader,
        commit: Arc::new(move || commit_local_genesis(&commit_node_config, &commit_db_rw)),
    })
}

/// In consensus-only mode, return a in-memory based [FakeAptosDB] and
/// do not run the backup service.
#[cfg(feature = "consensus-only-perf-test")]
pub(crate) fn bootstrap_db(node_config: &NodeConfig) -> Result<StorageHandles> {
    use aptos_db::db::fake_aptosdb::FakeAptosDB;

    let aptos_db = AptosDB::open(
        node_config.storage.get_dir_paths(),
        false, /* readonly */
        node_config.storage.storage_pruner_config,
        node_config.storage.rocksdb_configs,
        node_config.storage.buffered_state_target_items,
        node_config.storage.max_num_nodes_per_lru_cache_shard,
        None,
        node_config.storage.hot_state_config,
    )
    .map_err(|err| anyhow!("DB failed to open {}", err))?;
    let (_fake_db, db_rw) = DbReaderWriter::wrap(FakeAptosDB::new(aptos_db));
    maybe_apply_genesis(&db_rw, node_config)?;
    Ok(StorageHandles {
        db: None,
        db_rw,
        internal_indexer_db: None,
        update_receiver: None,
        local_genesis: None,
    })
}

/// Creates a RocksDb checkpoint for the consensus_db, state_sync_db,
/// ledger_db and state_merkle_db and saves it to the checkpoint_path.
/// Also, changes the working directory to run the node on the new path,
/// so that the existing data won't change. For now this is a test-only feature.
fn create_rocksdb_checkpoint_and_change_working_dir(
    node_config: &mut NodeConfig,
    working_dir: impl AsRef<Path>,
) {
    // Update the source and checkpoint directories
    let source_dir = node_config.storage.dir();
    node_config.set_data_dir(working_dir.as_ref().to_path_buf());
    let checkpoint_dir = node_config.storage.dir();
    assert!(source_dir != checkpoint_dir);

    // Create rocksdb checkpoint directory
    fs::create_dir_all(&checkpoint_dir).unwrap();

    // Open the database and create a checkpoint
    AptosDB::create_checkpoint(&source_dir, &checkpoint_dir)
        .expect("AptosDB checkpoint creation failed.");

    // Create a consensus db checkpoint
    aptos_consensus::create_checkpoint(&source_dir, &checkpoint_dir)
        .expect("ConsensusDB checkpoint creation failed.");

    // Create a state sync db checkpoint
    let state_sync_db =
        aptos_state_sync_driver::metadata_storage::PersistentMetadataStorage::new(&source_dir);
    state_sync_db
        .create_checkpoint(&checkpoint_dir)
        .expect("StateSyncDB checkpoint creation failed.");
}

/// Creates any rocksdb checkpoints, opens the storage database,
/// starts the backup service, handles genesis initialization and returns
/// the various handles.
pub fn initialize_database_and_checkpoints(
    node_config: &mut NodeConfig,
) -> Result<(StorageHandles, Waypoint)> {
    // If required, create RocksDB checkpoints and change the working directory.
    // This is test-only.
    if let Some(working_dir) = node_config.base.working_dir.clone() {
        create_rocksdb_checkpoint_and_change_working_dir(node_config, working_dir);
    }

    // Open the database
    let instant = Instant::now();
    let storage_handles = bootstrap_db(node_config)?;

    // Log the duration to open storage
    debug!(
        "Storage service started in {} ms",
        instant.elapsed().as_millis()
    );

    Ok((
        storage_handles,
        node_config.base.waypoint.genesis_waypoint(),
    ))
}
