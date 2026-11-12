// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource groups assembled in post-commit materialization from the chain of
//! metadata entries at each group's own slot.
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
use std::{
    collections::BTreeMap,
    fmt::Debug,
    hash::Hash,
    sync::{Arc, OnceLock},
};

/// A group's members, keyed by the BCS encoding of their tags. Byte order on
/// the keys is the order BCS writes map entries in, so encoding is a
/// concatenation.
#[derive(Clone, Default)]
pub(crate) struct GroupView(BTreeMap<Bytes, Bytes>);

impl GroupView {
    /// The view of `members`, each a tag's BCS encoding and the member's bytes.
    pub(crate) fn new(members: impl IntoIterator<Item = (Bytes, Bytes)>) -> Self {
        Self(members.into_iter().collect())
    }

    /// The bytes of the member whose tag encodes to `tag`.
    pub(crate) fn get(&self, tag: &[u8]) -> Option<&Bytes> {
        self.0.get(tag)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Each member's tag encoding and bytes, in key order.
    #[cfg(test)]
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&Bytes, &Bytes)> {
        self.0.iter()
    }

    /// This view with `changes` applied, each a member's tag encoding and its
    /// new bytes, or [`None`] if it was deleted.
    fn with_changes(&self, changes: impl IntoIterator<Item = (Bytes, Option<Bytes>)>) -> Self {
        let mut members = self.0.clone();
        for (tag, bytes) in changes {
            match bytes {
                Some(bytes) => members.insert(tag, bytes),
                None => members.remove(&tag),
            };
        }
        Self(members)
    }

    /// The write for the group's slot: creation, modification or deletion by
    /// whether the group existed before and has members now.
    //
    // TODO(correctness): ops carry no `StateValueMetadata` (slot deposits,
    // refunds, creation time); the legacy VM's `WriteOpConverter` fills these.
    pub(crate) fn write_op(&self, existed: bool) -> Result<WriteOp, PanicError> {
        Ok(match (existed, self.is_empty()) {
            (false, false) => WriteOp::legacy_creation(self.encode()),
            (true, false) => WriteOp::legacy_modification(self.encode()),
            (true, true) => WriteOp::legacy_deletion(),
            (false, true) => return Err(code_invariant_error("An empty group emitted a write")),
        })
    }

    /// BCS-encodes the group: the member count, then each entry's tag and
    /// length-prefixed bytes in key order.
    pub(crate) fn encode(&self) -> Bytes {
        let len = uleb128_len(self.0.len())
            + self
                .0
                .iter()
                .map(|(tag, bytes)| tag.len() + uleb128_len(bytes.len()) + bytes.len())
                .sum::<usize>();
        let mut out = Vec::with_capacity(len);
        write_uleb128(&mut out, self.0.len());
        for (tag, bytes) in &self.0 {
            out.extend_from_slice(tag);
            write_uleb128(&mut out, bytes.len());
            out.extend_from_slice(bytes);
        }
        Bytes::from(out)
    }
}

/// One entry at a group's slot: the stored group, or one transaction's changes
/// to it.
pub struct GroupMetadata<K> {
    /// The keys of the members the transaction that wrote this entry changed.
    /// Their new values are that transaction's entries under these keys.
    /// Empty for the stored group.
    changes: Vec<K>,
    /// The group's members after these changes. Set when the stored group is
    /// initialized, and built once for a transaction's entry.
    current: OnceLock<Arc<GroupView>>,
    /// The group's members before these changes. Only set in sequential
    /// execution, where this entry replaces the previous one at the group's
    /// slot before it is materialized.
    before: OnceLock<Arc<GroupView>>,
}

impl<K> GroupMetadata<K> {
    /// The metadata for the group as storage holds it.
    pub(crate) fn from_storage(view: GroupView) -> Self {
        Self {
            changes: vec![],
            current: OnceLock::from(Arc::new(view)),
            before: OnceLock::new(),
        }
    }

    /// The metadata for one transaction's changes to the group.
    pub(crate) fn from_changes(changes: Vec<K>) -> Self {
        Self {
            changes,
            current: OnceLock::new(),
            before: OnceLock::new(),
        }
    }

    pub(crate) fn changes(&self) -> &[K] {
        &self.changes
    }

    /// Records the group's members before this entry's changes.
    pub(crate) fn set_before(&self, before: Arc<GroupView>) -> Result<(), PanicError> {
        self.before
            .set(before)
            .map_err(|_| code_invariant_error("Group metadata's before view was set twice"))
    }

    pub(crate) fn before(&self) -> Option<&Arc<GroupView>> {
        self.before.get()
    }

    pub(crate) fn current(&self) -> Option<&Arc<GroupView>> {
        self.current.get()
    }

    /// This entry's view: `prev` with `changes` applied, each a member's key in
    /// the view and its new bytes, or [`None`] if it was deleted.
    pub(crate) fn build_view(
        &self,
        prev: &GroupView,
        changes: impl IntoIterator<Item = (Bytes, Option<Bytes>)>,
    ) -> Arc<GroupView> {
        let view = prev.with_changes(changes);
        // Another worker may have built the same view meanwhile. Whichever
        // landed first stands, and both are equal.
        self.current.get_or_init(|| Arc::new(view)).clone()
    }
}

/// The latest built view of the group before `txn_idx`, and the metadata
/// entries between it and `txn_idx` whose views are not built yet, lowest
/// first, each with the index of the transaction that wrote it.
///
/// Only called in post-commit, when every entry below `txn_idx` is committed.
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
                if let Some(view) = metadata.current() {
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
                    "Group {group_key:?} has uncommitted metadata at {dep_idx} in post-commit"
                )))
            },
        }
    }
}

/// The value transaction `writer_idx` wrote at `key`.
///
/// Only called in post-commit, when the writer's entries are committed.
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
        let view = GroupView::new(
            members
                .iter()
                .map(|(tag, bytes)| (Bytes::from(bcs::to_bytes(tag).unwrap()), bytes.clone())),
        );
        assert_eq!(view.encode(), bcs::to_bytes(&members).unwrap());
    }
}
