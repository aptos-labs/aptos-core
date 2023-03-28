// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Legacy traits for resolving Move modules and resources from persistent
//! storage as blobs.

use crate::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
};
use std::fmt::Debug;

/// A persistent storage backend that can resolve modules by address + name.
/// Storage backends should return
///   - Ok(Some(..)) if the data exists
///   - Ok(None)     if the data does not exist
///   - Err(..)      only when something really wrong happens, for example
///                    - invariants are broken and observable from the storage side
///                      (this is not currently possible as ModuleId and StructTag
///                       are always structurally valid)
///                    - storage encounters internal error
pub trait ModuleBlobResolver {
    type Error: Debug;

    fn get_module_blob(&self, id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error>;
}

/// A persistent storage backend that can resolve resources by address + type.
/// Storage backends should return
///   - Ok(Some(..)) if the data exists
///   - Ok(None)     if the data does not exist
///   - Err(..)      only when something really wrong happens, for example
///                    - invariants are broken and observable from the storage side
///                      (this is not currently possible as ModuleId and StructTag
///                       are always structurally valid)
///                    - storage encounters internal error
pub trait ResourceBlobResolver {
    type Error: Debug;

    fn get_resource_blob(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error>;
}

/// A persistent storage implementation that can resolve both resources and modules, but
/// only as blobs. For resolver operating on non-blobs see `move-vm/types/resolver.rs`.
pub trait MoveBlobResolver: ModuleBlobResolver<Error = Self::Err> + ResourceBlobResolver<Error = Self::Err> {
    type Err: Debug;
}

impl<E: Debug, T: ModuleBlobResolver<Error = E> + ResourceBlobResolver<Error = E> + ?Sized> MoveBlobResolver for T {
    type Err = E;
}

impl<T: ResourceBlobResolver + ?Sized> ResourceBlobResolver for &T {
    type Error = T::Error;

    fn get_resource_blob(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Vec<u8>>, Self::Error> {
        (**self).get_resource_blob(address, tag)
    }
}

impl<T: ModuleBlobResolver + ?Sized> ModuleBlobResolver for &T {
    type Error = T::Error;

    fn get_module_blob(&self, module_id: &ModuleId) -> Result<Option<Vec<u8>>, Self::Error> {
        (**self).get_module_blob(module_id)
    }
}
