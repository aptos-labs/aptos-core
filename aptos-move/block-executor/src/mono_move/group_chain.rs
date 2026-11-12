// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource groups assembled at commit from a chain of links at each group's
//! own slot.
//!
//! Group members live in the data map under their own keys. A transaction that
//! writes members of a group also writes one link at the group's slot, naming
//! the members it changed. The slot's base entry holds the stored group. A
//! transaction's group is then the previous link's view plus its own changes,
//! so nothing tracks which members a group has.

use crate::mono_move::MonoValue;
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, TxnIndex},
    versioned_data::VersionedData,
};
use aptos_types::{
    error::{code_invariant_error, PanicError},
    write_set::WriteOp,
};
use bytes::Bytes;
use mono_move_core::{
    nominal_tag, storage::resource_provider::InMemoryStorageKey, types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::serialize;
use move_core_types::language_storage::StructTag;
use std::{
    collections::BTreeMap,
    fmt::Debug,
    hash::Hash,
    ptr::NonNull,
    sync::{Arc, OnceLock},
};

/// A group's members, keyed by the BCS encoding of their tags. Byte order on
/// the keys is the order BCS writes map entries in, so encoding is a
/// concatenation.
pub(crate) type GroupView = BTreeMap<Bytes, Bytes>;

/// How group members are keyed and encoded.
pub(crate) trait GroupCodec<K> {
    /// The data-map key of the member of type `member_ty` in the group at
    /// `group_key`.
    fn member_key(&self, group_key: &K, member_ty: InternedType) -> K;

    /// The BCS encoding of the tag of the member of type `member_ty`.
    fn tag_bytes(&self, member_ty: InternedType) -> Result<Bytes, PanicError>;

    /// Serializes the member value of type `member_ty` that `ptr` points to.
    /// The caller holds the pin on the memory behind `ptr`.
    fn serialize(&self, ptr: NonNull<u8>, member_ty: InternedType) -> Result<Bytes, PanicError>;
}

/// The codec for MonoMove's own keys and values.
pub(crate) struct MonoGroupCodec<'a, 'ctx> {
    pub(crate) guard: &'a ExecutionGuard<'ctx>,
}

impl GroupCodec<InMemoryStorageKey> for MonoGroupCodec<'_, '_> {
    fn member_key(
        &self,
        group_key: &InMemoryStorageKey,
        member_ty: InternedType,
    ) -> InMemoryStorageKey {
        InMemoryStorageKey::resource(group_key.address(), member_ty)
    }

    fn tag_bytes(&self, member_ty: InternedType) -> Result<Bytes, PanicError> {
        member_tag(member_ty)
    }

    fn serialize(&self, ptr: NonNull<u8>, member_ty: InternedType) -> Result<Bytes, PanicError> {
        // SAFETY: the caller holds the pin on the frozen heap `ptr` points
        // into, and that heap is never mutated again.
        unsafe { serialize(self.guard, ptr.as_ptr(), member_ty) }
            .map(Bytes::from)
            .map_err(|e| code_invariant_error(format!("Failed to serialize a group member: {e}")))
    }
}

/// One entry at a group's slot: the stored group, or one transaction's changes
/// to it.
pub struct GroupLink {
    /// The types of the members the transaction that wrote this link changed.
    /// Their new values are that transaction's entries under the members' own
    /// keys. Empty for the stored group.
    changed: Vec<InternedType>,
    /// The group's members after these changes. Set when the stored group is
    /// initialized, and built once for a transaction's link.
    view: OnceLock<Arc<GroupView>>,
    /// The group's members before these changes. Only set in sequential
    /// execution, where this link replaces the previous one at the group's
    /// slot before it is materialized.
    prev: OnceLock<Arc<GroupView>>,
}

impl GroupLink {
    /// The link for the group as storage holds it.
    pub(crate) fn from_storage(view: GroupView) -> Self {
        Self {
            changed: vec![],
            view: OnceLock::from(Arc::new(view)),
            prev: OnceLock::new(),
        }
    }

    /// The link for one transaction's changes to the group.
    pub(crate) fn from_changes(changed: Vec<InternedType>) -> Self {
        Self {
            changed,
            view: OnceLock::new(),
            prev: OnceLock::new(),
        }
    }

    /// Records the group's members before this link's changes.
    pub(crate) fn set_prev(&self, prev: Arc<GroupView>) -> Result<(), PanicError> {
        self.prev
            .set(prev)
            .map_err(|_| code_invariant_error("A group link's previous view was set twice"))
    }

    pub(crate) fn prev(&self) -> Option<&Arc<GroupView>> {
        self.prev.get()
    }

    pub(crate) fn view(&self) -> Option<&Arc<GroupView>> {
        self.view.get()
    }

    /// This link's view, built on top of `prev` if no one built it yet.
    /// `written` returns the value the link's transaction wrote for a member
    /// type.
    pub(crate) fn build_view<K>(
        &self,
        codec: &impl GroupCodec<K>,
        prev: &GroupView,
        written: impl Fn(InternedType) -> Result<MonoValue, PanicError>,
    ) -> Result<Arc<GroupView>, PanicError> {
        if let Some(view) = self.view.get() {
            return Ok(view.clone());
        }
        let mut view = prev.clone();
        for member_ty in &self.changed {
            let tag = codec.tag_bytes(*member_ty)?;
            match written(*member_ty)? {
                MonoValue::Write { ptr, .. } => {
                    view.insert(tag, codec.serialize(ptr, *member_ty)?);
                },
                MonoValue::Deletion => {
                    view.remove(&tag);
                },
                MonoValue::GroupMetadata { .. } => {
                    return Err(code_invariant_error(
                        "A group member's written value is a group link",
                    ))
                },
            }
        }
        // Another worker may have built the same view meanwhile. Whichever
        // landed first stands, and both are equal.
        Ok(self.view.get_or_init(|| Arc::new(view)).clone())
    }
}

/// The group's view just before `txn_idx`, from the latest link below it.
/// Links below whose views are not built yet are built on the way.
///
/// Only called at commit, when every entry below `txn_idx` is committed.
pub(crate) fn view_before<K: Hash + Clone + Debug + Eq>(
    data: &VersionedData<K, MonoValue>,
    codec: &impl GroupCodec<K>,
    group_key: &K,
    txn_idx: TxnIndex,
) -> Result<Arc<GroupView>, PanicError> {
    let mut unbuilt = vec![];
    let mut idx = txn_idx;
    let mut view = loop {
        match data.fetch_data_no_record(group_key, idx) {
            Ok(MVDataOutput::Versioned(version, MonoValue::GroupMetadata { group: link })) => {
                if let Some(view) = link.view() {
                    break view.clone();
                }
                match version {
                    Ok((writer_idx, _)) => {
                        unbuilt.push((writer_idx, link));
                        idx = writer_idx;
                    },
                    Err(_) => {
                        return Err(code_invariant_error("The stored group's link has no view"))
                    },
                }
            },
            Ok(MVDataOutput::Versioned(_, MonoValue::Write { .. } | MonoValue::Deletion)) => {
                return Err(code_invariant_error(format!(
                    "Group slot {group_key:?} holds a value that is not a link"
                )))
            },
            Err(MVDataError::Uninitialized) => {
                return Err(code_invariant_error(format!(
                    "Group {group_key:?} was written before it was read"
                )))
            },
            Err(MVDataError::Dependency(dep_idx)) => {
                return Err(code_invariant_error(format!(
                    "Group {group_key:?} has an uncommitted link at {dep_idx} at commit"
                )))
            },
        }
    };
    for (writer_idx, link) in unbuilt.into_iter().rev() {
        view = link.build_view(codec, &view, |member_ty| {
            committed_member_value(data, codec, group_key, member_ty, writer_idx)
        })?;
    }
    Ok(view)
}

/// The value transaction `writer_idx` wrote for the group member of type
/// `member_ty`.
///
/// Only called at commit, when the writer's entries are committed.
pub(crate) fn committed_member_value<K: Hash + Clone + Debug + Eq>(
    data: &VersionedData<K, MonoValue>,
    codec: &impl GroupCodec<K>,
    group_key: &K,
    member_ty: InternedType,
    writer_idx: TxnIndex,
) -> Result<MonoValue, PanicError> {
    let key = codec.member_key(group_key, member_ty);
    match data.fetch_data_no_record(&key, writer_idx + 1) {
        Ok(MVDataOutput::Versioned(Ok((idx, _)), value)) if idx == writer_idx => Ok(value),
        Ok(MVDataOutput::Versioned(_, _)) | Err(_) => Err(code_invariant_error(format!(
            "Transaction {writer_idx} has no committed write for a member of {group_key:?}"
        ))),
    }
}

/// The write for a group's slot: creation, modification or deletion by whether
/// the group existed before and has members after.
//
// TODO(correctness): ops carry no `StateValueMetadata` (slot deposits, refunds,
// creation time); the legacy VM's `WriteOpConverter` fills these.
pub(crate) fn group_write(existed: bool, view: &GroupView) -> Result<WriteOp, PanicError> {
    Ok(match (existed, view.is_empty()) {
        (false, false) => WriteOp::legacy_creation(encode_group(view)),
        (true, false) => WriteOp::legacy_modification(encode_group(view)),
        (true, true) => WriteOp::legacy_deletion(),
        (false, true) => return Err(code_invariant_error("An empty group emitted a write")),
    })
}

/// The key of the member of type `member_ty` in a view.
//
// TODO(perf): cache the tag bytes per interned type.
pub(crate) fn member_tag(member_ty: InternedType) -> Result<Bytes, PanicError> {
    let tag = nominal_tag(member_ty)
        .map_err(|e| code_invariant_error(format!("Group member has no tag: {e:#}")))?;
    tag_bytes(&tag)
}

/// The BCS encoding of a member's struct tag, which keys it in a view.
pub(crate) fn tag_bytes(tag: &StructTag) -> Result<Bytes, PanicError> {
    bcs::to_bytes(tag)
        .map(Bytes::from)
        .map_err(|e| code_invariant_error(format!("Failed to encode a struct tag: {e}")))
}

/// BCS-encodes a group: the member count, then each entry's tag and
/// length-prefixed bytes in key order.
pub(crate) fn encode_group(view: &GroupView) -> Bytes {
    let len = uleb128_len(view.len())
        + view
            .iter()
            .map(|(tag, bytes)| tag.len() + uleb128_len(bytes.len()) + bytes.len())
            .sum::<usize>();
    let mut out = Vec::with_capacity(len);
    write_uleb128(&mut out, view.len());
    for (tag, bytes) in view {
        out.extend_from_slice(tag);
        write_uleb128(&mut out, bytes.len());
        out.extend_from_slice(bytes);
    }
    Bytes::from(out)
}

fn uleb128_len(mut value: usize) -> usize {
    let mut len = 1;
    while value >= 0x80 {
        value >>= 7;
        len += 1;
    }
    len
}

fn write_uleb128(out: &mut Vec<u8>, mut value: usize) {
    while value >= 0x80 {
        out.push((value as u8 & 0x7F) | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_matches_bcs() {
        // Keys whose byte order differs from their numeric order, and a value
        // long enough to need a two-byte length.
        let members = BTreeMap::from([
            (1u32, Bytes::from(vec![7; 200])),
            (256, Bytes::from(vec![1, 2])),
            (2, Bytes::new()),
        ]);
        let view = members
            .iter()
            .map(|(tag, bytes)| (Bytes::from(bcs::to_bytes(tag).unwrap()), bytes.clone()))
            .collect::<GroupView>();
        assert_eq!(encode_group(&view), bcs::to_bytes(&members).unwrap());
    }
}
