// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_backup_service::start_backup_service;
use aptos_config::{config::NodeConfig, utils::get_genesis_txn};
use aptos_db::{fast_sync_storage_wrapper::FastSyncStorageWrapper, AptosDB};
use aptos_db_indexer::db_indexer::InternalIndexerDB;
use aptos_executor::db_bootstrapper::maybe_bootstrap;
use aptos_indexer_grpc_table_info::internal_indexer_db_service::InternalIndexerDBService;
use aptos_logger::{debug, info};
use aptos_storage_interface::{DbReader, DbReaderWriter};
use aptos_types::{
    ledger_info::{set_waypoint_version, LedgerInfoWithSignatures},
    transaction::Version,
    waypoint::Waypoint,
};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use either::Either;
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

#[cfg(not(feature = "consensus-only-perf-test"))]
pub(crate) fn bootstrap_db(
    node_config: &NodeConfig,
) -> Result<(
    Arc<dyn DbReader>,
    DbReaderWriter,
    Option<Runtime>,
    Option<InternalIndexerDB>,
    Option<WatchReceiver<(Instant, Version)>>,
)> {
    let internal_indexer_db = InternalIndexerDBService::get_indexer_db(node_config);
    let (update_sender, update_receiver) = if internal_indexer_db.is_some() {
        let (sender, receiver) = channel::<(Instant, Version)>((Instant::now(), 0 as Version));
        (Some(sender), Some(receiver))
    } else {
        (None, None)
    };

    let (aptos_db_reader, db_rw, backup_service) = match FastSyncStorageWrapper::initialize_dbs(
        node_config,
        internal_indexer_db.clone(),
        update_sender,
    )? {
        Either::Left(db) => {
            let (db_arc, db_rw) = DbReaderWriter::wrap(db);
            let db_backup_service =
                start_backup_service(node_config.storage.backup_service_address, db_arc.clone());
            maybe_apply_genesis(&db_rw, node_config)?;
            (db_arc as Arc<dyn DbReader>, db_rw, Some(db_backup_service))
        },
        Either::Right(fast_sync_db_wrapper) => {
            let temp_db = fast_sync_db_wrapper.get_temporary_db_with_genesis();
            maybe_apply_genesis(&DbReaderWriter::from_arc(temp_db), node_config)?;
            let (db_arc, db_rw) = DbReaderWriter::wrap(fast_sync_db_wrapper);
            let fast_sync_db = db_arc.get_fast_sync_db();
            // FastSyncDB requires ledger info at epoch 0 to establish provenance to genesis
            let ledger_info = db_arc
                .get_temporary_db_with_genesis()
                .get_epoch_ending_ledger_info(0)
                .expect("Genesis ledger info must exist");

            if fast_sync_db
                .get_latest_ledger_info_option()
                .expect("should returns Ok results")
                .is_none()
            {
                // it means the DB is empty and we need to
                // commit the genesis ledger info to the DB.
                fast_sync_db.commit_genesis_ledger_info(&ledger_info)?;
            }
            let db_backup_service =
                start_backup_service(node_config.storage.backup_service_address, fast_sync_db);
            (db_arc as Arc<dyn DbReader>, db_rw, Some(db_backup_service))
        },
    };
    Ok((
        aptos_db_reader,
        db_rw,
        backup_service,
        internal_indexer_db,
        update_receiver,
    ))
}

/// In consensus-only mode, return a in-memory based [FakeAptosDB] and
/// do not run the backup service.
#[cfg(feature = "consensus-only-perf-test")]
pub(crate) fn bootstrap_db(
    node_config: &NodeConfig,
) -> Result<(Arc<dyn DbReader>, DbReaderWriter, Option<Runtime>)> {
    use aptos_db::db::fake_aptosdb::FakeAptosDB;

    let aptos_db = AptosDB::open(
        node_config.storage.get_dir_paths(),
        false, /* readonly */
        node_config.storage.storage_pruner_config,
        node_config.storage.rocksdb_configs,
        node_config.storage.enable_indexer,
        node_config.storage.buffered_state_target_items,
        node_config.storage.max_num_nodes_per_lru_cache_shard,
    )
    .map_err(|err| anyhow!("DB failed to open {}", err))?;
    let (aptos_db, db_rw) = DbReaderWriter::wrap(FakeAptosDB::new(aptos_db));
    maybe_apply_genesis(&db_rw, node_config)?;
    Ok((aptos_db, db_rw, None))
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
    AptosDB::create_checkpoint(
        &source_dir,
        &checkpoint_dir,
        node_config.storage.rocksdb_configs.enable_storage_sharding,
    )
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
) -> Result<(
    DbReaderWriter,
    Option<Runtime>,
    Waypoint,
    Option<InternalIndexerDB>,
    Option<WatchReceiver<(Instant, Version)>>,
)> {
    // If required, create RocksDB checkpoints and change the working directory.
    // This is test-only.
    if let Some(working_dir) = node_config.base.working_dir.clone() {
        create_rocksdb_checkpoint_and_change_working_dir(node_config, working_dir);
    }

    // Open the database
    let instant = Instant::now();
    let (_aptos_db, db_rw, backup_service, indexer_db_opt, update_receiver) =
        bootstrap_db(node_config)?;

    // Log the duration to open storage
    debug!(
        "Storage service started in {} ms",
        instant.elapsed().as_millis()
    );

    let waypoint = node_config.base.waypoint.waypoint();
    set_waypoint_version(waypoint.version());
    Ok((
        db_rw,
        backup_service,
        waypoint,
        indexer_db_opt,
        update_receiver,
    ))
}
