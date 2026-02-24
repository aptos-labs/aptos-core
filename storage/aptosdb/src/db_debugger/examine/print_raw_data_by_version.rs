// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::AptosDB;
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_storage_interface::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about = "Print the version of each types of data.")]
pub struct Cmd {
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    #[clap(long)]
    version: u64,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let rocksdb_config = RocksdbConfigs::default();
        let env = None;
        let block_cache = None;

        let (ledger_db, _, _, _, _) = AptosDB::open_dbs(
            &StorageDirPaths::from_path(&self.db_dir),
            rocksdb_config,
            env,
            block_cache,
            /*readonly=*/ true,
            /*max_num_nodes_per_lru_cache_shard=*/ 0,
            /*reset_hot_state=*/ false,
        )?;

        println!(
            "Transaction: {:?}",
            ledger_db.transaction_db().get_transaction(self.version)?
        );

        println!(
            "PersistedAuxiliaryInfo: {:?}",
            ledger_db
                .persisted_auxiliary_info_db()
                .get_persisted_auxiliary_info(self.version)?
        );

        println!(
            "WriteSet: {:?}",
            ledger_db.write_set_db().get_write_set(self.version)?
        );

        println!(
            "Events: {:?}",
            ledger_db.event_db().get_events_by_version(self.version)?
        );

        println!(
            "TransactionInfo: {:?}",
            ledger_db
                .transaction_info_db()
                .get_transaction_info(self.version)?
        );

        println!(
            "TransactionAccumulatorHash: {:?}",
            ledger_db
                .transaction_accumulator_db()
                .get_root_hash(self.version)?
        );

        Ok(())
    }
}
