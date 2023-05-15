// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::types::ResourceRef;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::StructTag,
    metadata::Metadata,
    resolver::ModuleResolver,
};

pub trait ResourceRefResolver {
    /// Returns a resource reference if it exists.
    fn get_resource_ref_with_metadata(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<Option<ResourceRef>>;

    /// Returns resource bytes if the resource exists. This allows avoiding
    /// passing MoveTypeLayout between MoveVM and cache/storage.
    fn get_resource_bytes_with_metadata(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<Option<Vec<u8>>>;
}

impl<T: ResourceRefResolver> ResourceRefResolver for &T {
    fn get_resource_ref_with_metadata(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<Option<ResourceRef>> {
        (**self).get_resource_ref_with_metadata(address, tag, metadata)
    }

    fn get_resource_bytes_with_metadata(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        metadata: &[Metadata],
    ) -> anyhow::Result<Option<Vec<u8>>> {
        (**self).get_resource_bytes_with_metadata(address, tag, metadata)
    }
}

pub trait MoveRefResolver: ModuleResolver + ResourceRefResolver {
    fn get_resource(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> anyhow::Result<Option<ResourceRef>> {
        self.get_resource_ref_with_metadata(address, typ, &self.get_module_metadata(&typ.module_id()))
    }

    fn get_resource_bytes(
        &self,
        address: &AccountAddress,
        typ: &StructTag,
    ) -> anyhow::Result<Option<Vec<u8>>> {
        self.get_resource_bytes_with_metadata(address, typ, &self.get_module_metadata(&typ.module_id()))
    }
}

impl<T: ModuleResolver + ResourceRefResolver> MoveRefResolver for T {}
