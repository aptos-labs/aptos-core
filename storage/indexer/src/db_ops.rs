// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use aptos_config::config::RocksdbConfig;
use aptos_db_indexer_schemas::schema::{
    column_families, gen_internal_indexer_cfds, internal_indexer_column_families,
};
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::DB;
use std::{mem, path::Path};

const INTERNAL_INDEXER_DB_NAME: &str = "internal_indexer_db";
const TABLE_INFO_DB_NAME: &str = "index_async_v2_db";

pub fn open_db<P: AsRef<Path>>(db_path: P, rocksdb_config: &RocksdbConfig) -> Result<DB> {
    Ok(DB::open(
        db_path,
        TABLE_INFO_DB_NAME,
        column_families(),
        &gen_rocksdb_options(rocksdb_config, false),
    )?)
}

pub fn open_internal_indexer_db<P: AsRef<Path>>(
    db_path: P,
    rocksdb_config: &RocksdbConfig,
) -> Result<DB> {
    Ok(DB::open_cf(
        &gen_rocksdb_options(rocksdb_config, false),
        db_path,
        INTERNAL_INDEXER_DB_NAME,
        gen_internal_indexer_cfds(rocksdb_config, internal_indexer_column_families()),
    )?)
}

pub fn close_db(db: DB) {
    mem::drop(db)
}
