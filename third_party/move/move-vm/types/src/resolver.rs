// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::types::ResourceRef;
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, resolver::ModuleResolver,
};

pub trait ResourceRefResolver {
    /// Returns a resource reference if it exists.
    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> anyhow::Result<Option<ResourceRef>>;

    /// Returns resource bytes if the resource exists. This allows avoiding
    /// passing MoveTypeLayout between MoveVM and cache/storage.
    fn get_resource_bytes(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> anyhow::Result<Option<Vec<u8>>>;
}

impl<T: ResourceRefResolver> ResourceRefResolver for &T {
    fn get_resource_ref(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> anyhow::Result<Option<ResourceRef>> {
        (**self).get_resource_ref(address, tag)
    }

    fn get_resource_bytes(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        (**self).get_resource_bytes(address, tag)
    }
}

pub trait MoveRefResolver: ModuleResolver + ResourceRefResolver {}

impl<T: ModuleResolver + ResourceRefResolver> MoveRefResolver for T {}
