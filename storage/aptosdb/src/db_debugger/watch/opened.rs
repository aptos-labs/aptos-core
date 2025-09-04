// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{AptosDB, db_debugger::ShardingConfig};
use aptos_config::config::StorageConfig;
use aptos_storage_interface::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[clap(about = "Print the version of each types of data.")]
pub struct Cmd {
    #[clap(long, value_parser)]
    db_dir: PathBuf,

    #[clap(flatten)]
    sharding_config: ShardingConfig,
}

impl Cmd {
    pub fn run(self) -> Result<()> {
        let mut config = StorageConfig::default();
        config.set_data_dir(self.db_dir);
        config.rocksdb_configs.enable_storage_sharding =
            self.sharding_config.enable_storage_sharding;

        let _db = AptosDB::open(
            config.get_dir_paths(),
            false, /* readonly */
            config.storage_pruner_config,
            config.rocksdb_configs,
            config.enable_indexer,
            config.buffered_state_target_items,
            config.max_num_nodes_per_lru_cache_shard,
            None,
        )
        .expect("Failed to open AptosDB");

        println!("AptosDB opened. Kill to exit.");

        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));
        }
    }
}
