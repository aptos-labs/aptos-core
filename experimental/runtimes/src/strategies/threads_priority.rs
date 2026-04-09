// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{common::set_thread_nice_value, thread_manager::ThreadManager};
use aptos_runtimes::spawn_rayon_thread_pool_with_start_hook;
use rayon::ThreadPool;
use tokio::runtime::{Handle, Runtime};

pub(crate) struct ThreadsPriorityThreadManager {
    exe_threads: ThreadPool,
    non_exe_threads: ThreadPool,
    high_pri_io_threads: ThreadPool,
    io_threads: ThreadPool,
    background_runtime: Runtime,
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

        let high_pri_io_threads = spawn_rayon_thread_pool_with_start_hook(
            "io_high".into(),
            Some(32),
            set_thread_nice_value(-20),
        );

        let io_threads = spawn_rayon_thread_pool_with_start_hook(
            "io_low".into(),
            Some(64),
            set_thread_nice_value(1),
        );

        let background_runtime = tokio::runtime::Builder::new_current_thread()
            .max_blocking_threads(32)
            .thread_name("bg-pool")
            .on_thread_start(set_thread_nice_value(20))
            .enable_all()
            .build()
            .expect("Failed to create background tokio runtime");

        Self {
            exe_threads,
            non_exe_threads,
            high_pri_io_threads,
            io_threads,
            background_runtime,
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

    fn get_io_pool(&'a self) -> &'a ThreadPool {
        &self.io_threads
    }

    fn get_high_pri_io_pool(&'a self) -> &'a ThreadPool {
        &self.high_pri_io_threads
    }

    fn get_background_pool(&'a self) -> Handle {
        self.background_runtime.handle().clone()
    }
}
