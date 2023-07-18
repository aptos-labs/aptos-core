// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    schema::{
        db_metadata::{DbMetadataKey, DbMetadataSchema},
        event_accumulator::EventAccumulatorSchema,
        ledger_info::LedgerInfoSchema,
        transaction::TransactionSchema,
        transaction_accumulator::TransactionAccumulatorSchema,
        transaction_info::TransactionInfoSchema,
        version_data::VersionDataSchema,
        write_set::WriteSetSchema,
    },
    utils::truncation_helper::{
        get_current_version_in_state_merkle_db, get_ledger_commit_progress,
        get_overall_commit_progress, get_state_kv_commit_progress,
        get_state_merkle_commit_progress,
    },
    AptosDB,
};
use anyhow::Result;
use aptos_config::config::RocksdbConfigs;
use aptos_schemadb::{schema::Schema, ReadOptions, DB};
use aptos_types::transaction::Version;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about = "Print the version of each types of data.")]
pub struct Cmd {
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    #[clap(long)]
    split_ledger_db: bool,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let rocksdb_config = RocksdbConfigs {
            split_ledger_db: self.split_ledger_db,
            ..Default::default()
        };
        let (ledger_db, state_merkle_db, state_kv_db) = AptosDB::open_dbs(
            &self.db_dir,
            rocksdb_config,
            /*readonly=*/ true,
            /*max_num_nodes_per_lru_cache_shard=*/ 0,
        )?;

        println!(
            "Overall Progress: {:?}",
            get_overall_commit_progress(ledger_db.metadata_db())?
        );

        println!(
            "Ledger Progress: {:?}",
            get_ledger_commit_progress(ledger_db.metadata_db())?
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
            ledger_db
                .metadata_db()
                .get::<DbMetadataSchema>(&DbMetadataKey::LedgerPrunerProgress)?
                .map_or(0, |v| v.expect_version())
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

        {
            let mut iter = ledger_db
                .metadata_db()
                .iter::<LedgerInfoSchema>(ReadOptions::default())?;
            iter.seek_to_last();
            println!("Current ledger info: {:?}", iter.next().transpose()?);
        }

        println!(
            "Max JMT node version: {:?}",
            get_current_version_in_state_merkle_db(&state_merkle_db)?,
        );

        println!(
            "Max TransactionInfo version: {:?}",
            Self::get_latest_version_for_schema::<TransactionInfoSchema>(
                ledger_db.transaction_info_db()
            )?,
        );

        println!(
            "Max Transaction version: {:?}",
            Self::get_latest_version_for_schema::<TransactionSchema>(ledger_db.transaction_db())?,
        );

        println!(
            "Max VersionData version: {:?}",
            Self::get_latest_version_for_schema::<VersionDataSchema>(ledger_db.metadata_db())?,
        );

        println!(
            "Max WriteSet version: {:?}",
            Self::get_latest_version_for_schema::<WriteSetSchema>(ledger_db.write_set_db())?,
        );

        {
            let mut iter = ledger_db
                .transaction_accumulator_db()
                .iter::<TransactionAccumulatorSchema>(ReadOptions::default())?;
            iter.seek_to_last();
            let position = iter.next().transpose()?.map(|kv| kv.0);
            let num_frozen_nodes = position.map(|p| p.to_postorder_index() + 1);
            println!(
                "# of frozen nodes in TransactionAccumulator: {:?}",
                num_frozen_nodes
            );
        }

        {
            let mut iter = ledger_db
                .event_db()
                .iter::<EventAccumulatorSchema>(ReadOptions::default())?;
            iter.seek_to_last();
            let key = iter.next().transpose()?.map(|kv| kv.0);
            let version = key.map(|k| k.0);
            println!("Max EventAccumulator version: {:?}", version)
        }

        Ok(())
    }

    fn get_latest_version_for_schema<S>(db: &DB) -> Result<Option<Version>>
    where
        S: Schema<Key = Version>,
    {
        let mut iter = db.iter::<S>(ReadOptions::default())?;
        iter.seek_to_last();
        Ok(iter.next().transpose()?.map(|kv| kv.0))
    }
}
