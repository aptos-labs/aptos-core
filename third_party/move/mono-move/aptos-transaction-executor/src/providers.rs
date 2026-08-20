// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::Result;
use aptos_types::{state_store::state_key::StateKey, write_set::WriteOp};
use bytes::Bytes;
use mono_move_core::{
    intern_struct_tag,
    storage::resource_provider::{nominal_tag, ResourceProvider},
    types::InternedType,
    Interner,
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

    /// Builds the single write op for a resource group's slot from a
    /// transaction's own member ops, keyed by nominal struct tag: `Some(bytes)`
    /// writes or creates a member, `None` removes it.
    ///
    /// The default rebuilds the full member map from [`group_members`] on every
    /// call, so the emitted blob reflects only the stored members plus this
    /// transaction's ops. It carries the same stored-only limitation as
    /// `group_members`: a member another transaction added earlier in the block
    /// is not seen. Callers that materialize a block in commit order (Block-STM)
    /// override this with a running state that accumulates across transactions.
    ///
    /// [`group_members`]: AptosDataProvider::group_members
    fn group_write_op(
        &self,
        group_key: &StateKey,
        member_ops: HashMap<StructTag, Option<Bytes>>,
    ) -> Result<WriteOp> {
        let old = self.group_members(group_key)?;
        let mut members: BTreeMap<StructTag, Bytes> = BTreeMap::new();
        for (ty, bytes) in old.iter().flat_map(|group| group.iter()) {
            members.insert(nominal_tag(*ty)?, bytes.clone());
        }
        let mut exists = old.is_some();
        finalize_group(&mut members, &mut exists, member_ops)
    }
}

/// Overlays a transaction's `member_ops` onto `members` (the group's current,
/// canonically keyed state) in place and returns the write op for the group's
/// slot. `exists` is the slot's presence before this transaction; it is updated
/// to its presence after.
///
/// The blob is `bcs::to_bytes` of a `BTreeMap<StructTag, Bytes>`, which the
/// legacy VM also emits, so the encoding is byte-identical.
///
/// SAFETY: the result must be deterministic, not depending on the iteration
/// order of `member_ops`.
pub fn finalize_group(
    members: &mut BTreeMap<StructTag, Bytes>,
    exists: &mut bool,
    member_ops: HashMap<StructTag, Option<Bytes>>,
) -> Result<WriteOp> {
    for (tag, op) in member_ops {
        match op {
            Some(bytes) => {
                members.insert(tag, bytes);
            },
            None => {
                members.remove(&tag);
            },
        }
    }
    let new_bytes = if members.is_empty() {
        None
    } else {
        Some(Bytes::from(bcs::to_bytes(&*members)?))
    };
    let existed = *exists;
    *exists = new_bytes.is_some();
    group_op(existed, new_bytes)
}

/// Builds the write op for a resource-group slot: creation/modification/deletion
/// by whether the group existed before and has members after.
//
// TODO(correctness): ops carry no `StateValueMetadata` (slot deposits, refunds,
// creation time); the legacy VM's `WriteOpConverter` fills these.
fn group_op(existed: bool, new_bytes: Option<Bytes>) -> Result<WriteOp> {
    Ok(match (existed, new_bytes) {
        (false, Some(bytes)) => WriteOp::legacy_creation(bytes),
        (true, Some(bytes)) => WriteOp::legacy_modification(bytes),
        (true, None) => WriteOp::legacy_deletion(),
        // Unreachable: a member deletion implies the member (and so the group)
        // existed, and a creation leaves the group non-empty.
        (false, None) => anyhow::bail!("an empty group emitted a write"),
    })
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
