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
use aptos_infallible::Mutex;
use aptos_mvhashmap::{
    types::{MVDataError, MVDataOutput, TxnIndex, Version},
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
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, OnceLock,
    },
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

/// Whether a metadata entry's view of its group is available.
#[derive(Clone)]
enum GroupViewState {
    /// Not built yet.
    None,
    Built(Arc<GroupView>),
    /// Dropped once no post-commit reads it any more.
    Released,
}

/// One entry at a group's slot: the stored group, or one transaction's changes
/// to it.
pub struct GroupMetadata<K> {
    /// The keys of the members the transaction that wrote this entry changed.
    /// Their new values are that transaction's entries under these keys.
    /// Empty for the stored group.
    changes: Vec<K>,
    /// The group's members after these changes. Built when the stored group is
    /// initialized, or once for a transaction's entry.
    current: Mutex<GroupViewState>,
    /// The post-commits that still read `current`: this transaction's own, and
    /// the next writer's, which builds on it. The view is released when both
    /// finished, as later writers stop at the next writer's view.
    pending_post_commits: AtomicU8,
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
            current: Mutex::new(GroupViewState::Built(Arc::new(view))),
            // Never released: execution reads the stored group.
            pending_post_commits: AtomicU8::new(u8::MAX),
            before: OnceLock::new(),
        }
    }

    /// The metadata for one transaction's changes to the group.
    pub(crate) fn from_changes(changes: Vec<K>) -> Self {
        Self {
            changes,
            current: Mutex::new(GroupViewState::None),
            pending_post_commits: AtomicU8::new(2),
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

    /// The group's members after these changes, if built and not released.
    pub(crate) fn current(&self) -> Option<Arc<GroupView>> {
        match &*self.current.lock() {
            GroupViewState::Built(view) => Some(view.clone()),
            GroupViewState::None | GroupViewState::Released => None,
        }
    }

    /// This entry's view: `prev` with `changes` applied, each a member's key in
    /// the view and its new bytes, or [`None`] if it was deleted.
    pub(crate) fn build_view(
        &self,
        prev: &GroupView,
        changes: impl IntoIterator<Item = (Bytes, Option<Bytes>)>,
    ) -> Arc<GroupView> {
        let view = Arc::new(prev.with_changes(changes));
        let mut state = self.current.lock();
        match &*state {
            GroupViewState::None => {
                *state = GroupViewState::Built(view.clone());
                view
            },
            // Another worker built the same view meanwhile.
            GroupViewState::Built(built) => built.clone(),
            // No one reads a released view, so it is not stored again.
            GroupViewState::Released => view,
        }
    }

    /// Records that one of the post-commits reading this entry's view
    /// finished, and releases the view after the last one.
    fn finish_post_commit(&self) {
        if self.changes.is_empty() {
            return;
        }
        if self.pending_post_commits.fetch_sub(1, Ordering::AcqRel) == 1 {
            *self.current.lock() = GroupViewState::Released;
        }
    }
}

/// The latest built view of the group before `txn_idx`, and the metadata
/// entries between it and `txn_idx` whose views are not built yet, lowest
/// first, each with the index of the transaction that wrote it.
///
/// Only called in post-commit by a transaction that wrote the group, when
/// every entry below `txn_idx` is committed.
#[allow(clippy::type_complexity)]
pub(crate) fn unbuilt_metadata_before<K: Hash + Clone + Debug + Eq + Send + Sync>(
    data: &VersionedData<K, MonoValue<K>>,
    group_key: &K,
    txn_idx: TxnIndex,
) -> Result<(Arc<GroupView>, Vec<(TxnIndex, Arc<GroupMetadata<K>>)>), PanicError> {
    let mut unbuilt: Vec<(TxnIndex, Arc<GroupMetadata<K>>)> = vec![];
    let mut idx = txn_idx;
    let view = 'walk: loop {
        let (version, metadata) = metadata_before(data, group_key, idx)?;
        let state = metadata.current.lock().clone();
        match state {
            GroupViewState::Built(view) => break view,
            // The next entry's post-commit finished, so that entry's view is
            // built, unless it was released too. The entry just below `txn_idx`
            // is only released after this transaction's post-commit, so
            // stepping back up ends at a built view.
            GroupViewState::Released => loop {
                let (_, next) = unbuilt.pop().ok_or_else(|| {
                    code_invariant_error(format!(
                        "Group {group_key:?} released the metadata below {txn_idx}"
                    ))
                })?;
                let next_state = next.current.lock().clone();
                match next_state {
                    GroupViewState::Built(view) => break 'walk view,
                    GroupViewState::Released => {},
                    GroupViewState::None => {
                        return Err(code_invariant_error(format!(
                            "Group {group_key:?} released metadata below an unbuilt entry"
                        )))
                    },
                }
            },
            GroupViewState::None => match version {
                Ok((writer_idx, _)) => {
                    unbuilt.push((writer_idx, metadata));
                    idx = writer_idx;
                },
                Err(_) => {
                    return Err(code_invariant_error(
                        "The stored group's metadata has no view",
                    ))
                },
            },
        }
    };
    unbuilt.reverse();
    Ok((view, unbuilt))
}

/// Records that the post-commit of `txn_idx`, which wrote `metadata` at the
/// group's slot, finished: it no longer reads its own view, nor the view of
/// the entry just below it.
pub(crate) fn finish_group_post_commit<K: Hash + Clone + Debug + Eq + Send + Sync>(
    data: &VersionedData<K, MonoValue<K>>,
    group_key: &K,
    txn_idx: TxnIndex,
    metadata: &GroupMetadata<K>,
) -> Result<(), PanicError> {
    metadata.finish_post_commit();
    metadata_before(data, group_key, txn_idx)?
        .1
        .finish_post_commit();
    Ok(())
}

/// The latest metadata entry at the group's slot below `idx`, with its
/// version.
///
/// Only called in post-commit, when every entry below `idx` is committed.
fn metadata_before<K: Hash + Clone + Debug + Eq + Send + Sync>(
    data: &VersionedData<K, MonoValue<K>>,
    group_key: &K,
    idx: TxnIndex,
) -> Result<(Version, Arc<GroupMetadata<K>>), PanicError> {
    match data.fetch_data_no_record(group_key, idx) {
        Ok(MVDataOutput::Versioned(version, MonoValue::GroupMetadata { group })) => {
            Ok((version, group))
        },
        Ok(MVDataOutput::Versioned(_, MonoValue::Write { .. } | MonoValue::Deletion)) => {
            Err(code_invariant_error(format!(
                "Group slot {group_key:?} holds a value that is not group metadata"
            )))
        },
        Err(MVDataError::Uninitialized) => Err(code_invariant_error(format!(
            "Group {group_key:?} was written before it was read"
        ))),
        Err(MVDataError::Dependency(dep_idx)) => Err(code_invariant_error(format!(
            "Group {group_key:?} has uncommitted metadata at {dep_idx} in post-commit"
        ))),
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
    use aptos_mvhashmap::MVHashMap;
    use move_vm_types::delayed_values::delayed_field_id::DelayedFieldID;

    fn tag(idx: TxnIndex) -> Bytes {
        Bytes::from(idx.to_le_bytes().to_vec())
    }

    /// Runs the post-commit of writer `txn_idx` the way materialization does,
    /// each writer adding a member keyed by its own index.
    fn post_commit(
        data: &VersionedData<u32, MonoValue<u32>>,
        txn_idx: TxnIndex,
        metadata: &GroupMetadata<u32>,
    ) -> Arc<GroupView> {
        let build = |metadata: &GroupMetadata<u32>, prev: &GroupView, idx: TxnIndex| {
            metadata
                .current()
                .unwrap_or_else(|| metadata.build_view(prev, [(tag(idx), Some(tag(idx)))]))
        };
        let (mut prev, unbuilt) = unbuilt_metadata_before(data, &0, txn_idx).unwrap();
        for (writer_idx, unbuilt) in unbuilt {
            prev = build(&unbuilt, &prev, writer_idx);
        }
        let view = build(metadata, &prev, txn_idx);
        finish_group_post_commit(data, &0, txn_idx, metadata).unwrap();
        view
    }

    #[test]
    fn views_are_released_once_no_post_commit_reads_them() {
        let map = MVHashMap::<u32, (), MonoValue<u32>, DelayedFieldID>::new();
        let data = map.data();
        let stored = Arc::new(GroupMetadata::from_storage(GroupView::default()));
        data.set_base_value(
            0,
            MonoValue::GroupMetadata {
                group: stored.clone(),
            },
            |_, _| {},
        );
        let [first, second, third] = [1, 3, 5].map(|idx| {
            let metadata = Arc::new(GroupMetadata::from_changes(vec![idx]));
            data.write(0, idx, 0, MonoValue::GroupMetadata {
                group: metadata.clone(),
            })
            .unwrap();
            metadata
        });

        // Post-commits finish out of order: 1, then 5, then 3.
        post_commit(data, 1, &first);
        assert!(first.current().is_some());

        // 5 builds 3's view on the way. 3's post-commit has not finished, so
        // neither its view nor the one below it is released.
        let third_view = post_commit(data, 5, &third);
        assert!(first.current().is_some());
        let second_view = second.current().expect("5 built 3's view");

        // 3 finds its own view built and releases what no one reads any more.
        assert!(Arc::ptr_eq(&post_commit(data, 3, &second), &second_view));
        assert!(first.current().is_none());
        assert!(second.current().is_none());
        // The last writer's view and the stored group stay.
        assert!(Arc::ptr_eq(&third.current().unwrap(), &third_view));
        assert!(stored.current().is_some());
        assert_eq!(
            third_view.encode(),
            GroupView::new([1, 3, 5].map(|idx| (tag(idx), tag(idx)))).encode()
        );
    }

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
