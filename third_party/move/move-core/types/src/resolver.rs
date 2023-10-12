// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    value::MoveTypeLayout,
};
use anyhow::Error;
use bytes::Bytes;

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
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata>;

    fn get_module(&self, id: &ModuleId) -> Result<Option<Bytes>, Error>;
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
    // TODO: this can return Value, so that we can push deserialization to
    // implementations of `ResourceResolver`.
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        metadata: &[Metadata],
        // Default implementation does not use layout. This way there is no
        // need to implement this method.
        #[allow(unused_variables)] layout: &MoveTypeLayout,
    ) -> anyhow::Result<(Option<Bytes>, usize), Error> {
        self.get_resource_bytes_with_metadata(address, typ, metadata)
    }

    fn get_resource_bytes_with_metadata(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<(Option<Bytes>, usize), Error>;
}

/// A persistent storage implementation that can resolve both resources and modules
pub trait MoveResolver: ModuleResolver + ResourceResolver {
    fn get_resource(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Bytes>, Error> {
        Ok(self
            .get_resource_with_metadata(address, typ, &self.get_module_metadata(&typ.module_id()))?
            .0)
    }

    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Bytes>, usize), Error> {
        self.get_resource_bytes_with_metadata(address, typ, metadata)
    }
}

impl<T: ModuleResolver + ResourceResolver + ?Sized> MoveResolver for T {}
