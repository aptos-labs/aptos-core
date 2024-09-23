// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::db_debugger::common::DbDir;
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
        let latest_version = ledger_db
            .metadata_db()
            .get_synced_version()?
            .expect("DB is empty.");
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

        match db.get_state_value_with_version_by_version(&key, self.version)? {
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
