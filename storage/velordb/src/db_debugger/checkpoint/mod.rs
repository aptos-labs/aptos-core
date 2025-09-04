// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_debugger::common::DbDir, VelorDB};
use velor_storage_interface::{db_ensure as ensure, VelorDbError, Result};
use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[clap(about = "Make a DB checkpoint by hardlinks.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long, value_parser)]
    output_dir: PathBuf,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        ensure!(!self.output_dir.exists(), "Output dir already exists.");
        fs::create_dir_all(&self.output_dir)?;
        let sharding_config = self.db_dir.sharding_config.clone();
        VelorDB::create_checkpoint(
            self.db_dir,
            self.output_dir,
            sharding_config.enable_storage_sharding,
        )
    }
}
