// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use move_binary_format::errors::VMResult;
use move_core_types::{account_address::AccountAddress, identifier::IdentStr};

/// Storage that contains serialized modules. Clients can implement this trait for their own
/// backends, so that [ModuleStorage] can be built on top of it.
pub trait ModuleBytesStorage {
    /// Returns bytes of modules, if they exist, and [None] otherwise. The error is returned in
    /// case there are some storage-related issues.
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>>;
}
