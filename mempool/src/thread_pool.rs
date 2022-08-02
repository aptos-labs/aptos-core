// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use once_cell::sync::Lazy;

pub(crate) static IO_POOL: Lazy<rayon::ThreadPool> = Lazy::new(|| {
    rayon::ThreadPoolBuilder::new()
        .thread_name(|index| format!("mempool_io_{}", index))
        .build()
        .unwrap()
});
