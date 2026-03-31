// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use once_cell::sync::{Lazy, OnceCell};
use std::sync::Arc;

pub static NATIVE_EXECUTOR_CONCURRENCY_LEVEL: OnceCell<usize> = OnceCell::new();
pub static NATIVE_EXECUTOR_POOL: Lazy<Arc<rayon::ThreadPool>> = Lazy::new(|| {
    Arc::new(
        rayon::ThreadPoolBuilder::new()
            .num_threads(NativeConfig::get_concurrency_level())
            .thread_name(|index| format!("native_exe_{}", index))
            .build()
            .unwrap(),
    )
});
pub static NATIVE_EXECUTOR_RUNTIME: Lazy<tokio::runtime::Runtime> = Lazy::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .max_blocking_threads(NativeConfig::get_concurrency_level())
        .thread_name("native_exe")
        .build()
        .unwrap()
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
