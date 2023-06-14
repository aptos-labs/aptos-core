// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_gauge, IntGauge};
use once_cell::sync::Lazy;

pub static NUM_EXECUTOR_SHARDS: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "num_executor_shards",
        "Number of shards for the sharded block executor"
    )
    .unwrap()
});
