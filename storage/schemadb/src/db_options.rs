// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::RocksdbConfig;
use rocksdb::Options;

pub fn gen_rocksdb_options(config: &RocksdbConfig, readonly: bool) -> Options {
    let mut db_opts = Options::default();
    db_opts.set_max_open_files(config.max_open_files);
    db_opts.set_max_total_wal_size(config.max_total_wal_size);
    db_opts.set_max_background_jobs(config.max_background_jobs);
    if !readonly {
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
    }

    db_opts
}
