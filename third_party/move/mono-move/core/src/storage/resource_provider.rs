// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource storage access for the runtime.

use crate::{native::TableHandle, types::InternedType, ExecutionErrorKind, IntoExecutionError};
use move_core_types::account_address::AccountAddress;
use std::ptr::NonNull;
use thiserror::Error;

/// Version of the read value (which can come from storage or from other
/// transaction write).
// TODO(completeness):
//   Replace with Block-STM transaction index and incarnation pair.
pub type Version = u64;

/// Key into the in-memory global storage of a single transaction.
///
/// Resources and table items live in the same per-transaction read-write set,
/// so they share one key enum and one map. Both are entries anchored at an
/// address: a resource at the address it is published under, a table item at
/// its table handle. Keeping them in one map lets every global-storage access
/// (resource ops and, later, native table ops) go through the same lookup,
/// read-set recording, undo journal, and checkpoint machinery.
///
/// The key is "in-memory" because it embeds interned, arena-backed data that
/// must not outlive the current execution. It is not a stable, serializable
/// storage key.
#[derive(Clone, Eq, PartialEq, Hash)]
pub enum InMemoryStorageKey {
    /// Every resource can be identified in storage by the address where it is
    /// published and its struct/enum type.
    ///
    /// A key embeds an [`InternedType`], which is a pointer into the global
    /// type arena. The key is therefore only valid while that arena is alive
    /// (for the duration of execution, bounded by the execution guard). Keys
    /// must not be stored past arena reset, nor compared across two different
    /// arenas: equality and hashing rely on the interned pointer identity.
    Resource {
        address: AccountAddress,
        ty: InternedType,
    },
    /// A table item, identified by its table handle and the serialized bytes of
    /// its key.
    TableItem {
        handle: TableHandle,
        // TODO(perf): consider interning these keys later.
        key: Box<[u8]>,
        /// The stored value's type, needed to materialize the item from storage.
        value_ty: InternedType,
    },
}

impl InMemoryStorageKey {
    /// Builds a resource key from its publishing address and interned type.
    pub fn resource(address: AccountAddress, ty: InternedType) -> Self {
        InMemoryStorageKey::Resource { address, ty }
    }

    /// Builds a table item key from its handle, serialized key bytes, and stored value type.
    pub fn table_item(handle: TableHandle, key: Box<[u8]>, value_ty: InternedType) -> Self {
        InMemoryStorageKey::TableItem {
            handle,
            key,
            value_ty,
        }
    }

    /// Returns the address a key is anchored at: the publishing address for a
    /// resource, or the table handle for a table item.
    pub fn address(&self) -> AccountAddress {
        match self {
            InMemoryStorageKey::Resource { address, .. } => *address,
            InMemoryStorageKey::TableItem { handle, .. } => handle.address(),
        }
    }
}

impl From<&InMemoryStorageKey> for InMemoryStorageKey {
    fn from(key: &InMemoryStorageKey) -> Self {
        key.clone()
    }
}

/// Errors a [`ResourceProvider`] can surface. Backends classify their
/// own failure modes into this enum as they grow.
#[derive(Debug, Error)]
pub enum ResourceProviderError {
    #[error("resource provider invariant violation: {0}")]
    InvariantViolation(String),
}

impl IntoExecutionError for ResourceProviderError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            ResourceProviderError::InvariantViolation(_) => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Storage read returned to the VM. Every VM execution records reads of any
/// value coming from global storage.
#[derive(Clone, Copy, Debug)]
pub enum StorageRead {
    /// Value does not exist at this key.
    DoesNotExist,
    /// Value is allocated in some other arena or cache. For example, it can be
    /// a cached DB read or a write from soe transaction at lower version.
    // TODO(cleanup):
    //   Figure out how to enforce compile-time guarantees here that owning
    //   arena is alive.
    ExternalHeap {
        /// Just like any other VM value, the pointer points to the start of
        /// the value allocation. Value's header is at negative offset.
        // TODO(cleanup): have a Value pointer unified API?
        ptr: NonNull<u8>,
        /// Version of this read from Block-STM. Used for read-set validation.
        version: Version,
    },
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
        Ok(StorageRead::DoesNotExist)
    }
}
