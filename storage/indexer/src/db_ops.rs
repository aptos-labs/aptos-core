// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::column_families;
use anyhow::Result;
use aptos_config::config::RocksdbConfig;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::DB;
use std::{mem, path::Path};

pub fn open_db<P: AsRef<Path>>(db_path: P, rocksdb_config: &RocksdbConfig) -> Result<DB> {
    Ok(DB::open(
        db_path,
        "index_asnync_v2_db",
        column_families(),
        &gen_rocksdb_options(rocksdb_config, false),
    )?)
}

pub fn close_db(db: DB) {
    mem::drop(db)
}
