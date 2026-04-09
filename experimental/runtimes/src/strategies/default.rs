// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::thread_manager::{ThreadManager, MAX_THREAD_POOL_SIZE};
use aptos_runtimes::spawn_rayon_thread_pool;
use rayon::ThreadPool;
use std::cmp::min;
use tokio::runtime::{Handle, Runtime};

pub struct DefaultThreadManager {
    exe_threads: ThreadPool,
    non_exe_threads: ThreadPool,
    io_threads: ThreadPool,
    background_runtime: Runtime,
}

impl DefaultThreadManager {
    pub(crate) fn new() -> DefaultThreadManager {
        // Do not use more than 32 threads for rayon thread pools as we have seen scalability issues with more threads.
        // This needs to be revisited once we resolve the scalability issues.
        let exe_threads = spawn_rayon_thread_pool(
            "exe".into(),
            Some(min(num_cpus::get(), MAX_THREAD_POOL_SIZE)),
        );
        let non_exe_threads = spawn_rayon_thread_pool(
            "non_exe".into(),
            Some(min(num_cpus::get(), MAX_THREAD_POOL_SIZE)),
        );
        let io_threads = spawn_rayon_thread_pool("io".into(), Some(64));
        let background_runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(MAX_THREAD_POOL_SIZE)
            .thread_name("bg-pool")
            .enable_all()
            .build()
            .expect("Failed to create background tokio runtime");
        Self {
            exe_threads,
            non_exe_threads,
            io_threads,
            background_runtime,
        }
    }
}

impl<'a> ThreadManager<'a> for DefaultThreadManager {
    fn get_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.exe_threads
    }

    fn get_non_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.non_exe_threads
    }

    fn get_io_pool(&'a self) -> &'a ThreadPool {
        &self.io_threads
    }

    fn get_high_pri_io_pool(&'a self) -> &'a ThreadPool {
        &self.io_threads
    }

    fn get_background_pool(&'a self) -> Handle {
        self.background_runtime.handle().clone()
    }
}
