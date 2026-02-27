// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    db_options::{
        event_db_column_families, ledger_metadata_db_column_families, skip_reporting_cf,
        state_kv_db_new_key_column_families, state_merkle_db_column_families,
        transaction_accumulator_db_column_families, transaction_db_column_families,
        transaction_info_db_column_families, write_set_db_column_families,
    },
    ledger_db::LedgerDb,
    metrics::{
        OTHER_TIMERS_SECONDS, ROCKSDB_PROPERTIES, ROCKSDB_SHARD_PROPERTIES, ROCKSDB_TICKERS,
        SHARD_NAME_BY_ID,
    },
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
};
use anyhow::Result;
use aptos_infallible::Mutex;
use aptos_logger::prelude::*;
use aptos_metrics_core::TimerHelper;
use aptos_schemadb::{ColumnFamilyName, Ticker, DB};
use aptos_types::state_store::NUM_STATE_SHARDS;
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{mpsc, Arc},
    thread,
    thread::JoinHandle,
    time::Duration,
};

static ROCKSDB_PROPERTY_MAP: Lazy<HashMap<&str, String>> = Lazy::new(|| {
    [
        "rocksdb.num-immutable-mem-table",
        "rocksdb.mem-table-flush-pending",
        "rocksdb.compaction-pending",
        "rocksdb.background-errors",
        "rocksdb.cur-size-active-mem-table",
        "rocksdb.cur-size-all-mem-tables",
        "rocksdb.size-all-mem-tables",
        "rocksdb.estimate-table-readers-mem",
        "rocksdb.estimate-live-data-size",
        "rocksdb.total-sst-files-size",
        "rocksdb.live-sst-files-size",
        "rocksdb.base-level",
        "rocksdb.estimate-pending-compaction-bytes",
        "rocksdb.num-running-compactions",
        "rocksdb.num-running-flushes",
        "rocksdb.actual-delayed-write-rate",
        "rocksdb.is-write-stopped",
        "rocksdb.block-cache-capacity",
        "rocksdb.block-cache-usage",
    ]
    .iter()
    .map(|x| (*x, format!("aptos_{}", x.replace('.', "_"))))
    .collect()
});

/// RocksDB tickers to report. These are per-DB cumulative counters (not per-CF).
static ROCKSDB_TICKERS_TO_REPORT: &[(Ticker, &str)] = &[
    (Ticker::StallMicros, "stall_micros"),
    (Ticker::CompactReadBytes, "compact_read_bytes"),
    (Ticker::CompactWriteBytes, "compact_write_bytes"),
    (Ticker::FlushWriteBytes, "flush_write_bytes"),
    (Ticker::BloomFilterUseful, "bloom_filter_useful"),
    (
        Ticker::BloomFilterFullPositive,
        "bloom_filter_full_positive",
    ),
    (
        Ticker::BloomFilterFullTruePositive,
        "bloom_filter_full_true_positive",
    ),
    (
        Ticker::BloomFilterPrefixChecked,
        "bloom_filter_prefix_checked",
    ),
    (
        Ticker::BloomFilterPrefixUseful,
        "bloom_filter_prefix_useful",
    ),
    (
        Ticker::BloomFilterPrefixTruePositive,
        "bloom_filter_prefix_true_positive",
    ),
    (Ticker::BlockCacheHit, "block_cache_hit"),
    (Ticker::BlockCacheMiss, "block_cache_miss"),
];

fn set_tickers(db: &DB) {
    let db_name = db.name();
    for (ticker, ticker_name) in ROCKSDB_TICKERS_TO_REPORT {
        ROCKSDB_TICKERS
            .with_label_values(&[db_name, ticker_name])
            .set(db.get_ticker_count(*ticker) as i64);
    }
}

fn set_property(cf_name: &str, db: &DB) -> Result<()> {
    if !skip_reporting_cf(cf_name) {
        for (rockdb_property_name, aptos_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
            ROCKSDB_PROPERTIES
                .with_label_values(&[cf_name, aptos_rocksdb_property_name])
                .set(db.get_property(cf_name, rockdb_property_name)? as i64);
        }
    }
    Ok(())
}

fn set_shard_property(cf_name: ColumnFamilyName, db: &DB, shard: usize) -> Result<()> {
    if !skip_reporting_cf(cf_name) {
        for (rockdb_property_name, aptos_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
            ROCKSDB_SHARD_PROPERTIES
                .with_label_values(&[
                    SHARD_NAME_BY_ID[shard],
                    cf_name,
                    aptos_rocksdb_property_name,
                ])
                .set(db.get_property(cf_name, rockdb_property_name)? as i64);
        }
    }
    Ok(())
}

fn update_rocksdb_properties(
    ledger_db: &LedgerDb,
    state_merkle_db: &StateMerkleDb,
    state_kv_db: &StateKvDb,
) -> Result<()> {
    let _timer = OTHER_TIMERS_SECONDS.timer_with(&["update_rocksdb_properties"]);

    for cf in ledger_metadata_db_column_families() {
        set_property(cf, &ledger_db.metadata_db_arc())?;
    }

    for cf in write_set_db_column_families() {
        set_property(cf, ledger_db.write_set_db_raw())?;
    }

    for cf in transaction_info_db_column_families() {
        set_property(cf, ledger_db.transaction_info_db_raw())?;
    }

    for cf in transaction_db_column_families() {
        set_property(cf, ledger_db.transaction_db_raw())?;
    }

    for cf in event_db_column_families() {
        set_property(cf, ledger_db.event_db_raw())?;
    }

    for cf in transaction_accumulator_db_column_families() {
        set_property(cf, ledger_db.transaction_accumulator_db_raw())?;
    }

    for cf in state_kv_db_new_key_column_families() {
        set_property(cf, state_kv_db.metadata_db())?;
        for shard in 0..NUM_STATE_SHARDS {
            set_shard_property(cf, state_kv_db.db_shard(shard), shard)?;
        }
    }

    for cf_name in state_merkle_db_column_families() {
        set_property(cf_name, state_merkle_db.metadata_db())?;
        for shard in 0..NUM_STATE_SHARDS {
            set_shard_property(cf_name, state_merkle_db.db_shard(shard), shard)?;
        }
    }

    // Report per-DB tickers (not per-CF — tickers are DB-level statistics).
    set_tickers(&ledger_db.metadata_db_arc());
    set_tickers(ledger_db.write_set_db_raw());
    set_tickers(ledger_db.transaction_info_db_raw());
    set_tickers(ledger_db.transaction_db_raw());
    set_tickers(ledger_db.event_db_raw());
    set_tickers(ledger_db.transaction_accumulator_db_raw());

    set_tickers(state_kv_db.metadata_db());
    for shard in 0..NUM_STATE_SHARDS {
        set_tickers(state_kv_db.db_shard(shard));
    }

    set_tickers(state_merkle_db.metadata_db());
    for shard in 0..NUM_STATE_SHARDS {
        set_tickers(state_merkle_db.db_shard(shard));
    }

    Ok(())
}

#[derive(Debug)]
pub(crate) struct RocksdbPropertyReporter {
    sender: Mutex<mpsc::Sender<()>>,
    join_handle: Option<JoinHandle<()>>,
}

impl RocksdbPropertyReporter {
    pub(crate) fn new(
        ledger_db: Arc<LedgerDb>,
        state_merkle_db: Arc<StateMerkleDb>,
        state_kv_db: Arc<StateKvDb>,
    ) -> Self {
        let (send, recv) = mpsc::channel();
        let join_handle = Some(
            thread::Builder::new()
                .name("rocksdb-prop".into())
                .spawn(move || loop {
                    if let Err(e) =
                        update_rocksdb_properties(&ledger_db, &state_merkle_db, &state_kv_db)
                    {
                        warn!(
                            error = ?e,
                            "Updating rocksdb property failed."
                        );
                    }
                    // report rocksdb properties each 60 seconds
                    const TIMEOUT_MS: u64 = if cfg!(test) { 10 } else { 60000 };

                    match recv.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
                        Ok(_) => break,
                        Err(mpsc::RecvTimeoutError::Timeout) => (),
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    }
                })
                .expect("failed to spawn rocksdb-prop thread"),
        );
        Self {
            sender: Mutex::new(send),
            join_handle,
        }
    }
}

impl Drop for RocksdbPropertyReporter {
    fn drop(&mut self) {
        // Notify the property reporting thread to exit
        self.sender.lock().send(()).unwrap();
        self.join_handle
            .take()
            .expect("Rocksdb property reporting thread must exist.")
            .join()
            .expect("Rocksdb property reporting thread should join peacefully.");
    }
}
