// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::interner::InternedIdentifier;
use move_core_types::account_address::AccountAddress;

/// Identifies an executable (module or script) by its address and name.
///   - For modules, constructed from module address and name.
///   - For scripts: TODO
///
/// # Safety
///
/// Must be created from a valid global arena pointer to module's name.
pub struct ExecutableId {
    address: AccountAddress,
    name: InternedIdentifier,
}

impl ExecutableId {
    /// Creates a new module ID.
    ///
    /// # Safety
    ///
    /// The caller must ensure name points to a valid, live arena allocation.
    pub const unsafe fn new(address: AccountAddress, name: InternedIdentifier) -> Self {
        Self { address, name }
    }

    /// Returns the account address of this module.
    pub fn address(&self) -> &AccountAddress {
        &self.address
    }

    /// Returns the arena pointer to the name.
    pub fn name(&self) -> InternedIdentifier {
        self.name
    }
}
