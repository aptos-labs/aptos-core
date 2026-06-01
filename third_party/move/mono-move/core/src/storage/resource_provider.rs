// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource storage access for the runtime.

use crate::{types::InternedType, ExecutionErrorKind, IntoExecutionError};
use move_core_types::account_address::AccountAddress;
use std::ptr::NonNull;
use thiserror::Error;

/// Version of the read value (which can come from storage or from other
/// transaction write).
// TODO:
//   Replace with Block-STM transaction index and incarnation pair.
pub type Version = u64;

/// Key to (in-memory) global storage.
///
/// A key embeds an [`InternedType`], which is a pointer into the global type
/// arena. The key is therefore only valid while that arena is alive (for the
/// duration of execution, bounded by the execution guard). Keys must not be
/// stored past arena reset, nor compared across two different arenas: equality
/// and hashing rely on the interned pointer identity.
#[derive(Copy, Clone, Eq, PartialEq, Hash)]
pub enum StorageKey {
    /// Every resource can be identified in storage by the address where it is
    /// published and its struct/enum type.
    Resource(AccountAddress, InternedType),
    // TODO: Can add tables and other extensions here.
}

impl StorageKey {
    pub fn address(&self) -> AccountAddress {
        match self {
            StorageKey::Resource(addr, _) => *addr,
        }
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
    // TODO(safety):
    //   Figure out how to enforce compile-time guarantees here that owning
    //   arena is alive.
    ExternalHeap {
        /// Just like any other VM value, the pointer points to the start of
        /// the value allocation. Value's header is at negative offset.
        // TODO(refactor): have a Value pointer unified API?
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
    fn get_resource(&self, key: StorageKey) -> Result<StorageRead, ResourceProviderError>;
}

/// Empty storage with no resources.
pub struct NoResourceProvider;

impl ResourceProvider for NoResourceProvider {
    fn get_resource(&self, _key: StorageKey) -> Result<StorageRead, ResourceProviderError> {
        Ok(StorageRead::DoesNotExist)
    }
}

// TODO(test):
//   This is only needed to make current tests work. Remove once specializer can emit
//   struct / enum operations or when testing framework is unified.
pub static NO_RESOURCE_PROVIDER: NoResourceProvider = NoResourceProvider;
