// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{
    db_options::{
        event_db_column_families, ledger_db_column_families, ledger_metadata_db_column_families,
        skip_reporting_cf, state_kv_db_column_families, state_kv_db_new_key_column_families,
        state_merkle_db_column_families, transaction_accumulator_db_column_families,
        transaction_db_column_families, transaction_info_db_column_families,
        write_set_db_column_families,
    },
    ledger_db::LedgerDb,
    metrics::{OTHER_TIMERS_SECONDS, ROCKSDB_PROPERTIES, ROCKSDB_SHARD_PROPERTIES},
    state_kv_db::StateKvDb,
    state_merkle_db::StateMerkleDb,
};
use anyhow::Result;
use velor_infallible::Mutex;
use velor_logger::prelude::*;
use velor_metrics_core::TimerHelper;
use velor_schemadb::{ColumnFamilyName, DB};
use velor_types::state_store::NUM_STATE_SHARDS;
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
        "rocksdb.num-entries-active-mem-table",
        "rocksdb.num-entries-imm-mem-tables",
        "rocksdb.num-deletes-active-mem-table",
        "rocksdb.num-deletes-imm-mem-tables",
        "rocksdb.estimate-num-keys",
        "rocksdb.estimate-table-readers-mem",
        "rocksdb.is-file-deletions-enabled",
        "rocksdb.num-snapshots",
        "rocksdb.oldest-snapshot-time",
        "rocksdb.num-live-versions",
        "rocksdb.current-super-version-number",
        "rocksdb.estimate-live-data-size",
        "rocksdb.min-log-number-to-keep",
        "rocksdb.min-obsolete-sst-number-to-keep",
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
        "rocksdb.block-cache-pinned-usage",
    ]
    .iter()
    .map(|x| (*x, format!("velor_{}", x.replace('.', "_"))))
    .collect()
});

fn set_property(cf_name: &str, db: &DB) -> Result<()> {
    if !skip_reporting_cf(cf_name) {
        for (rockdb_property_name, velor_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
            ROCKSDB_PROPERTIES
                .with_label_values(&[cf_name, velor_rocksdb_property_name])
                .set(db.get_property(cf_name, rockdb_property_name)? as i64);
        }
    }
    Ok(())
}

const SHARD_NAME_BY_ID: [&str; NUM_STATE_SHARDS] = [
    "0", "1", "2", "3", "4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15",
];

fn set_shard_property(cf_name: ColumnFamilyName, db: &DB, shard: usize) -> Result<()> {
    if !skip_reporting_cf(cf_name) {
        for (rockdb_property_name, velor_rocksdb_property_name) in &*ROCKSDB_PROPERTY_MAP {
            ROCKSDB_SHARD_PROPERTIES
                .with_label_values(&[
                    SHARD_NAME_BY_ID[shard],
                    cf_name,
                    velor_rocksdb_property_name,
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

    let enable_storage_sharding = state_kv_db.enabled_sharding();

    if enable_storage_sharding {
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

        if !state_kv_db.enabled_sharding() {
            for cf in state_kv_db_column_families() {
                set_property(cf, state_kv_db.metadata_db())?;
            }
        } else {
            for cf in state_kv_db_new_key_column_families() {
                set_property(cf, state_kv_db.metadata_db())?;
                for shard in 0..NUM_STATE_SHARDS {
                    set_shard_property(cf, state_kv_db.db_shard(shard), shard)?;
                }
            }
        }
    } else {
        for cf in ledger_db_column_families() {
            set_property(cf, &ledger_db.metadata_db_arc())?;
        }
    }

    for cf_name in state_merkle_db_column_families() {
        set_property(cf_name, state_merkle_db.metadata_db())?;
        if state_merkle_db.sharding_enabled() {
            for shard in 0..NUM_STATE_SHARDS {
                set_shard_property(cf_name, state_merkle_db.db_shard(shard), shard)?;
            }
        }
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
        let join_handle = Some(thread::spawn(move || loop {
            if let Err(e) = update_rocksdb_properties(&ledger_db, &state_merkle_db, &state_kv_db) {
                warn!(
                    error = ?e,
                    "Updating rocksdb property failed."
                );
            }
            // report rocksdb properties each 10 seconds
            const TIMEOUT_MS: u64 = if cfg!(test) { 10 } else { 10000 };

            match recv.recv_timeout(Duration::from_millis(TIMEOUT_MS)) {
                Ok(_) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => (),
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }));
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
