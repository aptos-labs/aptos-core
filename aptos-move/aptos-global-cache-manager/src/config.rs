// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Configuration used for global caches.
pub struct GlobalCacheConfig {
    /// If true, when global caches are empty, Aptos framework is prefetched into module cache.
    pub prefetch_framework_code: bool,
    /// The maximum size serialized modules can take in module cache.
    pub max_module_cache_size_in_bytes: usize,
    /// The maximum size (in terms of entries) of struct name re-indexing map stored in runtime
    /// environment.
    pub max_struct_name_index_map_size: usize,
}

impl Default for GlobalCacheConfig {
    fn default() -> Self {
        // TODO(loader_v2):
        //   Right now these are hardcoded here, we probably want to add them to gas schedule or
        //   some on-chain config.
        Self {
            prefetch_framework_code: true,
            // Use 50 Mb for now, should be large enough to cache many modules.
            max_module_cache_size_in_bytes: 50 * 1024 * 1024,
            max_struct_name_index_map_size: 100_000,
        }
    }
}
