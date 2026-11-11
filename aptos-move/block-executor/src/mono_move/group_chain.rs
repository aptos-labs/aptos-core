// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource groups assembled in post-commit from a chain of links at each
//! group's own slot.
//!
//! Group members live in the data map under their own keys. A transaction that
//! writes members of a group also writes one link at the group's slot, holding
//! its member changes. The slot's base entry holds the stored group. A
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
    sync::{Arc, OnceLock},
};

/// A group's members, keyed by the BCS encoding of their struct tags. Byte
/// order on the keys is the order BCS writes map entries in, so encoding is a
/// concatenation.
pub(crate) type GroupView = BTreeMap<Bytes, Bytes>;

/// One entry at a group's slot: the stored group, or one transaction's changes
/// to it.
pub struct GroupLink {
    /// The changes of the transaction that wrote this link, by member type: a
    /// new value, or a deletion. Empty for the stored group.
    changes: Vec<(InternedType, MonoValue)>,
    /// The group's members after these changes. Set when the stored group is
    /// provisioned, and built once for a transaction's link.
    view: OnceLock<Arc<GroupView>>,
}

impl GroupLink {
    /// The link for the group as storage holds it.
    pub(crate) fn stored(view: GroupView) -> Self {
        Self {
            changes: vec![],
            view: OnceLock::from(Arc::new(view)),
        }
    }

    /// The link for one transaction's changes to the group.
    pub(crate) fn changes(changes: Vec<(InternedType, MonoValue)>) -> Self {
        Self {
            changes,
            view: OnceLock::new(),
        }
    }

    pub(crate) fn built_view(&self) -> Option<&Arc<GroupView>> {
        self.view.get()
    }

    /// This link's view, built on top of `prev` if no one built it yet.
    pub(crate) fn view_on(
        &self,
        guard: &ExecutionGuard<'_>,
        prev: &GroupView,
    ) -> Result<Arc<GroupView>, PanicError> {
        if let Some(view) = self.view.get() {
            return Ok(view.clone());
        }
        let mut view = prev.clone();
        for (member_ty, value) in &self.changes {
            let tag = nominal_tag(*member_ty)
                .map_err(|e| code_invariant_error(format!("Group member has no tag: {e:#}")))?;
            let tag = tag_bytes(&tag)?;
            match value {
                MonoValue::Write { ptr, ty, .. } => {
                    // SAFETY: the link holds the value, and with it the pin on
                    // the frozen heap the pointer points into.
                    let blob = unsafe { serialize(guard, ptr.as_ptr(), *ty) }.map_err(|e| {
                        code_invariant_error(format!("Failed to serialize a group member: {e}"))
                    })?;
                    view.insert(tag, Bytes::from(blob));
                },
                MonoValue::Deletion => {
                    view.remove(&tag);
                },
                MonoValue::ResourceGroupMemberBytes(_)
                | MonoValue::GroupMetadata
                | MonoValue::GroupLink(_) => {
                    return Err(code_invariant_error(
                        "A group link holds a change that is not a member write",
                    ))
                },
            }
        }
        // Another worker may have built the same view meanwhile; whichever
        // landed first stands, and both are equal.
        Ok(self.view.get_or_init(|| Arc::new(view)).clone())
    }
}

/// The view of a stored group, from its decoded members.
pub(crate) fn stored_view(
    members: impl IntoIterator<Item = (StructTag, Bytes)>,
) -> Result<GroupView, PanicError> {
    members
        .into_iter()
        .map(|(tag, bytes)| Ok((tag_bytes(&tag)?, bytes)))
        .collect()
}

/// The group's view just before `txn_idx`, from the latest link below it.
/// Links below whose views are not built yet are built on the way.
///
/// Only called in post-commit, when every entry below `txn_idx` is committed.
pub(crate) fn view_below(
    data: &VersionedData<InMemoryStorageKey, MonoValue>,
    guard: &ExecutionGuard<'_>,
    group_key: &InMemoryStorageKey,
    txn_idx: TxnIndex,
) -> Result<Arc<GroupView>, PanicError> {
    let mut unbuilt = vec![];
    let mut idx = txn_idx;
    let mut view = loop {
        match data.fetch_data_no_record(group_key, idx) {
            Ok(MVDataOutput::Versioned(version, MonoValue::GroupLink(link))) => {
                if let Some(view) = link.built_view() {
                    break view.clone();
                }
                match version {
                    Ok((writer_idx, _)) => {
                        unbuilt.push(link);
                        idx = writer_idx;
                    },
                    Err(_) => {
                        return Err(code_invariant_error("The stored group's link has no view"))
                    },
                }
            },
            Ok(MVDataOutput::Versioned(_, _)) => {
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
                    "Group {group_key:?} has an uncommitted link at {dep_idx} in post-commit"
                )))
            },
        }
    };
    for link in unbuilt.into_iter().rev() {
        view = link.view_on(guard, &view)?;
    }
    Ok(view)
}

/// The write for a group's slot: creation, modification or deletion by whether
/// the group existed before and has members after.
pub(crate) fn group_write(existed: bool, view: &GroupView) -> Result<WriteOp, PanicError> {
    Ok(match (existed, view.is_empty()) {
        (false, false) => WriteOp::legacy_creation(encode(view)),
        (true, false) => WriteOp::legacy_modification(encode(view)),
        (true, true) => WriteOp::legacy_deletion(),
        (false, true) => return Err(code_invariant_error("An empty group emitted a write")),
    })
}

fn tag_bytes(tag: &StructTag) -> Result<Bytes, PanicError> {
    bcs::to_bytes(tag)
        .map(Bytes::from)
        .map_err(|e| code_invariant_error(format!("Failed to encode a struct tag: {e}")))
}

/// BCS-encodes a group: the member count, then each entry's tag and
/// length-prefixed bytes in key order.
fn encode(view: &GroupView) -> Bytes {
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
