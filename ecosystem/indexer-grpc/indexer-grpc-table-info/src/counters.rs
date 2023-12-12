// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_gauge_vec, register_int_gauge_vec, GaugeVec, IntGaugeVec};
use once_cell::sync::Lazy;

pub const SERVICE_TYPE: &str = "indexer_table_info";

pub enum IndexerTableInfoStep {
    TableInfoProcessedBatch, // [Indexer Table Info] Processed batch of transactions from fullnode
    TableInfoProcessed,      // [Indexer Table Info] Processed transactions from fullnode
}

impl IndexerTableInfoStep {
    pub fn get_step(&self) -> &'static str {
        match self {
            IndexerTableInfoStep::TableInfoProcessedBatch => "1",
            IndexerTableInfoStep::TableInfoProcessed => "2",
        }
    }

    pub fn get_label(&self) -> &'static str {
        match self {
            IndexerTableInfoStep::TableInfoProcessedBatch => {
                "[Indexer Table Info] Processed batch successfully"
            },
            IndexerTableInfoStep::TableInfoProcessed => {
                "[Indexer Table Info] Processed successfully"
            },
        }
    }
}

/// Latest processed transaction version.
pub static LATEST_PROCESSED_VERSION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_table_info_latest_processed_version",
        "Latest processed transaction version",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Transactions' total size in bytes at each step
pub static TOTAL_SIZE_IN_BYTES: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_table_info_total_size_in_bytes",
        "Total size in bytes at this step",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Number of transactions at each step
pub static NUM_TRANSACTIONS_COUNT: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "indexer_table_info_num_transactions_count",
        "Total count of transactions at this step",
        &["service_type", "step", "message"],
    )
    .unwrap()
});

/// Generic duration metric
pub static DURATION_IN_SECS: Lazy<GaugeVec> =
    Lazy::new(|| {
        register_gauge_vec!(
            "indexer_table_info_duration_in_secs",
            "Duration in seconds",
            &["service_type", "step", "message"]
        )
        .unwrap()
    });

/// Transaction timestamp in unixtime
pub static TRANSACTION_UNIX_TIMESTAMP: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "indexer_table_info_transaction_unix_timestamp",
        "Transaction timestamp in unixtime",
        &["service_type", "step", "message"]
    )
    .unwrap()
});
