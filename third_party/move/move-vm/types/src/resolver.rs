// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Traits for resolving Move resources from persistent storage at runtime.

use crate::values::Value;
use move_binary_format::errors::{PartialVMError, PartialVMResult};
use move_core_types::{
    account_address::AccountAddress,
    language_storage::StructTag,
    resolver::{ModuleResolver, ResourceResolver},
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
use std::{
    fmt::Debug,
    sync::{Arc, Mutex, RwLock},
};

/// Encapsulates Move values so that they are thread-safe.
#[derive(Debug)]
pub struct ValueMutex(Arc<Mutex<Value>>);

/// Error to propagate to VM instead of panicking on poisoned lock.
fn poison_error() -> PartialVMError {
    PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR)
        .with_message("cannot handle poisoned values: {:?}".to_string())
}

impl ValueMutex {
    pub fn new(value: Value) -> Self {
        Self(Arc::new(Mutex::new(value)))
    }

    pub fn lock(&self) -> PartialVMResult<std::sync::MutexGuard<'_, Value>> {
        self.0.lock().map_err(|_| poison_error())
    }

    pub fn try_lock(&self) -> PartialVMResult<std::sync::MutexGuard<'_, Value>> {
        self.0.try_lock().map_err(|_| poison_error())
    }
}

impl Clone for ValueMutex {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

/// Encapsulates `MoveTypeLayout` so it is read-only and thread-safe.
#[derive(Debug)]
pub struct ReadOnlyLayout(Arc<RwLock<MoveTypeLayout>>);

impl ReadOnlyLayout {
    pub fn new(move_type_layout: MoveTypeLayout) -> Self {
        Self(Arc::new(RwLock::new(move_type_layout)))
    }

    pub fn read(&self) -> PartialVMResult<std::sync::RwLockReadGuard<'_, MoveTypeLayout>> {
        self.0
            .read()
            .map_err(|_| PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR))
    }

    pub fn try_read(&self) -> PartialVMResult<std::sync::RwLockReadGuard<'_, MoveTypeLayout>> {
        self.0
            .try_read()
            .map_err(|_| PartialVMError::new(StatusCode::UNKNOWN_INVARIANT_VIOLATION_ERROR))
    }
}

impl Clone for ReadOnlyLayout {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

/// Represents any resource stored in persistent storage or cache.
#[derive(Debug)]
pub enum Resource {
    // Resource is stored as a blob.
    Serialized(Arc<Vec<u8>>),
    // Resource is stored as a Move value and is not serialized yet. This type is
    // useful to cache outputs of VM session and avoid unnecessary deserialization.
    Cached(ValueMutex, ReadOnlyLayout),
}

impl Resource {
    /// Creates a new resource from Move value and its layout.
    pub fn from_value_layout(value: Value, layout: MoveTypeLayout) -> Resource {
        Resource::Cached(ValueMutex::new(value), ReadOnlyLayout::new(layout))
    }

    /// Creates a new resource from blob.
    pub fn from_blob(blob: Vec<u8>) -> Resource {
        Resource::Serialized(Arc::new(blob))
    }

    /// Serializes the resources into bytes.
    ///
    /// This function involves serialization or copying and is therefore expensive to use, and
    /// should be avoided if possible.
    pub fn serialize(&self) -> PartialVMResult<Option<Vec<u8>>> {
        match self {
            Self::Serialized(blob) => Ok(Some(blob.as_ref().clone())),
            Self::Cached(value, layout) => {
                let value = value.lock()?;
                let layout = layout.read()?;
                Ok(value.simple_serialize(&layout))
            },
        }
    }
}

// Implement clone to satisfy Clone bounds on `AccountChangeSet` and `ChangeSet`.
impl Clone for Resource {
    fn clone(&self) -> Self {
        match self {
            Resource::Serialized(blob) => Resource::Serialized(Arc::clone(blob)),
            Resource::Cached(value, layout) => Resource::Cached(value.clone(), layout.clone()),
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
