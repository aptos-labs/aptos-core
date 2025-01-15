// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, format_err, Context, Result};
use aptos_config::config::{
    RocksdbConfigs, StorageDirPaths, BUFFERED_STATE_TARGET_ITEMS,
    DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
};
use aptos_db::AptosDB;
use aptos_executor::db_bootstrapper::calculate_genesis;
use aptos_storage_interface::DbReaderWriter;
use aptos_types::{transaction::Transaction, waypoint::Waypoint};
use aptos_vm::aptos_vm::AptosVMBlockExecutor;
use clap::Parser;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Parser)]
#[clap(
    name = "aptos-db-bootstrapper",
    about = "Calculate, verify and commit the genesis to local DB without a consensus among validators."
)]
pub struct Command {
    #[clap(value_parser)]
    db_dir: PathBuf,

    #[clap(short, long, value_parser)]
    genesis_txn_file: PathBuf,

    #[clap(short, long)]
    waypoint_to_verify: Option<Waypoint>,

    #[clap(long, requires("waypoint_to_verify"))]
    commit: bool,
}

impl Command {
    pub fn run(self) -> Result<()> {
        let genesis_txn = load_genesis_txn(&self.genesis_txn_file)
            .with_context(|| format_err!("Failed loading genesis txn."))?;
        assert!(
            matches!(genesis_txn, Transaction::GenesisTransaction(_)),
            "Not a GenesisTransaction"
        );

        // Opening the DB exclusively, it's not allowed to run this tool alongside a running node which
        // operates on the same DB.
        let db = AptosDB::open(
            StorageDirPaths::from_path(&self.db_dir),
            false,
            NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
            RocksdbConfigs::default(),
            false, /* indexer */
            BUFFERED_STATE_TARGET_ITEMS,
            DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
            None,
        )
        .expect("Failed to open DB.");
        let db = DbReaderWriter::new(db);

        let ledger_summary = db
            .reader
            .get_pre_committed_ledger_summary()
            .with_context(|| format_err!("Failed to get latest tree state."))?;
        println!("Db has {} transactions", ledger_summary.next_version());
        if let Some(waypoint) = self.waypoint_to_verify {
            ensure!(
                waypoint.version() == ledger_summary.next_version(),
                "Trying to generate waypoint at version {}, but DB has {} transactions.",
                waypoint.version(),
                ledger_summary.next_version(),
            )
        }

        let committer =
            calculate_genesis::<AptosVMBlockExecutor>(&db, ledger_summary, &genesis_txn)
                .with_context(|| format_err!("Failed to calculate genesis."))?;
        println!(
            "Successfully calculated genesis. Got waypoint: {}",
            committer.waypoint()
        );

        if let Some(waypoint) = self.waypoint_to_verify {
            ensure!(
                waypoint == committer.waypoint(),
                "Waypoint verification failed. Expected {:?}, got {:?}.",
                waypoint,
                committer.waypoint(),
            );
            println!("Waypoint verified.");

            if self.commit {
                committer
                    .commit()
                    .with_context(|| format_err!("Committing genesis to DB."))?;
                println!("Successfully committed genesis.")
            }
        }

        Ok(())
    }
}

fn load_genesis_txn(path: &Path) -> Result<Transaction> {
    let mut file = File::open(path)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;

    Ok(bcs::from_bytes(&buffer)?)
}

#[test]
fn verify_tool() {
    use clap::CommandFactory;
    Command::command().debug_assert()
}
