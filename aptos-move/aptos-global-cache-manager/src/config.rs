// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Configuration used for global caches.
pub struct GlobalCacheConfig {
    /// The maximum size of module cache. If module cache exceeds this capacity, it should be
    /// flushed.
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
            module_cache_capacity: 100_000,
            struct_name_index_map_capacity: 100_000,
        }
    }
}
