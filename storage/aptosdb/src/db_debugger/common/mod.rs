// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{ledger_db::LedgerDb, state_kv_db::StateKvDb, state_merkle_db::StateMerkleDb};
use aptos_config::config::{RocksdbConfig, StorageDirPaths};
use aptos_storage_interface::Result;
use aptos_types::nibble::{nibble_path::NibblePath, Nibble};
use clap::Parser;
use std::path::{Path, PathBuf};

pub const PAGE_SIZE: usize = 10;

#[derive(Parser, Clone)]
pub struct DbDir {
    // TODO(grao): Support path override here.
    #[clap(long, value_parser)]
    db_dir: PathBuf,
}

impl DbDir {
    pub fn open_state_merkle_db(&self) -> Result<StateMerkleDb> {
        let env = None;
        let block_cache = None;
        StateMerkleDb::new(
            &StorageDirPaths::from_path(&self.db_dir),
            RocksdbConfig::default(),
            env,
            block_cache,
            /* read_only = */ false,
            /* max_nodes_per_lru_cache_shard = */ 0,
            /* is_hot = */ false,
            /* delete_on_restart = */ false,
        )
    }

    pub fn open_state_kv_db(&self) -> Result<StateKvDb> {
        let env = None;
        let block_cache = None;
        StateKvDb::new(
            &StorageDirPaths::from_path(&self.db_dir),
            RocksdbConfig::default(),
            env,
            block_cache,
            true,
        )
    }

    pub fn open_ledger_db(&self) -> Result<LedgerDb> {
        let env = None;
        let block_cache = None;
        LedgerDb::new(
            self.db_dir.as_path(),
            RocksdbConfig::default(),
            env,
            block_cache,
            true,
        )
    }
}

impl AsRef<Path> for DbDir {
    fn as_ref(&self) -> &Path {
        self.db_dir.as_path()
    }
}

pub fn parse_nibble_path(src: &str) -> Result<NibblePath> {
    src.chars()
        .map(|c| Ok(Nibble::from(u8::from_str_radix(&c.to_string(), 16)?)))
        .collect()
}
