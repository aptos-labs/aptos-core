// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::db_debugger::common::{DbDir, PAGE_SIZE};
use anyhow::Result;
use velor_types::transaction::Version;
use clap::Parser;

#[derive(Parser)]
#[clap(about = "List state snapshots before input version.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long, default_value_t = 18446744073709551615)]
    next_version: Version,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        println!(
            "* Looking for state snapshots strictly before version {}. \n",
            self.next_version
        );

        if self.next_version > 0 {
            let db = self.db_dir.open_state_merkle_db()?;

            let mut version = self.next_version - 1;
            for n in 0..PAGE_SIZE {
                let res = db.get_state_snapshot_version_before(version)?;

                if let Some(ver) = res {
                    println!("{} {}", n, ver);
                    if ver == 0 {
                        break;
                    }
                    version = ver - 1;
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
