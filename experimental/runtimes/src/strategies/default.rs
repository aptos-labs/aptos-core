// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::thread_manager::{ThreadManager, MAX_THREAD_POOL_SIZE};
use aptos_runtimes::spawn_rayon_thread_pool;
use rayon::ThreadPool;
use std::{cmp::min, sync::Arc};

pub struct DefaultThreadManager {
    exe_threads: Arc<ThreadPool>,
    non_exe_threads: ThreadPool,
    io_threads: ThreadPool,
    background_threads: ThreadPool,
}

impl DefaultThreadManager {
    pub(crate) fn new() -> DefaultThreadManager {
        // Do not use more than 32 threads for rayon thread pools as we have seen scalability issues with more threads.
        // This needs to be revisited once we resolve the scalability issues.
        let exe_threads = Arc::new(spawn_rayon_thread_pool(
            "exe".into(),
            Some(min(num_cpus::get(), MAX_THREAD_POOL_SIZE)),
        ));
        let non_exe_threads = spawn_rayon_thread_pool(
            "non_exe".into(),
            Some(min(num_cpus::get(), MAX_THREAD_POOL_SIZE)),
        );
        let io_threads = spawn_rayon_thread_pool("io".into(), Some(64));
        let background_threads =
            spawn_rayon_thread_pool("background".into(), Some(MAX_THREAD_POOL_SIZE));
        Self {
            exe_threads,
            non_exe_threads,
            io_threads,
            background_threads,
        }
    }
}

impl<'a> ThreadManager<'a> for DefaultThreadManager {
    fn get_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.exe_threads
    }

    fn get_exe_cpu_pool_arc(&self) -> Arc<ThreadPool> {
        Arc::clone(&self.exe_threads)
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

    fn get_background_pool(&'a self) -> &'a ThreadPool {
        &self.background_threads
    }
}
