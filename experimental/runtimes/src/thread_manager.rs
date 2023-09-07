// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::strategies::default::DefaultThreadManager;
#[cfg(target_os = "linux")]
use crate::strategies::{
    pin_exe_threads_to_cores::PinExeThreadsToCoresThreadManager,
    threads_priority::ThreadsPriorityThreadManager,
};
use once_cell::sync::{Lazy, OnceCell};
use rayon::ThreadPool;

pub static THREAD_MANAGER: Lazy<Box<dyn ThreadManager>> = Lazy::new(|| {
    ThreadManagerBuilder::create_thread_manager(ThreadManagerBuilder::get_thread_config_strategy())
});

static THREAD_CONFIG_STRATEGY: OnceCell<ThreadConfigStrategy> = OnceCell::new();

#[derive(Clone, Debug)]
pub enum ThreadConfigStrategy {
    DefaultStrategy,
    #[cfg(target_os = "linux")]
    PinExeThreadsToCores(usize),
    #[cfg(target_os = "linux")]
    ThreadsPriority(usize),
}

pub trait ThreadManager<'a>: Send + Sync {
    fn get_exe_cpu_pool(&'a self) -> &'a ThreadPool;
    fn get_non_exe_cpu_pool(&'a self) -> &'a ThreadPool;
    fn get_high_pri_io_pool(&'a self) -> &'a ThreadPool;
    fn get_io_pool(&'a self) -> &'a ThreadPool;
}

pub struct ThreadManagerBuilder;

impl ThreadManagerBuilder {
    pub fn get_thread_config_strategy() -> ThreadConfigStrategy {
        match THREAD_CONFIG_STRATEGY.get() {
            Some(strategy) => strategy.clone(),
            None => ThreadConfigStrategy::DefaultStrategy,
        }
    }

    pub fn set_thread_config_strategy(strategy: ThreadConfigStrategy) {
        THREAD_CONFIG_STRATEGY
            .set(strategy)
            .expect("ThreadConfigStrategy can only be set once.");
    }

    fn create_thread_manager(strategy: ThreadConfigStrategy) -> Box<dyn ThreadManager<'static>> {
        match strategy {
            ThreadConfigStrategy::DefaultStrategy => Box::new(DefaultThreadManager::new()),

            #[cfg(target_os = "linux")]
            ThreadConfigStrategy::PinExeThreadsToCores(num_exe_cpu) => {
                Box::new(PinExeThreadsToCoresThreadManager::new(num_exe_cpu))
            },

            #[cfg(target_os = "linux")]
            ThreadConfigStrategy::ThreadsPriority(num_exe_threads) => {
                Box::new(ThreadsPriorityThreadManager::new(num_exe_threads))
            },
        }
    }
}
