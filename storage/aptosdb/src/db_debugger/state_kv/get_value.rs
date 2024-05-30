// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{db_debugger::common::DbDir, schema::state_value::StateValueSchema};
use aptos_schemadb::ReadOptions;
use aptos_storage_interface::Result;
use aptos_types::{state_store::state_key::StateKey, transaction::Version};
use clap::Parser;
use owo_colors::OwoColorize;

#[derive(Parser)]
#[clap(about = "Print state value by given key and version.")]
pub struct Cmd {
    #[clap(flatten)]
    db_dir: DbDir,

    #[clap(long)]
    key_hex: String,

    #[clap(long)]
    version: Version,

    #[clap(long)]
    max_skips: u64,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let key_vec = hex::decode(&self.key_hex).unwrap();
        let key: StateKey = bcs::from_bytes(&key_vec)?;
        println!(
            "{}",
            format!(
                "* Get state value for key {:?} at version {}. \n",
                key, self.version,
            )
            .yellow()
        );

        let ledger_db = self.db_dir.open_ledger_db()?;
        let db = self.db_dir.open_state_kv_db()?;
        let latest_version = ledger_db.metadata_db().get_synced_version()?;
        println!("latest version: {latest_version}");
        if self.version != Version::MAX && self.version > latest_version {
            println!(
                "{}",
                format!(
                    "warning: version {} is greater than latest version {}",
                    self.version, latest_version
                )
                .red()
            );
        }

        let mut read_opts = ReadOptions::default();
        // We want `None` if the state_key changes in iteration.
        read_opts.set_prefix_same_as_start(true);
        read_opts.set_max_skippable_internal_keys(self.max_skips);
        let mut iter = db
            .db_shard(key.get_shard_id())
            .iter::<StateValueSchema>(read_opts)?;
        iter.seek(&(key.clone(), self.version))?;
        let res = iter
            .next()
            .transpose()?
            .and_then(|((_, version), value_opt)| value_opt.map(|value| (version, value)));

        match res {
            None => {
                println!("{}", "Value not found.".to_string().yellow());
            },
            Some((version, value)) => {
                println!("{}", "Value found:".to_string().yellow());
                println!("   version: {version}");
                if value.bytes().len() > 1024 {
                    println!("     value: {} bytes", value.bytes().len())
                } else {
                    println!("     value: {:?}", value.bytes())
                }
                println!("  metadata: {:?}", value.into_metadata());
            },
        }

        Ok(())
    }
}
