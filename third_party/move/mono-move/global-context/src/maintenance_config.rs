// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

/// Configuration for maintenance phase of [`crate::GlobalContext`].
#[derive(Clone)]
pub struct MaintenanceConfig {
    // TODO: add actual configs here.
    pub dummy: u64,
}

impl Default for MaintenanceConfig {
    fn default() -> Self {
        Self { dummy: 123 }
    }
}
