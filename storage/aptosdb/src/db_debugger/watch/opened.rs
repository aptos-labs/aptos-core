// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::AptosDB;
use aptos_config::config::StorageConfig;
use aptos_storage_interface::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about = "Print the version of each types of data.")]
pub struct Cmd {
    #[clap(long, value_parser)]
    db_dir: PathBuf,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let mut config = StorageConfig::default();
        config.set_data_dir(self.db_dir);
        config.hot_state_config.delete_on_restart = false;

        let _db = AptosDB::open(
            config.get_dir_paths(),
            false, /* readonly */
            config.storage_pruner_config,
            config.rocksdb_configs,
            config.buffered_state_target_items,
            config.max_num_nodes_per_lru_cache_shard,
            None,
            config.hot_state_config,
        )
        .expect("Failed to open AptosDB");

        println!("AptosDB opened. Kill to exit.");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
