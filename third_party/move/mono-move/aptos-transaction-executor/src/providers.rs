// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Result};
use bytes::Bytes;
use mono_move_core::{
    intern_struct_tag,
    storage::resource_provider::{InMemoryStorageKey, ResourceProvider},
    struct_tag_of,
    types::InternedType,
    Interner,
};
use move_core_types::language_storage::StructTag;
use std::collections::{BTreeMap, HashMap};
use triomphe::Arc;

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups and table items.
pub trait AptosDataProvider: ResourceProvider {
    /// The stored members of the group behind the in-memory `group_key`, as
    /// execution read them, or `None` if no group is stored there.
    fn group_members(&self, group_key: &InMemoryStorageKey) -> Result<Option<Arc<GroupMembers>>>;

    /// Records that `member` belongs to the group behind `group_key`, so a
    /// later [`group_members`](Self::group_members) enumeration can include it.
    /// The default is a no-op for providers that do not maintain a reverse
    /// member index.
    fn note_group_member(
        &self,
        _group_key: &InMemoryStorageKey,
        _member: InternedType,
    ) -> Result<()> {
        Ok(())
    }
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

/// The struct tag of a nominal type, for storage keys.
//
// TODO(perf): should be a cached method on the context, which would also let the
// state-view providers stop open-coding it.
pub(crate) fn nominal_tag(ty: InternedType) -> Result<StructTag> {
    struct_tag_of(ty).ok_or_else(|| anyhow!("resource type is not nominal"))
}
