// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics_core::{register_int_counter, IntCounter};
use once_cell::sync::Lazy;

pub static APTOS_BSMT_LEAF_ENCODED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_bsmt_leaf_encoded_bytes",
        "Aptos BSMT leaf encoded bytes in total"
    )
    .unwrap()
});

pub static APTOS_BSMT_INTERNAL_ENCODED_BYTES: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_bsmt_internal_encoded_bytes",
        "Aptos BSMT total internal nodes encoded in bytes"
    )
    .unwrap()
});

pub static APTOS_BSMT_STORAGE_READS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!("aptos_bsmt_storage_reads", "Aptos BSMT reads from storage").unwrap()
});
