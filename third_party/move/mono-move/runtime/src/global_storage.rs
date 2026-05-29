// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Per-transaction global-storage state: read cache with pending writes and
//! rollback journal with the checkpoint stack.

use crate::{
    error::{GlobalStorageOp, RuntimeError, RuntimeResult},
    heap::RootScanner,
    invariant_violation,
};
use mono_move_core::{storage::resource_provider::StorageKey, ResourceProvider, StorageRead};
use std::{
    collections::{hash_map, HashMap},
    ptr::NonNull,
};

/// Global state can be saved during execution by recording checkpoints. Epoch
/// is a counter incremented by every checkpoint.
pub type Epoch = u64;

/// Represents a pending write to storage.
#[derive(Clone, Copy)]
pub enum StorageWrite {
    /// There is no write to this resource yet.
    NotModified,
    /// This resource has been moved out at the specified epoch.
    Deleted { epoch: Epoch },
    /// This resource has been copied into local transaction heap at the
    /// specified epoch. It may or may not be modified.
    LocalHeap { ptr: NonNull<u8>, epoch: Epoch },
}

impl StorageWrite {
    /// Returns true if there is a possible modification made in the current
    /// epoch.
    fn is_at_epoch(&self, current_epoch: Epoch) -> bool {
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
    pub(crate) fn as_ptr_mut(&self, current_epoch: Epoch) -> Option<EntryPtr> {
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
    key: StorageKey,
    write: StorageWrite,
}

/// A single checkpoint that snapshots the length of the undo journal at a
/// particular epoch. In order to roll back to some checkpoint it is enough
/// to re-apply writes from the log backwards until the chosen length.
#[derive(Clone, Copy)]
struct Checkpoint {
    journal_len: usize,
    epoch: Epoch,
}

/// Per-transaction global storage state.
#[derive(Default)]
pub struct ResourceReadWriteSet {
    /// Reads and writes to the global storage.
    entries: HashMap<StorageKey, Entry>,
    /// Undo log of writes originating from older epochs. Used for rolling back
    /// to older checkpoints.
    journal: Vec<JournalEntry>,
    /// A list of saved checkpoints.
    checkpoints: Vec<Checkpoint>,
    /// Current epoch, incremented on every new checkpoint.
    current_epoch: Epoch,
}

impl ResourceReadWriteSet {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true if the resource exists at the specified key.
    pub(crate) fn exists(
        &mut self,
        provider: &dyn ResourceProvider,
        key: StorageKey,
    ) -> RuntimeResult<bool> {
        Ok(get_or_create_resource_entry(&mut self.entries, provider, key)?.exists())
    }

    /// Returns the pointer to the resource. Returns an error if the resource
    /// does not exist or was deleted.
    pub(crate) fn borrow_global(
        &mut self,
        provider: &dyn ResourceProvider,
        key: StorageKey,
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
        key: StorageKey,
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
    pub(crate) fn commit_borrow_global_mut(&mut self, key: StorageKey, ptr: NonNull<u8>) {
        let entry = self
            .entries
            .get_mut(&key)
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
        key: StorageKey,
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
        key: StorageKey,
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
    pub(crate) fn commit_move_from(&mut self, key: StorageKey) {
        let entry = self
            .entries
            .get_mut(&key)
            .expect("Entry must exist after move_from attempt");
        let old_write = std::mem::replace(&mut entry.write, StorageWrite::Deleted {
            epoch: self.current_epoch,
        });
        self.record_write_to_journal(key, old_write);
    }

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> Epoch {
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
        // Note: this can never overflow in practice.
        self.current_epoch += 1;
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
    fn record_write_to_journal(&mut self, key: StorageKey, write: StorageWrite) {
        if !write.is_at_epoch(self.current_epoch) {
            self.journal.push(JournalEntry { key, write });
        }
    }
}

/// Looks up the resource entry, materializing it as a read and recording in
/// the read-set.
fn get_or_create_resource_entry<'a>(
    entries: &'a mut HashMap<StorageKey, Entry>,
    provider: &dyn ResourceProvider,
    key: StorageKey,
) -> RuntimeResult<&'a mut Entry> {
    match entries.entry(key) {
        hash_map::Entry::Occupied(entry) => Ok(entry.into_mut()),
        hash_map::Entry::Vacant(entry) => Ok(entry.insert(Entry {
            read: provider.get_resource(key)?,
            write: StorageWrite::NotModified,
        })),
    }
}
