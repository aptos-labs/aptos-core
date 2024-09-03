// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use move_binary_format::errors::VMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Storage that contains serialized modules. Clients can implement this trait
/// for their own backends, so that [ModuleStorage] can be built on top of it.
pub trait ModuleBytesStorage {
    /// Returns the bytes of the module with the specified name at the specified
    /// address. If the module does not exist, [None] is returned. Returns an error
    /// if the storage fails.
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;
}
