// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Traits for resolving Move resources from persistent storage at runtime.

use crate::values::Value;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::ModuleBlobResolver, value::MoveTypeLayout,
};
use std::{fmt::Debug, ops::Deref, sync::{Arc, Mutex, RwLock}};
use move_core_types::resolver::ResourceBlobResolver;

/// Encapsulates Move values so that they are thread-safe.
#[derive(Debug)]
pub struct ValueHandle(Arc<Mutex<Value>>);

impl Deref for ValueHandle {
    type Target = Arc<Mutex<Value>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Encapsulates `MoveTypeLayout` so it is read-only and thread-safe.
#[derive(Debug)]
pub struct ReadOnlyLayout(Arc<RwLock<MoveTypeLayout>>);

impl ReadOnlyLayout {
    pub fn read(&self) -> std::sync::LockResult<std::sync::RwLockReadGuard<'_, MoveTypeLayout>> {
        self.0.read()
    }

    pub fn try_read(&self) -> std::sync::TryLockResult<std::sync::RwLockReadGuard<'_, MoveTypeLayout>> {
        self.0.try_read()
    }
}

impl Deref for ReadOnlyLayout {
    type Target = Arc<RwLock<MoveTypeLayout>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Represents any resource stored in persistent storage or cache.
#[derive(Debug)]
pub enum Resource {
    // Resource is stored as a blob.
    Serialized(Arc<Vec<u8>>),
    // Resource is stored as a Move value and is not serialized yet. This type is
    // useful to cache outputs of VM session and avoid unnecessary deserialization.
    Cached(ValueHandle, ReadOnlyLayout),
}

impl Resource {
    /// Creates a new resource from Move value and its layout.
    pub fn from_value_layout(value: Value, layout: MoveTypeLayout) -> Resource {
        let value_handle = ValueHandle(Arc::new(Mutex::new(value)));
        let ro_layout = ReadOnlyLayout(Arc::new(RwLock::new(layout)));
        Resource::Cached(value_handle, ro_layout)
    }

    /// Creates a new resource from blob.
    pub fn from_blob(blob: Vec<u8>) -> Resource {
        Resource::Serialized(Arc::new(blob))
    }

    /// Serializes the resources into bytes.
    ///
    /// This function involves serialization or copying and is therefore expensive to use, and
    /// should be avoided if possible.
    pub fn serialize(&self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob.as_ref().clone()),
            Self::Cached(value_handle, layout) => {
                let value = value_handle.lock().unwrap();
                let layout = layout.read().unwrap();
                value.simple_serialize(&layout)
            }
        }
    }
}

// Implement clone to satisfy Clone bounds on `AccountRuntimeChangeSet` and `RuntimeChangeSet`.
impl Clone for Resource {
    fn clone(&self) -> Self {
        match self {
            Resource::Serialized(blob) => Resource::Serialized(Arc::clone(blob)),
            Resource::Cached(value_handle, layout) => {
                let value_handle = ValueHandle(Arc::clone(value_handle));
                let layout = ReadOnlyLayout(Arc::clone(layout));
                Resource::Cached(value_handle, layout)
            }
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
/// This trait is similar to `ResourceBlobResolver` but avoids serialization.
pub trait ResourceResolver {
    type Error: Debug;

    fn get_resource(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Resource>, Self::Error>;
}

/// A persistent storage implementation that can resolve both resources and
/// modules at runtime and avoid serialization if necessary.
pub trait MoveResolver: ModuleBlobResolver<Error = Self::Err> + ResourceBlobResolver<Error = Self::Err> + ResourceResolver<Error = Self::Err> {
    type Err: Debug;
}

impl<E: Debug, T: ModuleBlobResolver<Error = E> + ResourceBlobResolver<Error = E> + ResourceResolver<Error = E> + ?Sized> MoveResolver for T {
    type Err = E;
}

// TODO: Replace `ModuleBlobResolver` with `ModuleResolver` and define it here.
// TODO: Remove `ResourceBlobResolver`.

impl<T: ResourceResolver + ?Sized> ResourceResolver for &T {
    type Error = T::Error;

    fn get_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Resource>, Self::Error> {
        (**self).get_resource(address, tag)
    }
}
