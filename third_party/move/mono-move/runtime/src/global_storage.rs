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
    error::{GlobalStorageOp, RuntimeError},
    heap::RootScanner,
    invariant_violation,
};
use hashbrown::{hash_map::EntryRef, HashMap};
use mono_move_core::{
    storage::resource_provider::InMemoryStorageKey, ResourceProvider, StorageRead, VMResult,
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

/// The write an [`Entry`] contributes to the write set, by its `(read, write)`
/// pair.
pub(crate) enum WriteClass {
    /// Did not exist before, now does. Carries the value pointer to serialize.
    Creation(NonNull<u8>),
    /// Existed before, now (possibly) modified — any mutable borrow counts,
    /// even if the value is unchanged. Carries the value pointer to serialize.
    Modification(NonNull<u8>),
    /// Existed before, now moved out.
    Deletion,
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

    /// Classifies this entry's `(read, write)` into a [`WriteClass`], or
    /// [`None`] if it is not a write.
    pub(crate) fn write_class(&self) -> Option<WriteClass> {
        match self.write {
            StorageWrite::NotModified => None,
            StorageWrite::LocalHeap { ptr, .. } => Some(match self.read {
                StorageRead::DoesNotExist => WriteClass::Creation(ptr),
                // TODO(correctness, perf): over-approximation — a `borrow_global_mut` copies to the
                // local heap even if nothing changed. Compare against the read value to drop no-op
                // modifications here rather than downstream.
                StorageRead::ExternalHeap { .. } => WriteClass::Modification(ptr),
            }),
            StorageWrite::Deleted { .. } => match self.read {
                // Published and then removed in the same transaction: the
                // on-chain slot never existed, so this is not a write.
                StorageRead::DoesNotExist => None,
                StorageRead::ExternalHeap { .. } => Some(WriteClass::Deletion),
            },
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
    ) -> Result<bool, RuntimeError> {
        Ok(get_or_create_resource_entry(&mut self.entries, provider, key)?.exists())
    }

    /// Returns the pointer to the resource. Returns an error if the resource
    /// does not exist or was deleted.
    pub(crate) fn borrow_global(
        &mut self,
        provider: &dyn ResourceProvider,
        key: &InMemoryStorageKey,
    ) -> Result<NonNull<u8>, RuntimeError> {
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
    ) -> Result<EntryPtr, RuntimeError> {
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
    ) -> Result<(), RuntimeError> {
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
    ) -> Result<EntryPtr, RuntimeError> {
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

    /// Yields each entry that is a write, with its [`WriteClass`]. Order is
    /// unspecified (backing hash map); callers must sort before emitting.
    pub(crate) fn writes(&self) -> impl Iterator<Item = (&InMemoryStorageKey, WriteClass)> {
        self.entries
            .iter()
            .filter_map(|(key, entry)| entry.write_class().map(|class| (key, class)))
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
    pub fn rollback(&mut self, n: usize) -> VMResult<()> {
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
) -> Result<&'a mut Entry, RuntimeError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{heap::Heap, write_object_header};
    use mono_move_alloc::GlobalArenaPtr;
    use mono_move_core::{
        storage::resource_provider::ResourceProviderError, types::Type, DescriptorId,
        OBJECT_HEADER_SIZE,
    };
    use move_core_types::account_address::AccountAddress;

    // An `InternedType` is just an arena pointer; a `'static` node gives a
    // stable, cheap one without standing up an interner. Two distinct types let
    // us check that keys discriminate on type, not only address.
    static TY_A: Type = Type::U64;
    static TY_B: Type = Type::Bool;

    fn addr(n: u8) -> AccountAddress {
        let mut bytes = [0u8; AccountAddress::LENGTH];
        bytes[AccountAddress::LENGTH - 1] = n;
        AccountAddress::new(bytes)
    }

    fn key_a(n: u8) -> InMemoryStorageKey {
        InMemoryStorageKey::Resource {
            address: addr(n),
            ty: GlobalArenaPtr::from_static(&TY_A),
        }
    }

    fn key_b(n: u8) -> InMemoryStorageKey {
        InMemoryStorageKey::Resource {
            address: addr(n),
            ty: GlobalArenaPtr::from_static(&TY_B),
        }
    }

    // The map only stores and compares these pointers — it never reads through
    // them (copying a value is the interpreter's job). So any non-null address
    // is a valid stand-in for a heap value.
    fn fake_ptr(n: usize) -> NonNull<u8> {
        NonNull::new(n as *mut u8).expect("non-null")
    }

    /// Minimal in-crate provider: keys present here are external (committed)
    /// resources; everything else does not exist.
    #[derive(Default)]
    struct Provider {
        external: HashMap<InMemoryStorageKey, NonNull<u8>>,
    }

    impl Provider {
        fn empty() -> Self {
            Self::default()
        }

        fn with(key: &InMemoryStorageKey, ptr: NonNull<u8>) -> Self {
            let mut external = HashMap::new();
            external.insert(key.clone(), ptr);
            Self { external }
        }
    }

    impl ResourceProvider for Provider {
        fn get_resource(
            &self,
            key: &InMemoryStorageKey,
        ) -> Result<StorageRead, ResourceProviderError> {
            Ok(match self.external.get(key) {
                Some(&ptr) => StorageRead::ExternalHeap { ptr, version: 0 },
                None => StorageRead::DoesNotExist,
            })
        }
    }

    fn entry(read: StorageRead, write: StorageWrite) -> Entry {
        Entry { read, write }
    }

    // -- Entry state machine --------------------------------------------------

    #[test]
    fn exists_reflects_read_and_write_state() {
        let p = fake_ptr(0x10);
        // An unmodified entry follows its read.
        assert!(!entry(StorageRead::DoesNotExist, StorageWrite::NotModified).exists());
        assert!(entry(
            StorageRead::ExternalHeap { ptr: p, version: 0 },
            StorageWrite::NotModified
        )
        .exists());
        // A local write means present, regardless of the read.
        assert!(entry(StorageRead::DoesNotExist, StorageWrite::LocalHeap {
            ptr: p,
            epoch: 0
        })
        .exists());
        // Deletion shadows any read.
        assert!(!entry(
            StorageRead::ExternalHeap { ptr: p, version: 0 },
            StorageWrite::Deleted { epoch: 0 }
        )
        .exists());
    }

    #[test]
    fn as_ptr_is_zero_copy_for_external_and_none_when_absent() {
        let ext = fake_ptr(0x20);
        let local = fake_ptr(0x21);
        // An external read hands back the provider's pointer unchanged — the
        // zero-copy read path.
        assert_eq!(
            entry(
                StorageRead::ExternalHeap {
                    ptr: ext,
                    version: 0
                },
                StorageWrite::NotModified
            )
            .as_ptr(),
            Some(ext)
        );
        assert_eq!(
            entry(StorageRead::DoesNotExist, StorageWrite::LocalHeap {
                ptr: local,
                epoch: 0
            })
            .as_ptr(),
            Some(local)
        );
        assert_eq!(
            entry(StorageRead::DoesNotExist, StorageWrite::NotModified).as_ptr(),
            None
        );
        assert_eq!(
            entry(
                StorageRead::ExternalHeap {
                    ptr: ext,
                    version: 0
                },
                StorageWrite::Deleted { epoch: 0 }
            )
            .as_ptr(),
            None
        );
    }

    #[test]
    fn as_ptr_mut_writability_tracks_ownership_and_epoch() {
        let ext = fake_ptr(0x30);
        let local = fake_ptr(0x31);
        // An external read is never directly writable — it needs a copy.
        assert!(matches!(
            entry(
                StorageRead::ExternalHeap { ptr: ext, version: 0 },
                StorageWrite::NotModified
            )
            .as_ptr_mut(0),
            Some(EntryPtr::NonWritable(p)) if p == ext
        ));
        // A local write in the current epoch is writable in place.
        assert!(matches!(
            entry(
                StorageRead::DoesNotExist,
                StorageWrite::LocalHeap { ptr: local, epoch: 5 }
            )
            .as_ptr_mut(5),
            Some(EntryPtr::Writable(p)) if p == local
        ));
        // A local write from an older epoch needs a copy.
        assert!(matches!(
            entry(
                StorageRead::DoesNotExist,
                StorageWrite::LocalHeap { ptr: local, epoch: 4 }
            )
            .as_ptr_mut(5),
            Some(EntryPtr::NonWritable(p)) if p == local
        ));
        // Deleted / absent have no pointer.
        assert!(entry(StorageRead::DoesNotExist, StorageWrite::Deleted {
            epoch: 0
        })
        .as_ptr_mut(0)
        .is_none());
        assert!(entry(StorageRead::DoesNotExist, StorageWrite::NotModified)
            .as_ptr_mut(0)
            .is_none());
    }

    // -- Map operations over the provider ------------------------------------

    #[test]
    fn exists_over_provider() {
        let k = key_a(1);
        assert!(!ResourceReadWriteSet::new()
            .exists(&Provider::empty(), &k)
            .unwrap());
        assert!(ResourceReadWriteSet::new()
            .exists(&Provider::with(&k, fake_ptr(0x40)), &k)
            .unwrap());
    }

    #[test]
    fn keys_discriminate_on_type_at_the_same_address() {
        // Same address, different resource type — distinct keys.
        let ka = key_a(1);
        let kb = key_b(1);
        let provider = Provider::with(&ka, fake_ptr(0x41));
        let mut rws = ResourceReadWriteSet::new();
        assert!(rws.exists(&provider, &ka).unwrap());
        assert!(!rws.exists(&provider, &kb).unwrap());
    }

    #[test]
    fn borrow_global_external_is_zero_copy() {
        let k = key_a(1);
        let ptr = fake_ptr(0x42);
        // The borrow returns the provider's pointer directly — no local copy.
        assert_eq!(
            ResourceReadWriteSet::new()
                .borrow_global(&Provider::with(&k, ptr), &k)
                .unwrap(),
            ptr
        );
    }

    #[test]
    fn borrow_global_missing_aborts() {
        let k = key_a(1);
        assert!(matches!(
            ResourceReadWriteSet::new().borrow_global(&Provider::empty(), &k),
            Err(RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::BorrowGlobal,
                ..
            })
        ));
    }

    #[test]
    fn try_borrow_global_mut_over_each_state() {
        let k = key_a(1);
        // Missing -> abort.
        assert!(matches!(
            ResourceReadWriteSet::new().try_borrow_global_mut(&Provider::empty(), &k),
            Err(RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::BorrowGlobalMut,
                ..
            })
        ));
        // External -> non-writable (copy required).
        let ext = fake_ptr(0x50);
        assert!(matches!(
            ResourceReadWriteSet::new().try_borrow_global_mut(&Provider::with(&k, ext), &k),
            Ok(EntryPtr::NonWritable(p)) if p == ext
        ));
        // Local same-epoch -> writable in place.
        let local = fake_ptr(0x51);
        let mut rws = ResourceReadWriteSet::new();
        rws.move_to(&Provider::empty(), &k, local).unwrap();
        assert!(matches!(
            rws.try_borrow_global_mut(&Provider::empty(), &k),
            Ok(EntryPtr::Writable(p)) if p == local
        ));
    }

    #[test]
    fn borrow_global_mut_copies_external_once_then_reuses_in_epoch() {
        // The first mutable borrow of an external resource is non-writable (the
        // caller copies into the local heap and commits). A second borrow in the
        // same epoch is writable and reuses that copy — no second copy.
        let k = key_a(1);
        let ext = fake_ptr(0xE0);
        let provider = Provider::with(&k, ext);
        let mut rws = ResourceReadWriteSet::new();

        assert!(matches!(
            rws.try_borrow_global_mut(&provider, &k),
            Ok(EntryPtr::NonWritable(p)) if p == ext
        ));
        let local = fake_ptr(0xE1);
        rws.commit_borrow_global_mut(&k, local);
        assert!(matches!(
            rws.try_borrow_global_mut(&provider, &k),
            Ok(EntryPtr::Writable(p)) if p == local
        ));
    }

    #[test]
    fn move_to_publishes_and_rejects_duplicates() {
        let k = key_a(1);
        let mut rws = ResourceReadWriteSet::new();
        rws.move_to(&Provider::empty(), &k, fake_ptr(0x60)).unwrap();
        assert!(rws.exists(&Provider::empty(), &k).unwrap());
        // Re-publishing over the just-written local resource aborts.
        assert!(matches!(
            rws.move_to(&Provider::empty(), &k, fake_ptr(0x61)),
            Err(RuntimeError::ResourceAlreadyExists { .. })
        ));
        // Publishing over a committed/external resource aborts too — the
        // ExternalHeap-already-exists branch the differential harness can't
        // reach (no committed state across calls).
        assert!(matches!(
            ResourceReadWriteSet::new().move_to(
                &Provider::with(&k, fake_ptr(0x62)),
                &k,
                fake_ptr(0x63)
            ),
            Err(RuntimeError::ResourceAlreadyExists { .. })
        ));
    }

    #[test]
    fn move_from_external_requires_copy_then_marks_deleted() {
        let k = key_a(1);
        let ext = fake_ptr(0x70);
        let provider = Provider::with(&k, ext);
        let mut rws = ResourceReadWriteSet::new();
        // External is non-writable: the caller must deep-copy.
        assert!(matches!(
            rws.try_move_from(&provider, &k),
            Ok(EntryPtr::NonWritable(p)) if p == ext
        ));
        // The move is not finalized until commit — the resource still exists.
        assert!(rws.exists(&provider, &k).unwrap());
        rws.commit_move_from(&k);
        // Now it is gone: exists is false and a borrow aborts.
        assert!(!rws.exists(&provider, &k).unwrap());
        assert!(matches!(
            rws.borrow_global(&provider, &k),
            Err(RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::BorrowGlobal,
                ..
            })
        ));
    }

    #[test]
    fn move_from_local_is_writable_and_deletes_in_place() {
        let k = key_a(1);
        let local = fake_ptr(0x71);
        let mut rws = ResourceReadWriteSet::new();
        rws.move_to(&Provider::empty(), &k, local).unwrap();
        // A same-epoch local value is writable; try_move_from deletes it right
        // away, no commit needed.
        assert!(matches!(
            rws.try_move_from(&Provider::empty(), &k),
            Ok(EntryPtr::Writable(p)) if p == local
        ));
        assert!(!rws.exists(&Provider::empty(), &k).unwrap());
    }

    #[test]
    fn move_from_missing_aborts() {
        let k = key_a(1);
        assert!(matches!(
            ResourceReadWriteSet::new().try_move_from(&Provider::empty(), &k),
            Err(RuntimeError::ResourceDoesNotExist {
                op: GlobalStorageOp::MoveFrom,
                ..
            })
        ));
    }

    // -- Checkpoints / rollback ----------------------------------------------

    #[test]
    fn checkpoint_advances_epoch_and_depth() {
        let mut rws = ResourceReadWriteSet::new();
        assert_eq!((rws.current_epoch(), rws.checkpoint_depth()), (0, 0));
        rws.checkpoint();
        assert_eq!((rws.current_epoch(), rws.checkpoint_depth()), (1, 1));
    }

    #[test]
    fn rollback_zero_is_noop() {
        let mut rws = ResourceReadWriteSet::new();
        rws.checkpoint();
        rws.rollback(0).unwrap();
        assert_eq!((rws.current_epoch(), rws.checkpoint_depth()), (1, 1));
    }

    #[test]
    fn rollback_more_than_depth_aborts() {
        let mut rws = ResourceReadWriteSet::new();
        rws.checkpoint();
        assert!(rws.rollback(2).is_err());
    }

    #[test]
    fn rollback_undoes_a_publish_made_after_the_checkpoint() {
        let k = key_a(1);
        let mut rws = ResourceReadWriteSet::new();
        rws.checkpoint();
        rws.move_to(&Provider::empty(), &k, fake_ptr(0x80)).unwrap();
        assert!(rws.exists(&Provider::empty(), &k).unwrap());
        rws.rollback(1).unwrap();
        assert!(!rws.exists(&Provider::empty(), &k).unwrap());
    }

    #[test]
    fn rollback_restores_to_deleted_not_absent() {
        // Design-doc case: an external resource is moved out, then after a
        // checkpoint re-published. Rolling back must restore the Deleted write,
        // not the original present state.
        let k = key_a(1);
        let provider = Provider::with(&k, fake_ptr(0x90));
        let mut rws = ResourceReadWriteSet::new();

        let _ = rws.try_move_from(&provider, &k).unwrap();
        rws.commit_move_from(&k);
        assert!(!rws.exists(&provider, &k).unwrap());

        rws.checkpoint();
        rws.move_to(&provider, &k, fake_ptr(0x91)).unwrap();
        assert!(rws.exists(&provider, &k).unwrap());

        rws.rollback(1).unwrap();
        assert!(!rws.exists(&provider, &k).unwrap());
    }

    #[test]
    fn journal_grows_once_per_epoch_for_repeated_mutations() {
        let k = key_a(1);
        let provider = Provider::with(&k, fake_ptr(0xA0));
        let mut rws = ResourceReadWriteSet::new();

        // First mutation of an external resource records its pre-txn state.
        let _ = rws.try_move_from(&provider, &k).unwrap();
        rws.commit_move_from(&k);
        assert_eq!(rws.journal_len(), 1);

        // A second mutation in the same epoch does not grow the journal — the
        // pre-epoch state is already saved.
        rws.move_to(&provider, &k, fake_ptr(0xA1)).unwrap();
        assert_eq!(rws.journal_len(), 1);
    }

    #[test]
    fn rollback_n_collapses_multiple_checkpoints() {
        let k = key_a(1);
        let mut rws = ResourceReadWriteSet::new();
        rws.checkpoint();
        rws.checkpoint();
        rws.move_to(&Provider::empty(), &k, fake_ptr(0xB0)).unwrap();
        assert_eq!(rws.checkpoint_depth(), 2);
        rws.rollback(2).unwrap();
        assert_eq!((rws.current_epoch(), rws.checkpoint_depth()), (0, 0));
        assert!(!rws.exists(&Provider::empty(), &k).unwrap());
    }

    #[test]
    fn journal_grows_for_first_mutation_each_epoch() {
        let k = key_a(1);
        let provider = Provider::with(&k, fake_ptr(0xC0));
        let mut rws = ResourceReadWriteSet::new();

        // First mutable borrow of the external resource: copy + commit records
        // the pre-txn state.
        let _ = rws.try_borrow_global_mut(&provider, &k).unwrap();
        rws.commit_borrow_global_mut(&k, fake_ptr(0xC1));
        assert_eq!(rws.journal_len(), 1);

        // The first mutation in a new epoch journals again — the local write is
        // now from an older epoch.
        rws.checkpoint();
        let _ = rws.try_borrow_global_mut(&provider, &k).unwrap();
        rws.commit_borrow_global_mut(&k, fake_ptr(0xC2));
        assert_eq!(rws.journal_len(), 2);
    }

    #[test]
    fn rollback_then_remutate_journals_again() {
        let k = key_a(1);
        let provider = Provider::with(&k, fake_ptr(0xD0));
        let mut rws = ResourceReadWriteSet::new();

        rws.checkpoint();
        let _ = rws.try_borrow_global_mut(&provider, &k).unwrap();
        rws.commit_borrow_global_mut(&k, fake_ptr(0xD1));
        assert_eq!(rws.journal_len(), 1);

        rws.rollback(1).unwrap();
        assert_eq!(rws.journal_len(), 0);

        // After rollback the entry is back to its pre-mutation state, so the
        // next mutable borrow journals again.
        let _ = rws.try_borrow_global_mut(&provider, &k).unwrap();
        rws.commit_borrow_global_mut(&k, fake_ptr(0xD2));
        assert_eq!(rws.journal_len(), 1);
    }

    // -- Write classification -------------------------------------------------

    fn class_of(read: StorageRead, write: StorageWrite) -> Option<WriteClass> {
        entry(read, write).write_class()
    }

    #[test]
    fn write_class_covers_the_read_write_matrix() {
        let ext = || StorageRead::ExternalHeap {
            ptr: fake_ptr(0x10),
            version: 0,
        };
        let local = || StorageWrite::LocalHeap {
            ptr: fake_ptr(0x11),
            epoch: 0,
        };
        let deleted = || StorageWrite::Deleted { epoch: 0 };

        // Untouched / read-only: no write.
        assert!(class_of(StorageRead::DoesNotExist, StorageWrite::NotModified).is_none());
        assert!(class_of(ext(), StorageWrite::NotModified).is_none());
        // Published this txn -> Creation.
        assert!(matches!(
            class_of(StorageRead::DoesNotExist, local()),
            Some(WriteClass::Creation(_))
        ));
        // Pre-existing + local write -> Modification.
        assert!(matches!(
            class_of(ext(), local()),
            Some(WriteClass::Modification(_))
        ));
        // Pre-existing + deleted -> Deletion.
        assert!(matches!(
            class_of(ext(), deleted()),
            Some(WriteClass::Deletion)
        ));
        // Published then removed within the txn: net no-op, not a write.
        assert!(class_of(StorageRead::DoesNotExist, deleted()).is_none());
    }

    // -- GC scan --------------------------------------------------------------

    #[test]
    fn scan_relocates_local_writes_but_not_external_reads() {
        // A from-space heap holding one object whose payload is a u64.
        let mut heap = Heap::new(4096);
        let payload: u64 = 0xABCD;
        // SAFETY: write a valid `[header | payload]` block at the bump pointer
        // and advance it, mirroring the allocator's layout.
        let local = unsafe {
            let header_start = heap.bump_ptr;
            let obj = header_start.add(OBJECT_HEADER_SIZE);
            let total = OBJECT_HEADER_SIZE + 8;
            write_object_header(obj, DescriptorId(1), total as u32);
            (obj as *mut u64).write(payload);
            heap.bump_ptr = header_start.add(total);
            NonNull::new_unchecked(obj)
        };

        // To-space for the copy; kept alive for the duration of the scan.
        let mut to_space = vec![0u64; 512].into_boxed_slice();
        let mut scanner = RootScanner::for_test(&heap, to_space.as_mut_ptr() as *mut u8);

        // One committed (external) resource and one locally-written resource.
        let k_ext = key_a(1);
        let k_local = key_a(2);
        let ext = fake_ptr(0xDEAD);
        let mut rws = ResourceReadWriteSet::new();
        rws.entries.insert(k_ext.clone(), Entry {
            read: StorageRead::ExternalHeap {
                ptr: ext,
                version: 0,
            },
            write: StorageWrite::NotModified,
        });
        rws.entries.insert(k_local.clone(), Entry {
            read: StorageRead::DoesNotExist,
            write: StorageWrite::LocalHeap {
                ptr: local,
                epoch: 0,
            },
        });

        rws.scan(&mut scanner);

        // The external read is owned by the provider and must not be relocated.
        match rws.entries[&k_ext].read {
            StorageRead::ExternalHeap { ptr, .. } => assert_eq!(ptr, ext),
            StorageRead::DoesNotExist => panic!("external read must survive the scan"),
        }
        // The local write is relocated into to-space, payload preserved.
        match rws.entries[&k_local].write {
            StorageWrite::LocalHeap { ptr, .. } => {
                assert_ne!(ptr, local, "local write must be relocated");
                // SAFETY: `ptr` is the relocated object in to-space.
                assert_eq!(unsafe { (ptr.as_ptr() as *const u64).read() }, payload);
            },
            StorageWrite::NotModified | StorageWrite::Deleted { .. } => {
                panic!("local write must stay a LocalHeap write")
            },
        }
    }
}
