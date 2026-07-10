// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource storage access for the runtime. Lives here (not in core) because
//! a read hands out a pointer into a [`Heap`] together with the `Arc` that
//! keeps that heap alive.

use crate::heap::Heap;
use mono_move_core::storage::resource_provider::{InMemoryStorageKey, ResourceProviderError};
use std::{fmt, ptr::NonNull, sync::Arc};

/// Version of a read value: either the committed (storage) state, or a
/// speculative write of another transaction in the same block. Mirrors
/// Block-STM's version without depending on its crates; the integration layer
/// converts between the two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Version {
    /// The value (or its absence) comes from committed storage.
    Storage,
    /// The value (or its deletion) was written by the transaction at
    /// `txn_idx`, by its `incarnation`-th execution.
    Write { txn_idx: u32, incarnation: u32 },
}

/// Storage read returned to the VM. Every VM execution records reads of any
/// value coming from global storage. Both variants carry the version the
/// read was observed at, so the read-write set doubles as the captured read
/// set for Block-STM validation. Non-existence needs a version too: a value
/// deleted by a speculative write must be re-validated against that exact
/// write, not against "still missing".
#[derive(Clone)]
pub enum StorageRead {
    /// Value does not exist at this key.
    DoesNotExist { version: Version },
    /// Value allocated in a heap owned by someone else — a deserialized
    /// storage value, or the write of a transaction at a lower version.
    ExternalHeap {
        /// Just like any other VM value, the pointer points to the start of
        /// the value allocation. Value's header is at negative offset.
        // TODO(cleanup): have a Value pointer unified API?
        ptr: NonNull<u8>,
        /// Version of this read from Block-STM. Used for read-set validation.
        version: Version,
        /// The heap `ptr` points into. Read-write-set entries store the read,
        /// so the pointer stays valid for as long as the entry exists — even
        /// if the owner (e.g. an aborted transaction's map entry) drops its
        /// copy concurrently.
        heap: Arc<Heap>,
    },
}

impl StorageRead {
    /// A non-existent value read from committed storage.
    pub fn does_not_exist_in_storage() -> Self {
        StorageRead::DoesNotExist {
            version: Version::Storage,
        }
    }

    /// A value read from committed storage, allocated in the given heap.
    pub fn external_heap_in_storage(ptr: NonNull<u8>, heap: Arc<Heap>) -> Self {
        StorageRead::ExternalHeap {
            ptr,
            version: Version::Storage,
            heap,
        }
    }

    /// The version this read was observed at.
    pub fn version(&self) -> Version {
        match self {
            StorageRead::DoesNotExist { version } => *version,
            StorageRead::ExternalHeap { version, .. } => *version,
        }
    }
}

// Manual: `Heap` has no `Debug`; print the pointer and version only.
impl fmt::Debug for StorageRead {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            StorageRead::DoesNotExist { version } => f
                .debug_struct("DoesNotExist")
                .field("version", version)
                .finish(),
            StorageRead::ExternalHeap { ptr, version, .. } => f
                .debug_struct("ExternalHeap")
                .field("ptr", ptr)
                .field("version", version)
                .finish(),
        }
    }
}

/// Returns resource data from storage. Storage backend is not fixed and can be
/// implemented for different clients:
///   - tests,
///   - Block-STM,
///   - actual DB.
pub trait ResourceProvider {
    /// Returns the resource of a particular type at the specified address.
    /// Returns [`StorageRead::DoesNotExist`] if the resource does not exist.
    /// Returns a [`ResourceProviderError`] if the backend cannot satisfy the
    /// read.
    fn get_resource(&self, key: &InMemoryStorageKey) -> Result<StorageRead, ResourceProviderError>;
}

/// Empty storage with no resources.
pub struct NoResourceProvider;

impl ResourceProvider for NoResourceProvider {
    fn get_resource(
        &self,
        _key: &InMemoryStorageKey,
    ) -> Result<StorageRead, ResourceProviderError> {
        Ok(StorageRead::does_not_exist_in_storage())
    }
}

// TODO(testing):
//   This is only needed to make current tests work. Remove once specializer can emit
//   struct / enum operations or when testing framework is unified.
pub static NO_RESOURCE_PROVIDER: NoResourceProvider = NoResourceProvider;
