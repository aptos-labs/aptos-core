// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use aptos_metrics_core::{
    exponential_buckets, make_thread_local_histogram_vec, register_int_gauge_vec, IntGaugeVec,
};
use once_cell::sync::Lazy;

make_thread_local_histogram_vec!(
    pub,
    TIMER,
    "aptos_scratchpad_smt_timer_seconds",
    "Various timers for performance analysis.",
    &["name"],
    exponential_buckets(/*start=*/ 1e-6, /*factor=*/ 2.0, /*count=*/ 22).unwrap(),
);

pub static GENERATION: Lazy<IntGaugeVec> = Lazy::new(|| {
    register_int_gauge_vec!(
        "aptos_scratchpad_smt_generation",
        "Various generations to help debugging.",
        &["name"],
    )
    .unwrap()
});
