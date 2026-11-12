// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource groups assembled at commit from the chain of metadata entries at
//! each group's own slot.
//!
//! Group members live in the data map under their own keys. A transaction that
//! writes members of a group also writes one metadata entry at the group's
//! slot, naming the members it changed. The slot's base entry holds the stored
//! group. A transaction's group is then the previous entry's view plus its own
//! changes, so nothing tracks which members a group has.

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
use mono_move_core::{nominal_tag, types::InternedType};
use move_core_types::language_storage::StructTag;
use std::{
    collections::BTreeMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, OnceLock},
};

/// A group's members, keyed by the BCS encoding of their tags. Byte order on
/// the keys is the order BCS writes map entries in, so encoding is a
/// concatenation.
pub(crate) type GroupView = BTreeMap<Bytes, Bytes>;

/// One entry at a group's slot: the stored group, or one transaction's changes
/// to it.
pub struct GroupMetadata<K> {
    /// The keys of the members the transaction that wrote this entry changed.
    /// Their new values are that transaction's entries under these keys.
    /// Empty for the stored group.
    changes: Vec<K>,
    /// The group's members after these changes. Set when the stored group is
    /// initialized, and built once for a transaction's entry.
    view: OnceLock<Arc<GroupView>>,
    /// The group's members before these changes. Only set in sequential
    /// execution, where this entry replaces the previous one at the group's
    /// slot before it is materialized.
    prev: OnceLock<Arc<GroupView>>,
}

impl<K> GroupMetadata<K> {
    /// The metadata for the group as storage holds it.
    pub(crate) fn from_storage(view: GroupView) -> Self {
        Self {
            changes: vec![],
            view: OnceLock::from(Arc::new(view)),
            prev: OnceLock::new(),
        }
    }

    /// The metadata for one transaction's changes to the group.
    pub(crate) fn from_changes(changes: Vec<K>) -> Self {
        Self {
            changes,
            view: OnceLock::new(),
            prev: OnceLock::new(),
        }
    }

    pub(crate) fn changes(&self) -> &[K] {
        &self.changes
    }

    /// Records the group's members before this entry's changes.
    pub(crate) fn set_prev(&self, prev: Arc<GroupView>) -> Result<(), PanicError> {
        self.prev
            .set(prev)
            .map_err(|_| code_invariant_error("Group metadata's previous view was set twice"))
    }

    pub(crate) fn prev(&self) -> Option<&Arc<GroupView>> {
        self.prev.get()
    }

    pub(crate) fn view(&self) -> Option<&Arc<GroupView>> {
        self.view.get()
    }

    /// This entry's view: `prev` with `changes` applied, each a member's key in
    /// the view and its new bytes, or [`None`] if it was deleted.
    pub(crate) fn build_view(
        &self,
        prev: &GroupView,
        changes: impl IntoIterator<Item = (Bytes, Option<Bytes>)>,
    ) -> Arc<GroupView> {
        let mut view = prev.clone();
        for (tag, bytes) in changes {
            match bytes {
                Some(bytes) => view.insert(tag, bytes),
                None => view.remove(&tag),
            };
        }
        // Another worker may have built the same view meanwhile. Whichever
        // landed first stands, and both are equal.
        self.view.get_or_init(|| Arc::new(view)).clone()
    }
}

/// The latest built view of the group before `txn_idx`, and the metadata
/// entries between it and `txn_idx` whose views are not built yet, lowest
/// first, each with the index of the transaction that wrote it.
///
/// Only called at commit, when every entry below `txn_idx` is committed.
#[allow(clippy::type_complexity)]
pub(crate) fn unbuilt_metadata_before<K: Hash + Clone + Debug + Eq + Send + Sync>(
    data: &VersionedData<K, MonoValue<K>>,
    group_key: &K,
    txn_idx: TxnIndex,
) -> Result<(Arc<GroupView>, Vec<(TxnIndex, Arc<GroupMetadata<K>>)>), PanicError> {
    let mut unbuilt = vec![];
    let mut idx = txn_idx;
    loop {
        match data.fetch_data_no_record(group_key, idx) {
            Ok(MVDataOutput::Versioned(version, MonoValue::GroupMetadata { group: metadata })) => {
                if let Some(view) = metadata.view() {
                    unbuilt.reverse();
                    return Ok((view.clone(), unbuilt));
                }
                match version {
                    Ok((writer_idx, _)) => {
                        unbuilt.push((writer_idx, metadata));
                        idx = writer_idx;
                    },
                    Err(_) => {
                        return Err(code_invariant_error(
                            "The stored group's metadata has no view",
                        ))
                    },
                }
            },
            Ok(MVDataOutput::Versioned(_, MonoValue::Write { .. } | MonoValue::Deletion)) => {
                return Err(code_invariant_error(format!(
                    "Group slot {group_key:?} holds a value that is not group metadata"
                )))
            },
            Err(MVDataError::Uninitialized) => {
                return Err(code_invariant_error(format!(
                    "Group {group_key:?} was written before it was read"
                )))
            },
            Err(MVDataError::Dependency(dep_idx)) => {
                return Err(code_invariant_error(format!(
                    "Group {group_key:?} has uncommitted metadata at {dep_idx} at commit"
                )))
            },
        }
    }
}

/// The value transaction `writer_idx` wrote at `key`.
///
/// Only called at commit, when the writer's entries are committed.
pub(crate) fn committed_value<K: Hash + Clone + Debug + Eq + Send + Sync>(
    data: &VersionedData<K, MonoValue<K>>,
    key: &K,
    writer_idx: TxnIndex,
) -> Result<MonoValue<K>, PanicError> {
    match data.fetch_data_no_record(key, writer_idx + 1) {
        Ok(MVDataOutput::Versioned(Ok((idx, _)), value)) if idx == writer_idx => Ok(value),
        Ok(MVDataOutput::Versioned(_, _)) | Err(_) => Err(code_invariant_error(format!(
            "Transaction {writer_idx} has no committed write at {key:?}"
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
