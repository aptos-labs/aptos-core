// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_types::state_store::state_key::StateKey;
use bytes::Bytes;
use mono_move_core::{
    intern_struct_tag, storage::resource_provider::ResourceProvider, types::InternedType, Interner,
};
use move_core_types::language_storage::StructTag;
use std::collections::{BTreeMap, HashMap};
use triomphe::Arc;

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups.
pub trait AptosDataProvider: ResourceProvider {
    /// The stored members of the group behind `group_key`, as execution read
    /// them, or `None` if no group is stored there.
    fn group_members(&self, group_key: &StateKey) -> Result<Option<Arc<GroupMembers>>>;
}

/// A resource group's members and their stored bytes. Unordered: the stored
/// encoding is canonical in struct-tag order.
//
// TODO(cleanup): consider `shared-dsa`'s `UnorderedSet` and make it explicit that
// the iteration order can be non-deterministic.
pub type GroupMembers = HashMap<InternedType, Bytes>;

/// Decodes a group's stored blob, interning each member's struct tag.
pub fn decode_group_members(blob: &[u8], interner: &impl Interner) -> Result<GroupMembers> {
    let tagged: BTreeMap<StructTag, Bytes> = bcs::from_bytes(blob)?;
    tagged
        .into_iter()
        .map(|(tag, bytes)| Ok((intern_struct_tag(&tag, interner)?, bytes)))
        .collect()
}
