// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    value::MoveTypeLayout,
};
use bytes::Bytes;
use std::fmt::Debug;

/// Traits for resolving Move modules and resources from persistent storage

/// A persistent storage backend that can resolve modules by address + name.
/// Storage backends should return
///   - Ok(Some(..)) if the data exists
///   - Ok(None)     if the data does not exist
///   - Err(..)      only when something really wrong happens, for example
///                    - invariants are broken and observable from the storage side
///                      (this is not currently possible as ModuleId and StructTag
///                       are always structurally valid)
///                    - storage encounters internal error
pub trait ModuleResolver {
    type Error: Debug;

    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata>;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Bytes>, Self::Error>;
}

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
    type Error: Debug;

    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
        layout: Option<&MoveTypeLayout>,
    ) -> Result<(Option<Bytes>, usize), Self::Error>;
}

/// A persistent storage implementation that can resolve both resources and modules
pub trait MoveResolver<E: Debug>: ModuleResolver<Error = E> + ResourceResolver<Error = E> {
    fn get_resource(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
    ) -> Result<Option<Bytes>, <Self as ResourceResolver>::Error> {
        Ok(self
            .get_resource_with_metadata(
                address,
                struct_tag,
                &self.get_module_metadata(&struct_tag.module_id()),
            )?
            .0)
    }

    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        struct_tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Bytes>, usize), <Self as ResourceResolver>::Error> {
        self.get_resource_bytes_with_metadata_and_layout(address, struct_tag, metadata, None)
    }
}

impl<E: Debug, T: ModuleResolver<Error = E> + ResourceResolver<Error = E> + ?Sized> MoveResolver<E>
    for T
{
}
