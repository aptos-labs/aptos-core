// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction global-storage state.
//!
//! Tracks every resource a transaction touches in a [`ResourceReadWriteSet`]:
//! a map from [`InMemoryStorageKey`] to an [`Entry`], plus an undo journal and a
//! checkpoint stack for rollback.
//!
//! # The read-write set
//!
//! Each [`Entry`] contains:
//!
//! - `read` ([`StorageRead`]) — the value as seen at the start of the
//!   transaction, fetched from DB, Block-STM, or a test backend. Either
//!   `DoesNotExist` or `ExternalHeap`, where the latter points into an arena
//!   owned by the provider, not by this transaction.
//! - `write` ([`StorageWrite`]) — the pending modification, if any:
//!   `NotModified`, `Deleted` (moved out), or `LocalHeap` (the value now lives
//!   in this transaction's heap and may have been mutated).
//!
//! # Copy-on-write
//!
//! An immutable borrow can hand out the `ExternalHeap` pointer directly, with
//! no copy. A mutable borrow needs a value that is both owned by this
//! transaction's heap and current (see below), so the first mutable borrow of
//! an external or stale value deep-copies it into the local heap and records a
//! `LocalHeap` write. Later mutable borrows in the same checkpoint reuse that
//! copy.
//!
//! # Checkpoints and the checkpoint counter
//!
//! Rather than snapshotting all state at each checkpoint, the set tags every
//! `LocalHeap`/`Deleted` write with the [`CheckpointCounter`] value current
//! when it was made. A new checkpoint just bumps the counter. A write tagged
//! with an older counter belongs to an earlier checkpoint's snapshot, so it
//! must be copied before being mutated again — otherwise rolling back to that
//! checkpoint would observe the later mutation. This is what makes a write
//! "current" (directly writable) versus "stale" (copy-on-write again).
//!
//! # Rollback journal
//!
//! Whenever a write overwrites a write from an older counter, the old write is
//! pushed to the undo `journal`. A [`Checkpoint`] records the journal length at
//! the time it was taken. `rollback(n)` replays journal entries backwards down
//! to the recorded length, restoring each resource's prior write. Allocations
//! that become unreachable are reclaimed by the next GC, not at rollback time.
//!
//! # GC interaction
//!
//! `LocalHeap` pointers are live roots: the GC scans and relocates them via
//! [`RootScanner`]. `ExternalHeap` pointers are owned by the provider and are
//! never relocated.

use crate::{
    error::{GlobalStorageOp, RuntimeError, RuntimeResult},
    heap::RootScanner,
    invariant_violation,
};
use hashbrown::{hash_map::EntryRef, HashMap};
use mono_move_core::{
    storage::resource_provider::InMemoryStorageKey, ResourceProvider, StorageRead,
};
use std::ptr::NonNull;

/// Counter incremented by every checkpoint. Used to tell whether a pending
/// write was made in the current checkpoint (and so is directly writable) or
/// in an older one (and so must be copied before mutation).
pub type CheckpointCounter = u64;

/// Represents a pending write to storage.
#[derive(Clone, Copy)]
pub enum StorageWrite {
    /// There is no write to this resource yet.
    NotModified,
    /// This resource has been moved out at the specified epoch.
    Deleted { epoch: CheckpointCounter },
    /// This resource has been copied into local transaction heap at the
    /// specified epoch. It may or may not be modified.
    LocalHeap {
        ptr: NonNull<u8>,
        epoch: CheckpointCounter,
    },
}

impl StorageWrite {
    /// Returns true if there is a possible modification made in the current
    /// epoch.
    fn is_at_epoch(&self, current_epoch: CheckpointCounter) -> bool {
        match self {
            StorageWrite::NotModified => false,
            StorageWrite::LocalHeap { epoch, .. } | StorageWrite::Deleted { epoch } => {
                *epoch == current_epoch
            },
        }
    }
}

/// An entry in the read-write set. Records a read of the value (before the
/// start of the transaction) and its modification (if any).
#[derive(Clone)]
pub struct Entry {
    pub read: StorageRead,
    pub write: StorageWrite,
}

/// Represents the state of data pointer in the map: owned by local allocation
/// with up-to-date epoch - writable and can be directly used for mutation or
/// needing a copy (if it is read-only or has an outdated epoch).
pub(crate) enum EntryPtr {
    /// This pointer lives in the local transaction heap and has the same epoch
    /// as the current epoch. Can be safely used for mutations.
    Writable(NonNull<u8>),
    /// This pointer is not in the local heap yet (read-only) or its epoch is
    /// below the current epoch. For mutations, this pointer needs to be
    /// copied.
    NonWritable(NonNull<u8>),
}

impl Entry {
    /// Returns true if the resource exists at the given key. Returns false if
    /// it does not or if it has been deleted.
    pub(crate) fn exists(&self) -> bool {
        match self.write {
            StorageWrite::NotModified => match self.read {
                StorageRead::DoesNotExist => false,
                StorageRead::ExternalHeap { .. } => true,
            },
            StorageWrite::Deleted { .. } => false,
            StorageWrite::LocalHeap { .. } => true,
        }
    }

    /// Returns the pointer to the global value (whether local write or
    /// external read). Returns [`None`] if the resource does not exist
    /// (it was deleted or never existed).
    pub(crate) fn as_ptr(&self) -> Option<NonNull<u8>> {
        match self.write {
            StorageWrite::NotModified => match self.read {
                StorageRead::DoesNotExist => None,
                StorageRead::ExternalHeap { ptr, .. } => Some(ptr),
            },
            StorageWrite::Deleted { .. } => None,
            StorageWrite::LocalHeap { ptr, .. } => Some(ptr),
        }
    }

    /// Returns the pointer to the global value. Returns [`EntryPtr::Writable`]
    /// with the pointer value that can be safely mutated and is owned by local
    /// transaction heap and that was created in the current epoch. Returns
    /// [`EntryPtr::NonWritable`] if the pointer is not owned or has been
    /// created in earlier epoch: in this case value requires a deep copy.
    /// Returns [`None`] if resource does not exist or was deleted.
    pub(crate) fn as_ptr_mut(&self, current_epoch: CheckpointCounter) -> Option<EntryPtr> {
        match self.write {
            StorageWrite::NotModified => match self.read {
                StorageRead::DoesNotExist => None,
                StorageRead::ExternalHeap { ptr, .. } => Some(EntryPtr::NonWritable(ptr)),
            },
            StorageWrite::Deleted { .. } => None,
            StorageWrite::LocalHeap { ptr, epoch } => {
                if epoch == current_epoch {
                    Some(EntryPtr::Writable(ptr))
                } else {
                    Some(EntryPtr::NonWritable(ptr))
                }
            },
        }
    }
}

/// An entry in the undo journal that records old state of the resource.
struct JournalEntry {
    key: InMemoryStorageKey,
    write: StorageWrite,
}

/// A single checkpoint that snapshots the length of the undo journal at a
/// particular epoch. In order to roll back to some checkpoint it is enough
/// to re-apply writes from the log backwards until the chosen length.
#[derive(Clone, Copy)]
struct Checkpoint {
    journal_len: usize,
    epoch: CheckpointCounter,
}

/// Per-transaction global storage state.
#[derive(Default)]
pub struct ResourceReadWriteSet {
    /// Reads and writes to the global storage.
    ///
    /// This is a hash map, so iteration order is non-deterministic. Every place
    /// that iterates `entries` must not depend on that order:
    ///   - read-set validation (Block-STM) only checks each entry, so order
    ///     does not matter;
    ///   - aggregation (such as gas) must use a commutative combine so the
    ///     result is order-independent;
    ///   - producing the final write set must sort the entries into a
    ///     deterministic order before emitting them.
    // TODO(correctness):
    //   Make sure we have a deterministic iteration API and write-set
    //   generation.
    entries: HashMap<InMemoryStorageKey, Entry>,
    /// Undo log of writes originating from older epochs. Used for rolling back
    /// to older checkpoints.
    journal: Vec<JournalEntry>,
    /// A list of saved checkpoints.
    checkpoints: Vec<Checkpoint>,
    /// Current epoch, incremented on every new checkpoint.
    current_epoch: CheckpointCounter,
}

impl ResourceReadWriteSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the resource exists at the specified key.
    pub(crate) fn exists(
        &mut self,
        provider: &dyn ResourceProvider,
        key: &InMemoryStorageKey,
    ) -> RuntimeResult<bool> {
        Ok(get_or_create_resource_entry(&mut self.entries, provider, key)?.exists())
    }

    /// Returns the pointer to the resource. Returns an error if the resource
    /// does not exist or was deleted.
    pub(crate) fn borrow_global(
        &mut self,
        provider: &dyn ResourceProvider,
        key: &InMemoryStorageKey,
    ) -> RuntimeResult<NonNull<u8>> {
        get_or_create_resource_entry(&mut self.entries, provider, key)?
            .as_ptr()
            .ok_or_else(|| RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::BorrowGlobal,
                addr: key.address(),
            })
    }

    /// First step when mutably borrowing a resource. Returns an error if the
    /// resource does not exist or was deleted.
    ///
    /// Returns the pointer to the data that currently exists, which may or may
    /// not be directly writable. If writable, caller can use it as is. If not
    /// writable, caller has to perform a **copy** of the value and update the
    /// map with the copied pointer via [`Self::commit_borrow_global_mut`].
    pub(crate) fn try_borrow_global_mut(
        &mut self,
        provider: &dyn ResourceProvider,
        key: &InMemoryStorageKey,
    ) -> RuntimeResult<EntryPtr> {
        get_or_create_resource_entry(&mut self.entries, provider, key)?
            .as_ptr_mut(self.current_epoch)
            .ok_or_else(|| RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::BorrowGlobalMut,
                addr: key.address(),
            })
    }

    /// Second step when mutably borrowing a resource. Only used for pointers
    /// that are not writable (i.e, not owned by the current transaction's heap
    /// or originating from older epoch). The caller must have performed a deep
    /// copy of data, allocating it in the local heap, and passed that pointer
    /// back to the map. Records the older write in the journal for reverts.
    ///
    /// ## Panics
    ///
    /// [`Self::try_borrow_global_mut`] ensures that entry exists in the map.
    /// If this condition is violated, panics.
    pub(crate) fn commit_borrow_global_mut(&mut self, key: &InMemoryStorageKey, ptr: NonNull<u8>) {
        let entry = self
            .entries
            .get_mut(key)
            .expect("Entry must exist after mutable borrow attempt");
        let old_write = std::mem::replace(&mut entry.write, StorageWrite::LocalHeap {
            ptr,
            epoch: self.current_epoch,
        });
        self.record_write_to_journal(key, old_write);
    }

    /// Records a write of a resource at the given key. Returns an error if
    /// the resource already exists.
    ///
    /// ## Invariants
    ///
    /// - The pointer is owned by this transaction's heap.
    /// - The read is also recorded for the entry (with is previous state).
    pub(crate) fn move_to(
        &mut self,
        provider: &dyn ResourceProvider,
        key: &InMemoryStorageKey,
        ptr: NonNull<u8>,
    ) -> RuntimeResult<()> {
        let entry = get_or_create_resource_entry(&mut self.entries, provider, key)?;
        if entry.exists() {
            return Err(RuntimeError::ResourceAlreadyExists {
                addr: key.address(),
            });
        }
        let old_write = std::mem::replace(&mut entry.write, StorageWrite::LocalHeap {
            ptr,
            epoch: self.current_epoch,
        });
        self.record_write_to_journal(key, old_write);
        Ok(())
    }

    /// First step when moving the resource out of storage. Returns an error if
    /// the resource does not exist or was already deleted.
    ///
    /// Returns the pointer to the data that currently exists, which may or may
    /// not be directly writable. If writable, caller can use it as is. If not
    /// writable, caller has to perform a **copy** of the value and update the
    /// map via [`Self::commit_move_from`].
    pub(crate) fn try_move_from(
        &mut self,
        provider: &dyn ResourceProvider,
        key: &InMemoryStorageKey,
    ) -> RuntimeResult<EntryPtr> {
        let entry = get_or_create_resource_entry(&mut self.entries, provider, key)?;
        let ptr = entry.as_ptr_mut(self.current_epoch).ok_or_else(|| {
            RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::MoveFrom,
                addr: key.address(),
            }
        })?;

        // If writable, it is same epoch. So no need to record anything in the
        // journal.
        if matches!(ptr, EntryPtr::Writable(..)) {
            entry.write = StorageWrite::Deleted {
                epoch: self.current_epoch,
            };
        }
        Ok(ptr)
    }

    /// Second step when moving the resource out of global storage. Only used
    /// for pointers that are not writable (i.e, not owned by the current
    /// transaction's heap or originating from older epoch). The caller must
    /// have performed a deep copy of data, allocating it in the local heap.
    /// Records the older write in the journal for reverts.
    ///
    /// ## Panics
    ///
    /// [`Self::try_move_from`] ensures that entry exists in the map.
    /// If this condition is violated, panics.
    pub(crate) fn commit_move_from(&mut self, key: &InMemoryStorageKey) {
        let entry = self
            .entries
            .get_mut(key)
            .expect("Entry must exist after move_from attempt");
        let old_write = std::mem::replace(&mut entry.write, StorageWrite::Deleted {
            epoch: self.current_epoch,
        });
        self.record_write_to_journal(key, old_write);
    }

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> CheckpointCounter {
        self.current_epoch
    }

    /// Returns the number of saved checkpoints.
    pub fn checkpoint_depth(&self) -> usize {
        self.checkpoints.len()
    }

    /// Returns the number of entries in the journal (undo log).
    pub fn journal_len(&self) -> usize {
        self.journal.len()
    }

    /// Save the current state and advance the epoch. A subsequent roll back
    /// can return here.
    pub fn checkpoint(&mut self) {
        self.checkpoints.push(Checkpoint {
            journal_len: self.journal.len(),
            epoch: self.current_epoch,
        });
        self.current_epoch = self
            .current_epoch
            .checked_add(1)
            .expect("Checkpoint counter must never overflow");
    }

    /// Undoes the given number of checkpoints.
    ///
    /// Note: allocations that became unreachable are eventually reclaimed by
    /// the next GC.
    pub fn rollback(&mut self, n: usize) -> RuntimeResult<()> {
        if n == 0 {
            return Ok(());
        }
        if n > self.checkpoints.len() {
            invariant_violation!(RollbackUnderflow {
                requested: n,
                available: self.checkpoints.len(),
            });
        }

        let target = self
            .checkpoints
            .drain(self.checkpoints.len() - n..)
            .next()
            .expect("There must be at least one checkpoint");
        self.current_epoch = target.epoch;

        // Walk the journal top-down to the saved length, applying each entry
        // from the undo log.
        for restored_entry in self.journal.drain(target.journal_len..).rev() {
            let curr_entry = self
                .entries
                .get_mut(&restored_entry.key)
                .expect("Journal entry always has a corresponding read/write-set entry");
            curr_entry.write = restored_entry.write;
        }
        Ok(())
    }

    /// Relocate every working-map / journal `LocalHeap` pointer via
    /// the GC's [`RootScanner`]. Called from `gc_collect` alongside
    /// the call-stack and pinned-root scans.
    pub(crate) fn scan(&mut self, scanner: &mut RootScanner<'_>) {
        for write in self
            .entries
            .values_mut()
            .map(|e| &mut e.write)
            .chain(self.journal.iter_mut().map(|e| &mut e.write))
        {
            if let StorageWrite::LocalHeap { ptr, .. } = write {
                // `LocalHeap` pointers always live in the local heap
                // by construction, so `relocate` returns `Some` here.
                if let Some(relocated) = scanner.relocate(ptr.as_ptr()) {
                    // SAFETY: `gc_copy_object` never produces a null
                    // pointer.
                    *ptr = unsafe { NonNull::new_unchecked(relocated) };
                }
            }
        }
    }

    /// Records old write in the journal (so the new write can be reverted to
    /// the old one) if:
    ///   - previous write does not exist, or
    ///   - previous write was made in a different (older) epoch.
    fn record_write_to_journal(&mut self, key: &InMemoryStorageKey, write: StorageWrite) {
        if !write.is_at_epoch(self.current_epoch) {
            self.journal.push(JournalEntry {
                key: key.clone(),
                write,
            });
        }
    }
}

/// Looks up the resource entry, materializing it as a read and recording in
/// the read-set.
fn get_or_create_resource_entry<'a>(
    entries: &'a mut HashMap<InMemoryStorageKey, Entry>,
    provider: &dyn ResourceProvider,
    key: &InMemoryStorageKey,
) -> RuntimeResult<&'a mut Entry> {
    match entries.entry_ref(key) {
        EntryRef::Occupied(entry) => Ok(entry.into_mut()),
        EntryRef::Vacant(entry) => {
            let read = provider.get_resource(key)?;
            Ok(entry.insert(Entry {
                read,
                write: StorageWrite::NotModified,
            }))
        },
    }
}
