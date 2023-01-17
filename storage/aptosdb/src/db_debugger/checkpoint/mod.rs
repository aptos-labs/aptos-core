// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{db_debugger::common::DbDir, LEDGER_DB_NAME, STATE_MERKLE_DB_NAME};
use anyhow::{ensure, Result};
use clap::Parser;
use std::{fs, path::PathBuf};

#[derive(Parser)]
#[clap(about = "Make a DB checkpoint by hardlinks.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long, parse(from_os_str))]
    output_dir: PathBuf,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        ensure!(!self.output_dir.exists(), "Output dir already exists.");
        fs::create_dir_all(&self.output_dir)?;

        // Weird enough, checkpoint doesn't work with readonly or secondary mode (gets stuck).
        // https://github.com/facebook/rocksdb/issues/11167
        {
            let ledger_db = self.db_dir.open_ledger_db_rw()?;
            ledger_db.create_checkpoint(&self.output_dir.join(LEDGER_DB_NAME))?;
        }
        {
            let state_merkle_db = self.db_dir.open_state_merkle_db_rw()?;
            state_merkle_db.create_checkpoint(&self.output_dir.join(STATE_MERKLE_DB_NAME))?;
        }

        Ok(())
    }
}
