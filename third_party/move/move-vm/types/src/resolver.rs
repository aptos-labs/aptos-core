// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::values::FrozenValue;
use move_binary_format::CompiledModule;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
    value::MoveTypeLayout,
};
use std::{fmt::Debug, sync::Arc};

/// Reference to any Move data. It encapsulates implementation details about
/// how data is managed internally and should be efficiently cloneable.
#[derive(Clone, Debug)]
pub struct MoveRef<T>(T);

impl<T> MoveRef<T> {
    pub fn new(data: T) -> Self {
        Self(data)
    }
}

impl<T> AsRef<T> for MoveRef<T> {
    fn as_ref(&self) -> &T {
        &self.0
    }
}

pub type ModuleRef = MoveRef<Module>;
pub type ResourceRef = MoveRef<Resource>;

impl ModuleRef {
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        self.0.as_bytes()
    }
}

impl ResourceRef {
    pub fn into_bytes(self) -> Option<Vec<u8>> {
        self.0.as_bytes()
    }
}

/// Wrapper around any Move resource.
#[derive(Clone, Debug)]
pub enum Resource {
    /// Resource serialized as bytes.
    Serialized(Arc<Vec<u8>>),
    /// Non-serialized resource, with a type layout and its size in bytes.
    Cached(Arc<FrozenValue>, Arc<MoveTypeLayout>, usize),
}

impl Resource {
    pub fn from_value_layout(value: FrozenValue, layout: Arc<MoveTypeLayout>) -> Self {
        // TODO: FrozenValue should carry the size (we know it during construction), and so
        // we can pass it here. For now, use arbitrary value.
        Self::Cached(Arc::new(value), layout, 1)
    }

    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self::Serialized(Arc::new(blob))
    }

    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob.as_ref().clone()),
            Self::Cached(value, layout, _) => value.simple_serialize(layout),
        }
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob.as_ref().clone()),
            Self::Cached(value, layout, _) => value.simple_serialize(&layout),
        }
    }

    pub fn num_bytes(&self) -> usize {
        match self {
            Self::Serialized(blob) => blob.len(),
            Self::Cached(_, _, num_bytes) => *num_bytes,
        }
    }
}

pub trait ResourceRefResolver {
    type Error: Debug;

    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<ResourceRef>, Self::Error>;
}

impl<T: ResourceRefResolver + ?Sized> ResourceRefResolver for &T {
    type Error = T::Error;

    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<ResourceRef>, Self::Error> {
        (**self).get_resource_ref(address, tag)
    }
}

/// Wrapper around any Move module.
#[derive(Clone, Debug)]
pub enum Module {
    // Module serialized as blob.
    Serialized(Vec<u8>),
    // Non-serialized module representation.
    Cached(CompiledModule),
}

impl Module {
    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self::Serialized(blob)
    }

    pub fn from_compiled_module(compiled_module: CompiledModule) -> Self {
        Self::Cached(compiled_module)
    }

    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob.clone()),
            Self::Cached(compiled_module) => {
                let mut binary = vec![];
                compiled_module.serialize(&mut binary).ok()?;
                Some(binary)
            },
        }
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob),
            Self::Cached(compiled_module) => {
                let mut binary = vec![];
                compiled_module.serialize(&mut binary).ok()?;
                Some(binary)
            },
        }
    }
}

pub trait ModuleRefResolver {
    type Error: Debug;

    fn get_module_ref(&self, id: &ModuleId) -> Result<Option<ModuleRef>, Self::Error>;
}

impl<T: ModuleRefResolver + ?Sized> ModuleRefResolver for &T {
    type Error = T::Error;

    fn get_module_ref(&self, module_id: &ModuleId) -> Result<Option<ModuleRef>, Self::Error> {
        (**self).get_module_ref(module_id)
    }
}

pub trait MoveRefResolver:
    ModuleRefResolver<Error = Self::Err> + ResourceRefResolver<Error = Self::Err>
{
    type Err: Debug;
}

impl<E: Debug, T: ModuleRefResolver<Error = E> + ResourceRefResolver<Error = E> + ?Sized>
    MoveRefResolver for T
{
    type Err = E;
}
