// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Traits for resolving Move resources from persistent storage at runtime.

use crate::values::FrozenValue;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::StructTag,
    resolver::{ModuleResolver, ResourceResolver},
    value::MoveTypeLayout,
};
use std::{fmt::Debug, sync::Arc};

/// Represents any resource stored in persistent storage or cache.
#[derive(Debug, PartialEq, Eq)]
pub enum Resource {
    // Resource is stored as a blob.
    Serialized(Arc<Vec<u8>>),
    // Resource is stored as a Move value and is not serialized yet. This type is
    // useful to cache outputs of VM session and avoid unnecessary deserialization.
    Cached(Arc<FrozenValue>, Arc<MoveTypeLayout>),
}

impl Resource {
    /// Creates a new resource from Move value and its layout.
    pub fn from_value_layout(value: FrozenValue, layout: MoveTypeLayout) -> Resource {
        Resource::Cached(Arc::new(value), Arc::new(layout))
    }

    /// Creates a new resource from blob.
    pub fn from_blob(blob: Vec<u8>) -> Resource {
        Resource::Serialized(Arc::new(blob))
    }

    /// Serializes the resources into bytes.
    pub fn serialize(&self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob.as_ref().clone()),
            Self::Cached(value, layout) => {
                value.simple_serialize(layout.as_ref())
            },
        }
    }
}

// Implement clone to satisfy Clone bounds on `AccountChangeSet` and `ChangeSet`.
impl Clone for Resource {
    fn clone(&self) -> Self {
        match self {
            Resource::Serialized(blob) => Resource::Serialized(Arc::clone(blob)),
            Resource::Cached(value, layout) => Resource::Cached(Arc::clone(value), Arc::clone(layout)),
        }
    }
}

/// Any persistent storage backend or cache that can resolve resources by
/// address and type at runtime. Storage backends should return:
///   - Ok(Some(..)) if the data exists
///   - Ok(None)     if the data does not exist
///   - Err(..)      only when something really wrong happens, for example
///                    - invariants are broken and observable from the storage side
///                      (this is not currently possible as ModuleId and StructTag
///                       are always structurally valid)
///                    - storage encounters internal error
///
/// This trait is similar to `ResourceResolver` but avoids serialization.
pub trait ResourceResolverV2 {
    type Error: Debug;

    fn get_resource_v2(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Resource>, Self::Error>;
}

/// A persistent storage implementation that can resolve both resources and
/// modules at runtime and avoid serialization if necessary.
pub trait MoveResolverV2:
    ModuleResolver<Error = Self::Err>
    + ResourceResolver<Error = Self::Err>
    + ResourceResolverV2<Error = Self::Err>
{
    type Err: Debug;
}

impl<
        E: Debug,
        T: ModuleResolver<Error = E>
            + ResourceResolver<Error = E>
            + ResourceResolverV2<Error = E>
            + ?Sized,
    > MoveResolverV2 for T
{
    type Err = E;
}

// TODO: Currently `MoveResolver` has `ModuleResolver` and `ResourceResolver` which operate on
// blobs. When we switch to values over blobs `ResourceBlobResolver` bound should be removed.
// Similarly, when we cache compiled modules we should use a new trait.

impl<T: ResourceResolverV2 + ?Sized> ResourceResolverV2 for &T {
    type Error = T::Error;

    fn get_resource_v2(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Resource>, Self::Error> {
        (**self).get_resource_v2(address, tag)
    }
}
