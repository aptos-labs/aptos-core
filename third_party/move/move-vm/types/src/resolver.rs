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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resource {
    Serialized(Vec<u8>),
    Cached(FrozenValue, MoveTypeLayout),
}

impl Resource {
    pub fn from_value_layout(value: FrozenValue, layout: MoveTypeLayout) -> Self {
        Self::Cached(value, layout)
    }

    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self::Serialized(blob)
    }

    pub fn as_bytes(&self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob.clone()),
            Self::Cached(value, layout) => value.simple_serialize(layout),
        }
    }

    pub fn into_bytes(self) -> Option<Vec<u8>> {
        match self {
            Self::Serialized(blob) => Some(blob),
            Self::Cached(value, layout) => value.simple_serialize(&layout),
        }
    }
}

pub trait FrozenResourceResolver {
    type Error: Debug;

    fn get_frozen_resource(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> Result<Option<Arc<Resource>>, Self::Error>;
}

impl<T: FrozenResourceResolver + ?Sized> FrozenResourceResolver for &T {
    type Error = T::Error;

    fn get_frozen_resource(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> Result<Option<Arc<Resource>>, Self::Error> {
        (**self).get_frozen_resource(address, tag)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Module {
    Serialized(Vec<u8>),
    Cached(CompiledModule),
}

impl Module {
    pub fn from_blob(blob: Vec<u8>) -> Self {
        Self::Serialized(blob)
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

    // TODO: conversion to compiled module.
}

pub trait FrozenModuleResolver {
    type Error: Debug;

    fn get_frozen_module(&self, id: &ModuleId) -> Result<Option<Arc<Module>>, Self::Error>;
}

impl<T: FrozenModuleResolver + ?Sized> FrozenModuleResolver for &T {
    type Error = T::Error;

    fn get_frozen_module(&self, module_id: &ModuleId) -> Result<Option<Arc<Module>>, Self::Error> {
        (**self).get_frozen_module(module_id)
    }
}

pub trait FrozenMoveResolver:
    FrozenModuleResolver<Error = Self::Err> + FrozenResourceResolver<Error = Self::Err>
{
    type Err: Debug;
}

impl<E: Debug, T: FrozenModuleResolver<Error = E> + FrozenResourceResolver<Error = E> + ?Sized>
    FrozenMoveResolver for T
{
    type Err = E;
}
