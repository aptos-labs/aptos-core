// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

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
