// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_config::config::RocksdbConfig;
use aptos_db_indexer_schemas::schema::{column_families, internal_indexer_column_families};
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::DB;
use std::{mem, path::Path};

const INTERNAL_INDEXER_DB_NAME: &str = "internal_indexer_db";
const TABLE_INFO_DB_NAME: &str = "index_async_v2_db";

pub fn open_db<P: AsRef<Path>>(
    db_path: P,
    rocksdb_config: &RocksdbConfig,
    readonly: bool,
) -> Result<DB> {
    let env = None;
    if readonly {
        Ok(DB::open_readonly(
            db_path,
            TABLE_INFO_DB_NAME,
            column_families(),
            gen_rocksdb_options(rocksdb_config, env, readonly),
        )?)
    } else {
        Ok(DB::open(
            db_path,
            TABLE_INFO_DB_NAME,
            column_families(),
            gen_rocksdb_options(rocksdb_config, env, readonly),
        )?)
    }
}

pub fn open_internal_indexer_db<P: AsRef<Path>>(
    db_path: P,
    rocksdb_config: &RocksdbConfig,
) -> Result<DB> {
    let env = None;
    Ok(DB::open(
        db_path,
        INTERNAL_INDEXER_DB_NAME,
        internal_indexer_column_families(),
        gen_rocksdb_options(rocksdb_config, env, false),
    )?)
}

pub fn close_db(db: DB) {
    mem::drop(db)
}
