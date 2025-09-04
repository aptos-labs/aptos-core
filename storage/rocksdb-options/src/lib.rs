// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_config::config::{RocksDBStatsLevel, RocksdbConfig};
use rocksdb::{statistics::StatsLevel, Options};

// TODO: Clean this up. It is currently separated into its own crate
// to avoid circular dependencies, because it depends on aptos-config (which
// is widely used).

fn convert_stats_level(level: RocksDBStatsLevel) -> StatsLevel {
    use RocksDBStatsLevel::*;
    match level {
        DisableAll => StatsLevel::DisableAll,
        ExceptHistogramOrTimers => StatsLevel::ExceptHistogramOrTimers,
        ExceptTimers => StatsLevel::ExceptTimers,
        ExceptDetailedTimers => StatsLevel::ExceptDetailedTimers,
        ExceptTimeForMutex => StatsLevel::ExceptTimeForMutex,
        All => StatsLevel::All,
    }
}

pub fn gen_rocksdb_options(config: &RocksdbConfig, readonly: bool) -> Options {
    let mut db_opts = Options::default();
    db_opts.set_max_open_files(config.max_open_files);
    db_opts.set_max_total_wal_size(config.max_total_wal_size);
    db_opts.set_max_background_jobs(config.max_background_jobs);
    db_opts.set_statistics_level(convert_stats_level(config.stats_level));
    db_opts.set_stats_dump_period_sec(config.stats_dump_period_sec);
    if !readonly {
        db_opts.create_if_missing(true);
        db_opts.create_missing_column_families(true);
    }

    db_opts
}
