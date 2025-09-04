// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use velor_metrics_core::{
    register_int_counter, register_int_counter_vec, register_int_gauge, IntCounter, IntCounterVec,
    IntGauge,
};
use once_cell::sync::Lazy;

pub static VELOR_JELLYFISH_LEAF_ENCODED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_jellyfish_leaf_encoded_bytes",
        "Velor jellyfish leaf encoded bytes in total"
    )
    .unwrap()
});

pub static VELOR_JELLYFISH_INTERNAL_ENCODED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_jellyfish_internal_encoded_bytes",
        "Velor jellyfish total internal nodes encoded in bytes"
    )
    .unwrap()
});

pub static VELOR_JELLYFISH_LEAF_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "velor_jellyfish_leaf_count",
        "Total number of leaves in the latest JMT."
    )
    .unwrap()
});

pub static VELOR_JELLYFISH_LEAF_DELETION_COUNT: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "velor_jellyfish_leaf_deletion_count",
        "The number of deletions happened in JMT."
    )
    .unwrap()
});

pub static COUNTER: Lazy<IntCounterVec> = Lazy::new(|| {
    register_int_counter_vec!(
        // metric name
        "velor_jellyfish_counter",
        // metric description
        "Various counters for the JellyfishMerkleTree",
        // metric labels (dimensions)
        &["name"],
    )
    .unwrap()
});
