// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::schema::column_families;
use anyhow::Result;
use aptos_config::config::RocksdbConfig;
use aptos_rocksdb_options::gen_rocksdb_options;
use aptos_schemadb::{
    schema::{KeyCodec, Schema, ValueCodec},
    SchemaBatch, DB,
};
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

pub fn read_db<K, V, S>(db: &DB, key: &K) -> Result<Option<V>>
where
    K: KeyCodec<S>,
    V: ValueCodec<S>,
    S: Schema<Key = K, Value = V>,
{
    Ok(db.get::<S>(key)?)
}

pub fn write_db<K, V, S>(db: &DB, key: K, value: V) -> Result<()>
where
    K: KeyCodec<S>,
    V: ValueCodec<S>,
    S: Schema<Key = K, Value = V>,
{
    let batch = SchemaBatch::new();
    batch.put::<S>(&key, &value)?;
    Ok(db.write_schemas(batch)?)
}
