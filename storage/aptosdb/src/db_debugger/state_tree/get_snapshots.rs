// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_debugger::common::{DbDir, PAGE_SIZE},
    jellyfish_merkle_node::JellyfishMerkleNodeSchema,
};
use anyhow::Result;
use aptos_jellyfish_merkle::node_type::NodeKey;
use aptos_types::transaction::Version;
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
            let mut iter = db.rev_iter::<JellyfishMerkleNodeSchema>(Default::default())?;

            let mut version = self.next_version - 1;
            for n in 0..PAGE_SIZE {
                iter.seek_for_prev(&NodeKey::new_empty_path(version))?;
                if let Some((key, _node)) = iter.next().transpose()? {
                    println!("{} {}", n, key.version());
                    if key.version() == 0 {
                        break;
                    }
                    version = key.version() - 1;
                } else {
                    break;
                }
            }
        }

        Ok(())
    }
}
