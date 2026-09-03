// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_types::state_store::state_key::StateKey;
use bytes::Bytes;
use mono_move_core::storage::resource_provider::ResourceProvider;
use move_core_types::language_storage::StructTag;
use std::collections::{BTreeMap, HashMap};

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups.
pub trait AptosDataProvider: ResourceProvider {
    /// The stored members of the group behind `group_key`, as execution read
    /// them, or `None` if no group is stored there.
    fn group_members(&self, group_key: &StateKey) -> Result<Option<GroupMembers>>;
}

/// A resource group's members and their stored bytes.
//
// TODO(cleanup): consider using interned types and unordered map.
pub type GroupMembers = BTreeMap<StructTag, Bytes>;

/// The resource groups a transaction assembled during materialization, keyed by
/// group slot. `None` marks a group the transaction deleted.
pub type MaterializedGroups = HashMap<StateKey, Option<GroupMembers>>;

/// Decodes a group's stored blob.
pub fn decode_group_members(blob: &[u8]) -> Result<GroupMembers> {
    Ok(bcs::from_bytes(blob)?)
}
