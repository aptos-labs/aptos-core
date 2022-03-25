// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_metrics::{register_int_counter, IntCounter};
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

pub static APTOS_JELLYFISH_STORAGE_READS: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        "aptos_jellyfish_storage_reads",
        "Aptos jellyfish reads from storage"
    )
    .unwrap()
});
