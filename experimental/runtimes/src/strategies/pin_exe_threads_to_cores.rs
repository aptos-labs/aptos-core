// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    common::{new_cpu_set, pin_cpu_set},
    thread_manager::ThreadManager,
};
use velor_runtimes::spawn_rayon_thread_pool_with_start_hook;
use libc::CPU_SET;
use rayon::ThreadPool;

pub(crate) struct PinExeThreadsToCoresThreadManager {
    exe_threads: ThreadPool,
    non_exe_threads: ThreadPool,
    high_pri_io_threads: ThreadPool,
    io_threads: ThreadPool,
    background_threads: ThreadPool,
}

impl PinExeThreadsToCoresThreadManager {
    pub(crate) fn new(num_exe_cpu: usize) -> Self {
        let core_ids = core_affinity::get_core_ids().unwrap();
        assert!(core_ids.len() > num_exe_cpu);

        let mut exe_cpu_set = new_cpu_set();
        let mut non_exe_cpu_set = new_cpu_set();
        for core_id in core_ids.iter().take(num_exe_cpu) {
            unsafe { CPU_SET(core_id.id, &mut exe_cpu_set) };
        }
        for core_id in core_ids.iter().skip(num_exe_cpu) {
            unsafe { CPU_SET(core_id.id, &mut non_exe_cpu_set) };
        }

        let exe_threads = spawn_rayon_thread_pool_with_start_hook(
            "exe".into(),
            Some(num_exe_cpu),
            pin_cpu_set(exe_cpu_set),
        );

        let non_exe_threads = spawn_rayon_thread_pool_with_start_hook(
            "non_exe".into(),
            Some(core_ids.len() - num_exe_cpu),
            pin_cpu_set(non_exe_cpu_set),
        );

        let high_pri_io_threads = spawn_rayon_thread_pool_with_start_hook(
            "io_high".into(),
            Some(32),
            pin_cpu_set(exe_cpu_set),
        );

        let io_threads = spawn_rayon_thread_pool_with_start_hook(
            "io_low".into(),
            Some(64),
            pin_cpu_set(non_exe_cpu_set),
        );

        let background_threads = spawn_rayon_thread_pool_with_start_hook(
            "background".into(),
            Some(32),
            pin_cpu_set(non_exe_cpu_set),
        );

        Self {
            exe_threads,
            non_exe_threads,
            high_pri_io_threads,
            io_threads,
            background_threads,
        }
    }
}

impl<'a> ThreadManager<'a> for PinExeThreadsToCoresThreadManager {
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

    fn get_background_pool(&'a self) -> &'a ThreadPool {
        &self.background_threads
    }
}
