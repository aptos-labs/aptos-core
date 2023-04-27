// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
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
    fn get_module(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Error>;
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
    fn get_resource(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Vec<u8>>, Error>;
}

/// A persistent storage implementation that can resolve both resources and modules
pub trait MoveResolver: ModuleResolver + ResourceResolver {}

impl<T: ModuleResolver + ResourceResolver + ?Sized> MoveResolver for T {}

impl<T: ResourceResolver + ?Sized> ResourceResolver for &T {
    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Error> {
        (**self).get_resource(address, tag)
    }
}

impl<T: ModuleResolver + ?Sized> ModuleResolver for &T {
    fn get_module(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Error> {
        (**self).get_module(module_id)
    }
}
