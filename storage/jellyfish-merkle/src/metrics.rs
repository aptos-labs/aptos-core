// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, register_int_gauge, IntCounter, IntGauge};
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
