// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::endpoints::RejectionReason;
use velor_metrics_core::{
    register_histogram_vec, register_int_counter_vec, register_int_gauge, HistogramVec,
    IntCounterVec, IntGauge,
};
use once_cell::sync::Lazy;

pub static HISTOGRAM: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_tap_requests",
        "Tap requests latency grouped by method, operation_id and status.",
        &["method", "operation_id", "status"]
    )
    .unwrap()
});

pub static RESPONSE_STATUS: Lazy<HistogramVec> = Lazy::new(|| {
    register_histogram_vec!(
        "velor_tap_response_status",
        "Tap requests latency grouped by status code only.",
        &["status"]
    )
    .unwrap()
});

static REJECTION_REASONS: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        "velor_tap_rejection_reason_count",
        "Number of times the tap has returned the given rejection reason.",
        &["rejection_reason_code"]
    )
    .unwrap()
});

pub static NUM_OUTSTANDING_TRANSACTIONS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_tap_num_outstanding_transactions",
        "Number of transactions we've submitted but have not been processed by the blockchain.",
    )
    .unwrap()
});

// TODO: Consider using IntGaugeVec to attach the account address as a label.
pub static TRANSFER_FUNDER_ACCOUNT_BALANCE: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_tap_transfer_funder_account_balance",
        "Balance of the account used by the tap instance. Only populated for the TransferFunder.",
    )
    .unwrap()
});

pub fn bump_rejection_reason_counters(rejection_reasons: &[RejectionReason]) {
    for rejection_reason in rejection_reasons {
        REJECTION_REASONS
            .with_label_values(&[&format!("{}", rejection_reason.get_code() as u32)])
            .inc();
    }
}
