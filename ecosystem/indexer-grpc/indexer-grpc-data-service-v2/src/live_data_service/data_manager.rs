// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::metrics::{
    CACHE_END_VERSION, CACHE_SIZE_BYTES, CACHE_SIZE_LIMIT_BYTES, CACHE_START_VERSION, COUNTER,
    LATENCY_MS,
};
use aptos_protos::transaction::v1::Transaction;
use prost::Message;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tracing::{error, trace, warn};

// TODO(grao): Naive implementation for now. This can be replaced by a more performant
// implementation in the future.
pub(super) struct DataManager {
    pub(super) start_version: u64,
    pub(super) end_version: u64,
    data: Vec<Option<Box<Transaction>>>,
    num_slots: usize,

    size_limit_bytes: usize,
    eviction_target: usize,
    total_size: usize,
}

impl DataManager {
    pub(super) fn new(end_version: u64, num_slots: usize, size_limit_bytes: usize) -> Self {
        CACHE_SIZE_LIMIT_BYTES.set(size_limit_bytes as i64);
        Self {
            start_version: end_version.saturating_sub(num_slots as u64),
            end_version,
            data: vec![None; num_slots],
            num_slots,
            size_limit_bytes,
            eviction_target: size_limit_bytes,
            total_size: 0,
        }
    }

    pub(super) fn get_data(&self, version: u64) -> &Option<Box<Transaction>> {
        &self.data[version as usize % self.num_slots]
    }

    pub(super) fn update_data(&mut self, start_version: u64, transactions: Vec<Transaction>) {
        let end_version = start_version + transactions.len() as u64;

        trace!(
            "Updating data for {} transactions in range [{start_version}, {end_version}).",
            transactions.len(),
        );
        if start_version > self.end_version {
            error!(
                "The data is in the future, cache end_version: {}, data start_version: {start_version}.",
                self.end_version
            );
            COUNTER.with_label_values(&["data_too_new"]).inc();
            return;
        }

        if end_version <= self.start_version {
            warn!(
                "The data is too old, cache start_version: {}, data end_version: {end_version}.",
                self.start_version
            );
            COUNTER.with_label_values(&["data_too_old"]).inc();
            return;
        }

        let num_to_skip = self.start_version.saturating_sub(start_version);
        let start_version = start_version.max(self.start_version);

        let mut size_increased = 0;
        let mut size_decreased = 0;

        for (i, transaction) in transactions
            .into_iter()
            .enumerate()
            .skip(num_to_skip as usize)
        {
            let version = start_version + i as u64;
            let slot_index = version as usize % self.num_slots;
            if let Some(transaction) = self.data[slot_index].take() {
                size_decreased += transaction.encoded_len();
            }
            size_increased += transaction.encoded_len();
            self.data[version as usize % self.num_slots] = Some(Box::new(transaction));
        }

        if end_version > self.end_version {
            self.end_version = end_version;
            if self.start_version + (self.num_slots as u64) < end_version {
                self.start_version = end_version - self.num_slots as u64;
            }
            if let Some(txn_timestamp) = self.get_data(end_version - 1).as_ref().unwrap().timestamp
            {
                let timestamp_since_epoch =
                    Duration::new(txn_timestamp.seconds as u64, txn_timestamp.nanos as u32);
                let now_since_epoch = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
                let latency = now_since_epoch.saturating_sub(timestamp_since_epoch);
                LATENCY_MS.set(latency.as_millis() as i64);
            }
        }

        self.total_size += size_increased;
        self.total_size -= size_decreased;

        if self.total_size >= self.size_limit_bytes {
            while self.total_size >= self.eviction_target {
                if let Some(transaction) =
                    self.data[self.start_version as usize % self.num_slots].take()
                {
                    self.total_size -= transaction.encoded_len();
                    drop(transaction);
                }
                self.start_version += 1;
            }
        }

        self.update_cache_metrics();
    }

    fn update_cache_metrics(&self) {
        CACHE_START_VERSION.set(self.start_version as i64);
        CACHE_END_VERSION.set(self.end_version as i64);
        CACHE_SIZE_BYTES.set(self.total_size as i64);
    }
}
