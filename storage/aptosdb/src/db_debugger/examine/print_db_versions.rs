// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_debugger::ShardingConfig,
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema},
        transaction::TransactionSchema,
        transaction_accumulator::TransactionAccumulatorSchema,
        transaction_info::TransactionInfoSchema,
        version_data::VersionDataSchema,
        write_set::WriteSetSchema,
    },
    utils::truncation_helper::{
        get_current_version_in_state_merkle_db, get_state_kv_commit_progress,
        get_state_merkle_commit_progress,
    },
    AptosDB,
};
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_schemadb::{schema::Schema, DB};
use aptos_storage_interface::Result;
use aptos_types::transaction::Version;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about = "Print the version of each types of data.")]
pub struct Cmd {
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    #[clap(flatten)]
    sharding_config: ShardingConfig,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let rocksdb_config = RocksdbConfigs {
            enable_storage_sharding: self.sharding_config.enable_storage_sharding,
            ..Default::default()
        };
        let (ledger_db, state_merkle_db, state_kv_db) = AptosDB::open_dbs(
            &StorageDirPaths::from_path(&self.db_dir),
            rocksdb_config,
            /*readonly=*/ true,
            /*max_num_nodes_per_lru_cache_shard=*/ 0,
        )?;

        println!(
            "Overall Progress: {:?}",
            ledger_db.metadata_db().get_synced_version()?
        );

        println!(
            "Ledger Progress: {:?}",
            ledger_db.metadata_db().get_ledger_commit_progress()?
        );

        println!(
            "StateKv Progress: {:?}",
            get_state_kv_commit_progress(&state_kv_db)?
        );

        println!(
            "StateMerkle Progress: {:?}",
            get_state_merkle_commit_progress(&state_merkle_db)?
        );

        println!(
            "LedgerPruner Progress: {:?}",
            ledger_db.metadata_db().get_pruner_progress()?
        );

        println!(
            "StateKvPruner Progress: {:?}",
            state_kv_db
                .metadata_db()
                .get::<DbMetadataSchema>(&DbMetadataKey::StateKvPrunerProgress)?
                .map_or(0, |v| v.expect_version())
        );

        println!(
            "StateMerklePruner Progress: {:?}",
            state_merkle_db
                .metadata_db()
                .get::<DbMetadataSchema>(&DbMetadataKey::StateMerklePrunerProgress)?
                .map_or(0, |v| v.expect_version())
        );

        println!(
            "EpochEndingStateMerkle Pruner Progress: {:?}",
            state_merkle_db
                .metadata_db()
                .get::<DbMetadataSchema>(&DbMetadataKey::EpochEndingStateMerklePrunerProgress)?
                .map_or(0, |v| v.expect_version())
        );

        println!(
            "Current ledger info: {:?}",
            ledger_db.metadata_db().get_latest_ledger_info_option()
        );

        println!(
            "Max JMT node version: {:?}",
            get_current_version_in_state_merkle_db(&state_merkle_db)?,
        );

        println!(
            "Max TransactionInfo version: {:?}",
            Self::get_latest_version_for_schema::<TransactionInfoSchema>(
                ledger_db.transaction_info_db_raw()
            )?,
        );

        println!(
            "Max Transaction version: {:?}",
            Self::get_latest_version_for_schema::<TransactionSchema>(
                ledger_db.transaction_db_raw()
            )?,
        );

        println!(
            "Max VersionData version: {:?}",
            Self::get_latest_version_for_schema::<VersionDataSchema>(&ledger_db.metadata_db_arc())?,
        );

        println!(
            "Max WriteSet version: {:?}",
            Self::get_latest_version_for_schema::<WriteSetSchema>(ledger_db.write_set_db_raw())?,
        );

        {
            let mut iter = ledger_db
                .transaction_accumulator_db_raw()
                .iter::<TransactionAccumulatorSchema>()?;
            iter.seek_to_last();
            let position = iter.next().transpose()?.map(|kv| kv.0);
            let num_frozen_nodes = position.map(|p| p.to_postorder_index() + 1);
            println!(
                "# of frozen nodes in TransactionAccumulator: {:?}",
                num_frozen_nodes
            );
        }

        Ok(())
    }

    fn get_latest_version_for_schema<S>(db: &DB) -> Result<Option<Version>>
    where
        S: Schema<Key = Version>,
    {
        let mut iter = db.iter::<S>()?;
        iter.seek_to_last();
        Ok(iter.next().transpose()?.map(|kv| kv.0))
    }
}
