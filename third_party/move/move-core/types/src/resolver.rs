// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
};
use anyhow::Error;

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

    fn get_module(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Error>;
}

pub fn resource_size(resource: &Option<Vec<u8>>) -> usize {
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
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Vec<u8>>, usize), Error>;
}

/// A persistent storage implementation that can resolve both resources and modules
pub trait MoveResolver: ModuleResolver + ResourceResolver {
    fn get_resource(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Vec<u8>>, Error> {
        Ok(self
            .get_resource_with_metadata(address, typ, &self.get_module_metadata(&typ.module_id()))?
            .0)
    }
}

impl<T: ModuleResolver + ResourceResolver + ?Sized> MoveResolver for T {}

impl<T: ResourceResolver + ?Sized> ResourceResolver for &T {
    fn get_resource_with_metadata(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        metadata: &[Metadata],
    ) -> Result<(Option<Vec<u8>>, usize), Error> {
        (**self).get_resource_with_metadata(address, tag, metadata)
    }
}

impl<T: ModuleResolver + ?Sized> ModuleResolver for &T {
    fn get_module_metadata(&self, module_id: &ModuleId) -> Vec<Metadata> {
        (**self).get_module_metadata(module_id)
    }

    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Error> {
        (**self).get_module(module_id)
    }
}
