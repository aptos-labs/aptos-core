// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::GlobalValue;
use bytes::Bytes;
use move_binary_format::errors::PartialVMResult;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, metadata::Metadata,
    value::MoveTypeLayout,
};

pub fn resource_size(resource: &Option<Bytes>) -> usize {
    resource.as_ref().map(|bytes| bytes.len()).unwrap_or(0)
}

pub enum ValueOrBytes {
    Value(GlobalValue),
    Bytes(Option<Bytes>),
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

    fn get_gv_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        layout: &MoveTypeLayout,
        contains_dfs: bool,
    ) -> PartialVMResult<(ValueOrBytes, usize)> {
        self.get_resource_bytes_with_metadata_and_layout(
            address,
            struct_tag,
            metadata,
            contains_dfs.then_some(layout),
        )
        .map(|(bytes, size)| (ValueOrBytes::Bytes(bytes), size))
    }
}
