// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use anyhow::{ensure, format_err, Context, Result};
use aptos_config::config::{
    RocksdbConfigs, DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD, NO_OP_STORAGE_PRUNER_CONFIG,
    TARGET_SNAPSHOT_SIZE,
};
use aptos_types::{transaction::Transaction, waypoint::Waypoint};
use aptos_vm::AptosVM;
use aptosdb::AptosDB;
use executor::db_bootstrapper::calculate_genesis;
use std::{
    fs::File,
    io::Read,
    path::{Path, PathBuf},
};
use storage_interface::DbReaderWriter;
use structopt::StructOpt;

#[derive(StructOpt)]
#[structopt(
    name = "db-bootstrapper",
    about = "Calculate, verify and commit the genesis to local DB without a consensus among validators."
)]
struct Opt {
    #[structopt(parse(from_os_str))]
    db_dir: PathBuf,

    #[structopt(short, long, parse(from_os_str))]
    genesis_txn_file: PathBuf,

    #[structopt(short, long)]
    waypoint_to_verify: Option<Waypoint>,

    #[structopt(long, requires("waypoint-to-verify"))]
    commit: bool,
}

fn main() -> Result<()> {
    let opt = Opt::from_args();

    let genesis_txn = load_genesis_txn(&opt.genesis_txn_file)
        .with_context(|| format_err!("Failed loading genesis txn."))?;
    assert!(
        matches!(genesis_txn, Transaction::GenesisTransaction(_)),
        "Not a GenesisTransaction"
    );

    // Opening the DB exclusively, it's not allowed to run this tool alongside a running node which
    // operates on the same DB.
    let db = AptosDB::open(
        &opt.db_dir,
        false,
        NO_OP_STORAGE_PRUNER_CONFIG, /* pruner */
        RocksdbConfigs::default(),
        false, /* indexer */
        TARGET_SNAPSHOT_SIZE,
        DEFAULT_MAX_NUM_NODES_PER_LRU_CACHE_SHARD,
    )
    .expect("Failed to open DB.");
    let db = DbReaderWriter::new(db);

    let executed_trees = db
        .reader
        .get_latest_executed_trees()
        .with_context(|| format_err!("Failed to get latest tree state."))?;
    println!("Db has {} transactions", executed_trees.num_transactions());
    if let Some(waypoint) = opt.waypoint_to_verify {
        ensure!(
            waypoint.version() == executed_trees.num_transactions(),
            "Trying to generate waypoint at version {}, but DB has {} transactions.",
            waypoint.version(),
            executed_trees.num_transactions(),
        )
    }

    let committer = calculate_genesis::<AptosVM>(&db, executed_trees, &genesis_txn)
        .with_context(|| format_err!("Failed to calculate genesis."))?;
    println!(
        "Successfully calculated genesis. Got waypoint: {}",
        committer.waypoint()
    );

    if let Some(waypoint) = opt.waypoint_to_verify {
        ensure!(
            waypoint == committer.waypoint(),
            "Waypoint verification failed. Expected {:?}, got {:?}.",
            waypoint,
            committer.waypoint(),
        );
        println!("Waypoint verified.");

        if opt.commit {
            committer
                .commit()
                .with_context(|| format_err!("Committing genesis to DB."))?;
            println!("Successfully committed genesis.")
        }
    }

    Ok(())
}

fn load_genesis_txn(path: &Path) -> Result<Transaction> {
    let mut file = File::open(&path)?;
    let mut buffer = vec![];
    file.read_to_end(&mut buffer)?;

    Ok(bcs::from_bytes(&buffer)?)
}
