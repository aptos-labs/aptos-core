// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Limits on the tree-size of interned types, used to prevent DoS via
/// deeply nested or extremely large type expressions.
#[derive(Clone, Debug)]
pub struct TypeTreeSizeLimits {
    /// Maximum allowed depth (root-to-leaf) for any interned type (default: 64).
    pub max_depth: u32,
    /// Maximum allowed node count for any interned type (default: 512).
    pub max_count: u32,
}

impl Default for TypeTreeSizeLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_count: 512,
        }
    }
}

/// Configuration for maintenance phase of [`crate::GlobalContext`].
#[derive(Clone)]
pub struct MaintenanceConfig {
    // TODO: add actual configs here.
    pub dummy: u64,
    /// Limits on type tree size enforced during interning.
    pub type_tree_size_limits: TypeTreeSizeLimits,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self {
            dummy: 123,
            type_tree_size_limits: TypeTreeSizeLimits::default(),
        }
    }
}
