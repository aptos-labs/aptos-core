// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    IntCounter, IntCounterVec, IntGauge, register_int_counter, register_int_counter_vec,
    register_int_gauge,
};
use once_cell::sync::Lazy;

pub static APTOS_JELLYFISH_LEAF_ENCODED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_jellyfish_leaf_encoded_bytes",
        "Aptos jellyfish leaf encoded bytes in total"
    )
    .unwrap()
});

pub static APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_jellyfish_internal_encoded_bytes",
        "Aptos jellyfish total internal nodes encoded in bytes"
    )
    .unwrap()
});

pub static APTOS_JELLYFISH_LEAF_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_jellyfish_leaf_count",
        "Total number of leaves in the latest JMT."
    )
    .unwrap()
});

pub static APTOS_JELLYFISH_LEAF_DELETION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_jellyfish_leaf_deletion_count",
        "The number of deletions happened in JMT."
    )
    .unwrap()
});

pub static COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        // metric name
        "aptos_jellyfish_counter",
        // metric description
        "Various counters for the JellyfishMerkleTree",
        // metric labels (dimensions)
        &["name"],
    )
    .unwrap()
});
