// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use once_cell::sync::{Lazy, OnceCell};
use rayon::{ThreadPool, ThreadPoolBuilder};
use std::sync::Arc;

pub static NATIVE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
pub static NATIVE_EXECUTOR_POOL: Lazy<Arc<ThreadPool>> = Lazy::new(|| {
    Arc::new(
        ThreadPoolBuilder::new()
            .num_threads(NativeConfig::get_concurrency_level())
            .thread_name(|index| format!("native_exe_{}", index))
            .build()
            .unwrap(),
    )
});

pub struct NativeConfig;

impl NativeConfig {
    pub fn set_concurrency_level_once(concurrency_level: usize) {
        NATIVE_EXECUTOR_CONCURRENCY_LEVEL
            .set(concurrency_level)
            .ok();
    }

    pub fn get_concurrency_level() -> usize {
        match NATIVE_EXECUTOR_CONCURRENCY_LEVEL.get() {
            Some(concurrency_level) => *concurrency_level,
            None => 1,
        }
    }
}
