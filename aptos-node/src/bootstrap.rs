// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{anyhow, Result};
use aptos_config::config::NodeConfig;
use aptos_db::AptosDB;
use aptos_storage_interface::DbReaderWriter;
use std::sync::Arc;
use tokio::runtime::Runtime;

#[cfg(not(feature = "consensus-only-perf-test"))]
pub(crate) fn bootstrap_db(
    node_config: &NodeConfig,
) -> Result<(Arc<AptosDB>, DbReaderWriter, Option<Runtime>)> {
    use aptos_backup_service::start_backup_service;

    let (aptos_db, db_rw) = DbReaderWriter::wrap(
        AptosDB::open(
            &node_config.storage.dir(),
            false, /* readonly */
            node_config.storage.storage_pruner_config,
            node_config.storage.rocksdb_configs,
            node_config.storage.enable_indexer,
            node_config.storage.buffered_state_target_items,
            node_config.storage.max_num_nodes_per_lru_cache_shard,
        )
        .map_err(|err| anyhow!("DB failed to open {}", err))?,
    );
    let db_backup_service =
        start_backup_service(node_config.storage.backup_service_address, aptos_db.clone());
    Ok((aptos_db, db_rw, Some(db_backup_service)))
}

#[cfg(feature = "consensus-only-perf-test")]
pub(crate) fn bootstrap_db(
    node_config: &NodeConfig,
) -> Result<(
    Arc<aptos_db::fake_aptosdb::FakeAptosDB>,
    DbReaderWriter,
    Option<Runtime>,
)> {
    use aptos_db::fake_aptosdb::FakeAptosDB;

    let (aptos_db, db_rw) = DbReaderWriter::wrap(FakeAptosDB::new(
        AptosDB::open(
            &node_config.storage.dir(),
            false, /* readonly */
            node_config.storage.storage_pruner_config,
            node_config.storage.rocksdb_configs,
            node_config.storage.enable_indexer,
            node_config.storage.buffered_state_target_items,
            node_config.storage.max_num_nodes_per_lru_cache_shard,
        )
        .map_err(|err| anyhow!("DB failed to open {}", err))?,
    ));
    Ok((aptos_db, db_rw, None))
}
