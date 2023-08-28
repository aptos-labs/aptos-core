// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_runtimes::{
    spawn_named_runtime_with_start_hook, spawn_rayon_thread_pool_with_start_hook,
};
#[cfg(target_os = "linux")]
use libc::{cpu_set_t, sched_setaffinity, CPU_SET};
use once_cell::sync::{Lazy, OnceCell};
use rayon::ThreadPool;
use tokio::runtime::{Handle, Runtime};

pub static THREAD_MANAGER: Lazy<ThreadManager> =
    Lazy::new(|| ThreadManager::new(ThreadManager::get_thread_config_strategy()));

static THREAD_CONFIG_STRATEGY: OnceCell<ThreadConfigStrategy> = OnceCell::new();

struct ThreadsConfig {
    num_threads: usize,
    on_thread_start: Box<dyn Fn() + Send + Sync + 'static>,
}

impl Default for ThreadsConfig {
    fn default() -> Self {
        Self::new_without_start_hook(num_cpus::get())
    }
}

impl ThreadsConfig {
    fn new_without_start_hook(num_threads: usize) -> Self {
        Self::new(num_threads, || {})
    }

    fn new<F>(num_threads: usize, on_thread_start: F) -> Self
    where
        F: Fn() + Send + Sync + 'static,
    {
        Self {
            num_threads,
            on_thread_start: Box::new(on_thread_start),
        }
    }
}

#[derive(Default)]
struct ThreadsConfigs {
    tokio_config: ThreadsConfig,
    rayon_config: ThreadsConfig,
}

struct Threads {
    runtime: Runtime,
    cpu_pool: ThreadPool,
}

pub struct ThreadManager {
    exe_threads: Threads,
    non_exe_threads: Threads,
    io_threads: ThreadPool,
}

impl Default for ThreadManager {
    fn default() -> Self {
        Self::new(ThreadConfigStrategy::DefaultStrategy)
    }
}

#[derive(Clone, Debug)]
pub enum ThreadConfigStrategy {
    DefaultStrategy,
    #[cfg(target_os = "linux")]
    PinExeThreadsToCores(usize),
}

// We probably want a "strategy trait" to define the methods below, and have different strategies
// to implement them separately, but I'm lazy.
impl ThreadManager {
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

    pub fn get_exe_runtime(&self) -> Handle {
        self.exe_threads.runtime.handle().clone()
    }

    pub fn get_exe_cpu_pool(&self) -> &ThreadPool {
        &self.exe_threads.cpu_pool
    }

    pub fn get_non_exe_runtime(&self) -> Handle {
        self.non_exe_threads.runtime.handle().clone()
    }

    pub fn get_non_exe_cpu_pool(&self) -> &ThreadPool {
        &self.non_exe_threads.cpu_pool
    }

    pub fn get_io_pool(&self) -> &ThreadPool {
        &self.io_threads
    }

    fn new(strategy: ThreadConfigStrategy) -> Self {
        match strategy {
            ThreadConfigStrategy::DefaultStrategy => {
                let exe_threads = Self::create_threads(ThreadsConfigs::default(), "exe");
                let non_exe_threads = Self::create_threads(ThreadsConfigs::default(), "non_exe");
                let io_threads =
                    Self::create_io_threads(ThreadsConfig::new_without_start_hook(64), "io");
                Self {
                    exe_threads,
                    non_exe_threads,
                    io_threads,
                }
            },

            #[cfg(target_os = "linux")]
            ThreadConfigStrategy::PinExeThreadsToCores(num_exe_cpu) => {
                let core_ids = core_affinity::get_core_ids().unwrap();
                assert!(core_ids.len() > num_exe_cpu);

                let mut exe_cpu_set = Self::new_cpu_set();
                let mut non_exe_cpu_set = Self::new_cpu_set();
                for core_id in core_ids.iter().take(num_exe_cpu) {
                    unsafe { CPU_SET(core_id.id, &mut exe_cpu_set) };
                }
                for core_id in core_ids.iter().skip(num_exe_cpu) {
                    unsafe { CPU_SET(core_id.id, &mut non_exe_cpu_set) };
                }

                let exe_threads = Self::create_threads(
                    ThreadsConfigs {
                        tokio_config: ThreadsConfig::new(
                            num_exe_cpu,
                            Self::pin_cpu_set(exe_cpu_set),
                        ),
                        rayon_config: ThreadsConfig::new(
                            num_exe_cpu,
                            Self::pin_cpu_set(exe_cpu_set),
                        ),
                    },
                    "exe",
                );

                let non_exe_threads = Self::create_threads(
                    ThreadsConfigs {
                        tokio_config: ThreadsConfig::new(
                            core_ids.len() - num_exe_cpu,
                            Self::pin_cpu_set(non_exe_cpu_set),
                        ),
                        rayon_config: ThreadsConfig::new(
                            core_ids.len() - num_exe_cpu,
                            Self::pin_cpu_set(non_exe_cpu_set),
                        ),
                    },
                    "non_exe",
                );

                let io_threads =
                    Self::create_io_threads(ThreadsConfig::new_without_start_hook(64), "io");

                Self {
                    exe_threads,
                    non_exe_threads,
                    io_threads,
                }
            },
        }
    }

    fn create_threads(config: ThreadsConfigs, name: &str) -> Threads {
        let runtime = spawn_named_runtime_with_start_hook(
            name.into(),
            Some(config.tokio_config.num_threads),
            config.tokio_config.on_thread_start,
        );

        let cpu_pool = spawn_rayon_thread_pool_with_start_hook(
            name.into(),
            Some(config.rayon_config.num_threads),
            config.rayon_config.on_thread_start,
        );

        Threads { runtime, cpu_pool }
    }

    fn create_io_threads(config: ThreadsConfig, name: &str) -> ThreadPool {
        spawn_rayon_thread_pool_with_start_hook(
            name.into(),
            Some(config.num_threads),
            config.on_thread_start,
        )
    }

    #[cfg(target_os = "linux")]
    fn new_cpu_set() -> cpu_set_t {
        unsafe { std::mem::zeroed::<cpu_set_t>() }
    }

    #[cfg(target_os = "linux")]
    fn pin_cpu_set(cpu_set: cpu_set_t) -> impl Fn() + Send + Sync + 'static {
        move || {
            unsafe {
                sched_setaffinity(
                    0, // Defaults to current thread
                    std::mem::size_of::<cpu_set_t>(),
                    &cpu_set,
                );
            };
        }
    }
}
