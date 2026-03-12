// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db::AptosDB,
    utils::truncation_helper::{
        get_current_version_in_state_merkle_db, get_state_kv_commit_progress,
    },
};
use aptos_config::config::{RocksdbConfigs, StorageDirPaths};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use claims::assert_le;
use clap::Parser;
use std::{fs, path::PathBuf, sync::Arc};

#[derive(Parser)]
#[clap(about = "Delete all data after the provided version.")]
#[clap(group(clap::ArgGroup::new("backup")
        .required(true)
        .args(&["backup_checkpoint_dir", "opt_out_backup_checkpoint"]),
))]
pub struct Cmd {
    // TODO(grao): Support db_path_overrides here.
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    #[clap(long)]
    target_version: u64,

    #[clap(long, default_value_t = 1000)]
    ledger_db_batch_size: usize,

    #[clap(long, value_parser, group = "backup")]
    backup_checkpoint_dir: Option<PathBuf>,

    #[clap(long, group = "backup")]
    opt_out_backup_checkpoint: bool,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        if !self.opt_out_backup_checkpoint {
            let backup_checkpoint_dir = self.backup_checkpoint_dir.unwrap();
            ensure!(
                !backup_checkpoint_dir.exists(),
                "Backup dir already exists."
            );
            println!("Creating backup at: {:?}", &backup_checkpoint_dir);
            fs::create_dir_all(&backup_checkpoint_dir)?;
            AptosDB::create_checkpoint(&self.db_dir, backup_checkpoint_dir)?;
            println!("Done!");
        } else {
            println!("Opted out backup creation!.");
        }

        let db_dir = StorageDirPaths::from_path(&self.db_dir);

        // Open DBs to validate versions and resolve target_version.
        let mut target_version = self.target_version;
        {
            let (ledger_db, _hot_state_merkle_db, state_merkle_db, _hot_state_kv_db, state_kv_db) =
                AptosDB::open_dbs(
                    &db_dir,
                    RocksdbConfigs::default(),
                    None,
                    None,
                    /*readonly=*/ false,
                    /*max_num_nodes_per_lru_cache_shard=*/ 0,
                    /*reset_hot_state=*/ true,
                )?;

            let ledger_db = Arc::new(ledger_db);
            let state_merkle_db = Arc::new(state_merkle_db);
            let state_kv_db = Arc::new(state_kv_db);

            let overall_version = ledger_db
                .metadata_db()
                .get_synced_version()
                .expect("DB read failed.")
                .expect("Overall commit progress must exist.");
            let ledger_db_version = ledger_db
                .metadata_db()
                .get_ledger_commit_progress()
                .expect("Current version of ledger db must exist.");
            let state_kv_db_version = get_state_kv_commit_progress(&state_kv_db)?
                .expect("Current version of state kv db must exist.");
            let state_merkle_db_version = get_current_version_in_state_merkle_db(&state_merkle_db)?
                .expect("Current version of state merkle db must exist.");

            assert_le!(overall_version, ledger_db_version);
            assert_le!(overall_version, state_kv_db_version);
            assert_le!(state_merkle_db_version, overall_version);
            assert_le!(target_version, overall_version);

            println!(
                "overall_version: {}, ledger_db_version: {}, state_kv_db_version: {}, \
                 state_merkle_db_version: {}, target_version: {}",
                overall_version,
                ledger_db_version,
                state_kv_db_version,
                state_merkle_db_version,
                target_version,
            );

            if ledger_db.metadata_db().get_usage(target_version).is_err() {
                println!(
                    "Unable to truncate to version {}, since there is no VersionData on that version.",
                    target_version
                );
                println!(
                    "Trying to fallback to the largest valid version before version {}.",
                    target_version,
                );
                target_version = ledger_db
                    .metadata_db()
                    .get_usage_before_or_at(target_version)?
                    .0;
            }
        }

        println!("Starting db truncation to version {}...", target_version);
        AptosDB::truncate_to_version(&db_dir, target_version)?;
        println!("Done!");

        Ok(())
    }
}
