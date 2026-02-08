// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Helper utilities for interner benchmarks.

use global_context::interner_impls::{
    dashmap_chunked::DashMapChunkedInterner,
    dashmap_mutex::DashMapMutexInterner,
    dashmap_perthread_array::{set_thread_index, DashMapPerThreadArrayInterner},
    dashmap_sharded::DashMapShardedInterner,
    rwlock_btree::RwLockBTreeInterner,
    rwlock_decoupled::RwLockDecoupledInterner,
    rwlock_hashmap::RwLockHashMapInterner,
};
use rayon::ThreadPool;
use std::sync::Arc;

/// Enum for selecting interner implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InternerType {
    RwLockBTree,
    RwLockHashMap,
    RwLockDecoupled,
    DashMapMutex,
    DashMapSharded,
    DashMapPerThreadArray,
    DashMapChunked,
}

impl InternerType {
    pub fn name(&self) -> &'static str {
        match self {
            InternerType::RwLockBTree => "rwlock_btree",
            InternerType::RwLockHashMap => "rwlock_hashmap",
            InternerType::RwLockDecoupled => "rwlock_decoupled",
            InternerType::DashMapMutex => "dashmap_mutex",
            InternerType::DashMapSharded => "dashmap_sharded",
            InternerType::DashMapPerThreadArray => "dashmap_perthread_array",
            InternerType::DashMapChunked => "dashmap_chunked",
        }
    }

    pub fn all() -> &'static [InternerType] {
        &[
            InternerType::RwLockBTree,
            InternerType::RwLockHashMap,
            InternerType::RwLockDecoupled,
            InternerType::DashMapMutex,
            InternerType::DashMapSharded,
            InternerType::DashMapPerThreadArray,
            InternerType::DashMapChunked,
        ]
    }
}

/// Trait for benchmark interners (type-erased).
pub trait BenchInterner: Send + Sync {
    fn intern_string(&self, value: &str) -> *const str;
}

// Implement BenchInterner for each interner type
impl BenchInterner for RwLockBTreeInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

impl BenchInterner for RwLockHashMapInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

impl BenchInterner for RwLockDecoupledInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

impl BenchInterner for DashMapMutexInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

impl BenchInterner for DashMapShardedInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

impl BenchInterner for DashMapPerThreadArrayInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

impl BenchInterner for DashMapChunkedInterner<String> {
    fn intern_string(&self, value: &str) -> *const str {
        let ptr = self.intern(&value.to_string());
        unsafe { (*ptr.as_ptr()).as_str() as *const str }
    }
}

/// Creates an interner based on the specified type.
pub fn create_interner(interner_type: InternerType, thread_count: usize) -> Arc<dyn BenchInterner> {
    match interner_type {
        InternerType::RwLockBTree => Arc::new(RwLockBTreeInterner::<String>::new()),
        InternerType::RwLockHashMap => Arc::new(RwLockHashMapInterner::<String>::new()),
        InternerType::RwLockDecoupled => Arc::new(RwLockDecoupledInterner::<String>::new()),
        InternerType::DashMapMutex => Arc::new(DashMapMutexInterner::<String>::new()),
        InternerType::DashMapSharded => Arc::new(DashMapShardedInterner::<String>::new()),
        InternerType::DashMapPerThreadArray => {
            Arc::new(DashMapPerThreadArrayInterner::<String>::new(thread_count))
        },
        InternerType::DashMapChunked => Arc::new(DashMapChunkedInterner::<String>::new()),
    }
}

/// Sets the thread index for the current thread (required for per-thread array interner).
pub fn set_thread_index_for_current(idx: usize) {
    set_thread_index(idx);
}

/// Creates a thread pool with optional core pinning.
pub fn create_thread_pool(num_threads: usize, pin_cores: bool) -> ThreadPool {
    let mut builder = rayon::ThreadPoolBuilder::new().num_threads(num_threads);

    if pin_cores {
        // Try to pin threads to cores if supported
        if let Some(core_ids) = core_affinity::get_core_ids() {
            builder = builder.start_handler(move |thread_idx| {
                let core_id = core_ids[thread_idx % core_ids.len()];
                core_affinity::set_for_current(core_id);

                // Set thread index for per-thread interners
                set_thread_index(thread_idx);
            });
        } else {
            // Fallback: just set thread index
            builder = builder.start_handler(move |thread_idx| {
                set_thread_index(thread_idx);
            });
        }
    } else {
        // Just set thread index without pinning
        builder = builder.start_handler(move |thread_idx| {
            set_thread_index(thread_idx);
        });
    }

    builder.build().expect("Failed to build thread pool")
}
