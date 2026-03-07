// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

/// Configuration for maintenance phase of [`crate::GlobalContext`].
#[derive(Clone)]
pub struct MaintenanceConfig {
    /// Interner arena memory threshold in bytes. When the total arena usage
    /// exceeds this value, a full flush of all interners and executables is
    /// triggered at the next maintenance phase.
    pub max_global_arena_allocated_bytes: usize,
    /// Maximum number of monomorphized functions to cache across all live
    /// executables. When exceeded, TTL-based eviction is triggered during the
    /// maintenance phase.
    pub max_monomorphized_functions: usize,
    /// Functions not accessed within this many blocks before the current block
    /// boundary are eligible for TTL eviction when
    /// `max_monomorphized_functions` is exceeded. A value of `1` means: evict
    /// anything not used this block or the previous block.
    pub mono_eviction_ttl_blocks: u32,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            max_global_arena_allocated_bytes: 1024 * 1024 * 1024,
            max_monomorphized_functions: 1_000_000,
            mono_eviction_ttl_blocks: 1,
        }
    }
}
