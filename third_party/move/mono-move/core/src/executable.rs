// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use mono_move_alloc::GlobalArenaPtr;
use move_core_types::account_address::AccountAddress;

/// Identifies an executable (module or script) by its address and name.
///   - For modules, constructed from module address and name.
///   - For scripts: TODO
///
/// # Safety
///
/// Must be created from a valid global arena pointer to executable's name.
pub struct ExecutableId {
    address: AccountAddress,
    name: GlobalArenaPtr<str>,
}

impl ExecutableId {
    /// Creates a new executable ID.
    ///
    /// # Safety
    ///
    /// The caller must ensure name points to a valid, live arena allocation.
    pub unsafe fn new(address: AccountAddress, name: GlobalArenaPtr<str>) -> Self {
        Self { address, name }
    }

    /// Returns the account address of this executable.
    pub fn address(&self) -> &AccountAddress {
        &self.address
    }

    /// Returns the arena pointer to the name.
    pub fn name(&self) -> GlobalArenaPtr<str> {
        self.name
    }
}
