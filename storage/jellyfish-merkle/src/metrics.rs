// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{
    make_local_int_counter, make_local_int_counter_vec, register_int_gauge, IntGauge,
};
use once_cell::sync::Lazy;

make_local_int_counter!(
    pub,
    APTOS_JELLYFISH_LEAF_ENCODED_BYTES,
    "aptos_jellyfish_leaf_encoded_bytes",
    "Aptos jellyfish leaf encoded bytes in total",
);

make_local_int_counter!(
    pub,
    APTOS_JELLYFISH_INTERNAL_ENCODED_BYTES,
    "aptos_jellyfish_internal_encoded_bytes",
    "Aptos jellyfish total internal nodes encoded in bytes"
);

pub static APTOS_JELLYFISH_LEAF_COUNT: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        "aptos_jellyfish_leaf_count",
        "Total number of leaves in the latest JMT."
    )
    .unwrap()
});

make_local_int_counter!(
    pub,
    APTOS_JELLYFISH_LEAF_DELETION_COUNT,
    "aptos_jellyfish_leaf_deletion_count",
    "The number of deletions happened in JMT."
);

make_local_int_counter_vec!(
    pub,
    COUNTER,
    // metric name
    "aptos_jellyfish_counter",
    // metric description
    "Various counters for the JellyfishMerkleTree",
    // metric labels (dimensions)
    &["name"],
);
