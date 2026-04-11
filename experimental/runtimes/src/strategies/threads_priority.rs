// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{common::set_thread_nice_value, thread_manager::ThreadManager};
use aptos_runtimes::spawn_rayon_thread_pool_with_start_hook;
use rayon::ThreadPool;
use tokio::runtime::{Handle, Runtime};

pub(crate) struct ThreadsPriorityThreadManager {
    exe_threads: ThreadPool,
    non_exe_threads: ThreadPool,
    io_threads: Runtime,
    background_threads: ThreadPool,
}

impl ThreadsPriorityThreadManager {
    pub(crate) fn new(num_exe_threads: usize) -> Self {
        // TODO(grao): Make priorities and thread numbers configurable.
        let exe_threads = spawn_rayon_thread_pool_with_start_hook(
            "exe".into(),
            Some(num_exe_threads),
            set_thread_nice_value(-20),
        );

        let non_exe_threads = spawn_rayon_thread_pool_with_start_hook(
            "non_exe".into(),
            Some(16),
            set_thread_nice_value(-10),
        );

        let io_threads = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(1)
            .max_blocking_threads(64)
            .thread_name("io")
            .build()
            .expect("Failed to create io tokio runtime");

        let background_threads = spawn_rayon_thread_pool_with_start_hook(
            "background".into(),
            Some(32),
            set_thread_nice_value(20),
        );

        Self {
            exe_threads,
            non_exe_threads,
            io_threads,
            background_threads,
        }
    }
}

impl<'a> ThreadManager<'a> for ThreadsPriorityThreadManager {
    fn get_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.exe_threads
    }

    fn get_non_exe_cpu_pool(&'a self) -> &'a ThreadPool {
        &self.non_exe_threads
    }

    fn get_io_pool(&'a self) -> Handle {
        self.io_threads.handle().clone()
    }

    fn get_high_pri_io_pool(&'a self) -> Handle {
        self.io_threads.handle().clone()
    }

    fn get_background_pool(&'a self) -> &'a ThreadPool {
        &self.background_threads
    }
}
