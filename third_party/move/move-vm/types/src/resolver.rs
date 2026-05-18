// Parts of the file are Copyright (c) The Diem Core Contributors
// Parts of the file are Copyright (c) The Move Contributors
// Parts of the file are Copyright (c) Aptos Foundation
// All Aptos Foundation code and content is licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, metadata::Metadata,
    value::MoveTypeLayout,
};

pub fn resource_size(resource: &Option<Bytes>) -> usize {
    resource.as_ref().map(|bytes| bytes.len()).unwrap_or(0)
}

/// A persistent storage backend that can resolve resources by address + type
/// Storage backends should return
///   - Ok(Some(..)) if the data exists
///   - Ok(None)     if the data does not exist
///   - Err(..)      only when something really wrong happens, for example
///                    - invariants are broken and observable from the storage side
///                      (this is not currently possible as ModuleId and StructTag
///                       are always structurally valid)
///                    - storage encounters internal error
pub trait ResourceResolver {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)>;
}
