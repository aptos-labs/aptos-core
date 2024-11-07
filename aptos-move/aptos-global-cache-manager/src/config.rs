// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Configuration used for global caches.
pub struct GlobalCacheConfig {
    /// If true, when global caches are empty, Aptos framework is prefetched into module cache.
    pub prefetch_framework_code: bool,
    /// The maximum number of entries stored in module cache. If module cache exceeds this value,
    /// all its entries should be flushed.
    pub module_cache_capacity: usize,
    /// The maximum size of struct name re-indexing map stored in runtime environment.
    pub struct_name_index_map_capacity: usize,
}

impl Default for GlobalCacheConfig {
    fn default() -> Self {
        // TODO(loader_v2):
        //   Right now these are just some numbers, we can set them based on the upper bounds of
        //   module or identifier sizes, or keep track in read-only module cache how many bytes we
        //   are using.
        Self {
            prefetch_framework_code: true,
            module_cache_capacity: 10_000,
            struct_name_index_map_capacity: 10_000,
        }
    }
}
