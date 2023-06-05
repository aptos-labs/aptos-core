// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::hash::{DefaultHasher, HashValue};
use aptos_types::write_set::TransactionWrite;

pub(crate) fn module_hash<V: TransactionWrite>(module: &V) -> HashValue {
    module
        .extract_raw_bytes()
        .map(|bytes| {
            let mut hasher = DefaultHasher::new(b"Module");
            hasher.update(&bytes);
            hasher.finish()
        })
        .expect("Module can't be deleted")
}
