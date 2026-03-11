// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{db::AptosDB, utils::truncation_helper::run_truncation};
use aptos_storage_interface::{db_ensure as ensure, AptosDbError, Result};
use clap::Parser;
use std::{fs, path::PathBuf};

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

        run_truncation(&self.db_dir, self.target_version)
    }
}
