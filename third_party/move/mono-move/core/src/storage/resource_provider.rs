// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Resource storage access for the runtime.

use crate::{
    native::TableHandle, struct_tag_of, types::InternedType, ExecutionErrorKind,
    IntoExecutionError, VMInternalError,
};
use anyhow::anyhow;
use aptos_types::state_store::{state_key::StateKey, table::TableHandle as AptosTableHandle};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, value::size_u32_as_uleb128,
};
use std::{ptr::NonNull, sync::Arc};
use thiserror::Error;

/// Block-STM index of a transaction in the block.
pub type TxnIdx = u32;

/// Block-STM incarnation of a transaction: how many times it was re-executed.
pub type Incarnation = u32;

/// Version of the read value (which can come from storage or from other
/// transaction write). [`None`] means the value comes from pre-block storage,
/// [`Some`] carries the Block-STM transaction index and incarnation that wrote
/// it.
pub type Version = Option<(TxnIdx, Incarnation)>;

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
// TODO(security): Debug derivation is not safe! Remove before production.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
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
    /// A resource group's own storage slot, identified by the address it is
    /// published under and the group's type. Group members keep their
    /// [`Self::Resource`] keys and are addressed as tags within this slot.
    //
    // TODO(cleanup): the only reason why we need it here is Block-STM treats
    // group metadata reads/writes as some dummy resources. Until this is
    // refactored, we keep this here.
    ResourceGroup {
        address: AccountAddress,
        group_ty: InternedType,
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
        Self::Resource { address, ty }
    }

    /// Builds a resource group key from its publishing address and interned
    /// group type.
    pub fn resource_group(address: AccountAddress, group_ty: InternedType) -> Self {
        Self::ResourceGroup { address, group_ty }
    }

    /// Builds a table item key from its handle, serialized key bytes, and stored value type.
    pub fn table_item(handle: TableHandle, key: Box<[u8]>, value_ty: InternedType) -> Self {
        Self::TableItem {
            handle,
            key,
            value_ty,
        }
    }

    /// Returns the address a key is anchored at: the publishing address for a
    /// resource, or the table handle for a table item.
    pub fn address(&self) -> AccountAddress {
        match self {
            Self::Resource { address, .. } => *address,
            Self::ResourceGroup { address, .. } => *address,
            Self::TableItem { handle, .. } => handle.address(),
        }
    }

    /// The type of the value stored at this key.
    pub fn value_ty(&self) -> InternedType {
        match self {
            Self::Resource { ty, .. } => *ty,
            Self::ResourceGroup { group_ty, .. } => *group_ty,
            Self::TableItem { value_ty, .. } => *value_ty,
        }
    }

    /// The own storage slot for a non-group-member key: a standalone resource, a
    /// resource group, or a table item. Table items are never resource-group
    /// members, so they always land here.
    pub fn as_state_key(&self) -> anyhow::Result<StateKey> {
        Ok(match self {
            Self::Resource { address, ty } => StateKey::resource(address, &nominal_tag(*ty)?)?,
            Self::ResourceGroup { address, group_ty } => {
                StateKey::resource_group(address, &nominal_tag(*group_ty)?)
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                StateKey::table_item(&AptosTableHandle(handle.address()), key)
            },
        })
    }
}

impl From<&InMemoryStorageKey> for InMemoryStorageKey {
    fn from(key: &InMemoryStorageKey) -> Self {
        key.clone()
    }
}

/// The struct tag of a nominal type, for storage keys.
//
// TODO(perf): should be a cached method on the context, which would also let the
// state-view providers stop open-coding it.
pub fn nominal_tag(ty: InternedType) -> anyhow::Result<StructTag> {
    struct_tag_of(ty).ok_or_else(|| anyhow!("resource type is not nominal"))
}

/// Errors a [`ResourceProvider`] can surface. Backends classify their
/// own failure modes into this enum as they grow.
#[derive(Debug, Error)]
pub enum ResourceProviderError {
    #[error("resource provider invariant violation: {0}")]
    InvariantViolation(String),
    /// The read cannot be served because the parallel execution this
    /// transaction runs under has been halted or the value it depends on is
    /// still unresolved. The transaction's outcome must be thrown away, not
    /// committed.
    //
    // TODO(cleanup): replace with Interrupted variant when killswitch lands.
    #[error("resource provider speculative abort: {0}")]
    SpeculativeAbort(String),
    /// A VM operation the provider ran on the read's behalf failed, such as
    /// deserializing a stored value.
    #[error("resource provider VM error: {0}")]
    VMInternal(#[from] VMInternalError),
}

impl IntoExecutionError for ResourceProviderError {
    fn kind(&self) -> ExecutionErrorKind {
        match self {
            // A stored value the provider cannot serve is an invariant
            // violation, whatever the VM operation failed with.
            ResourceProviderError::InvariantViolation(_)
            | ResourceProviderError::SpeculativeAbort(_)
            | ResourceProviderError::VMInternal(_) => ExecutionErrorKind::InvariantViolation,
        }
    }
}

/// Marks an arena as safe to hand out pointers into: implementing it asserts
/// that allocations never move and are never freed while the arena is alive.
/// Implemented by a storage provider's cache arena and by another transaction's
/// frozen heap.
///
/// A [`StorageRead::ExternalHeap`] holds an `Arc<dyn ReadPin>` on the arena its
/// pointer points into, so that arena outlives the read.
///
/// This trait must stay method-free. Under parallel execution a pin is held by
/// threads other than the one that owns the arena, and those arenas are
/// interior-mutable and `!Sync`. Holding the pin is sound only because a holder
/// cannot reach the arena through it.
pub trait ReadPin {}

/// Storage read returned to the VM. Every VM execution records reads of any
/// value coming from global storage.
#[derive(Clone)]
pub enum StorageRead {
    /// Value does not exist at this key.
    DoesNotExist {
        /// Version of this negative read from Block-STM. Used for read-set
        /// validation.
        version: Version,
    },
    /// Value is allocated in some other arena or cache. For example, it can be
    /// a cached DB read or a write from soe transaction at lower version.
    ExternalHeap {
        /// Just like any other VM value, the pointer points to the start of
        /// the value allocation. Value's header is at negative offset.
        // TODO(cleanup): have a Value pointer unified API?
        ptr: NonNull<u8>,
        /// Size of the value's BCS encoding.
        bcs_size: usize,
        /// Version of this read from Block-STM. Used for read-set validation.
        version: Version,
        /// Keeps the arena `ptr` points into alive while this read is retained.
        pin: Arc<dyn ReadPin>,
    },
}

impl StorageRead {
    /// Size of the value's BCS encoding, or [`None`] if there is no value.
    pub fn bcs_size(&self) -> Option<usize> {
        match self {
            StorageRead::DoesNotExist { .. } => None,
            StorageRead::ExternalHeap { bcs_size, .. } => Some(*bcs_size),
        }
    }

    /// The Block-STM version this read observed.
    pub fn version(&self) -> Version {
        match self {
            StorageRead::DoesNotExist { version } => *version,
            StorageRead::ExternalHeap { version, .. } => *version,
        }
    }
}

/// A resource group's size as BCS encodes it: the member count, then each
/// member's tag and length-prefixed bytes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GroupSize {
    num_members: usize,
    /// Sum over members of the encoded tag and the length-prefixed bytes.
    members_size: usize,
}

impl GroupSize {
    /// The size of a group whose members' tags and values encode to the given
    /// numbers of bytes.
    pub fn from_members(members: impl IntoIterator<Item = (usize, usize)>) -> Self {
        let mut num_members = 0;
        let mut members_size = 0;
        for (tag_len, value_len) in members {
            num_members += 1;
            members_size += member_size(tag_len, value_len);
        }
        Self {
            num_members,
            members_size,
        }
    }

    /// Counts in a member whose tag encodes to `tag_len` bytes and whose value
    /// to `value_len` bytes. Returns [`None`] on overflow.
    pub fn add_member(&mut self, tag_len: usize, value_len: usize) -> Option<()> {
        let num_members = self.num_members.checked_add(1)?;
        let members_size = self
            .members_size
            .checked_add(member_size(tag_len, value_len))?;
        *self = Self {
            num_members,
            members_size,
        };
        Some(())
    }

    /// Counts out a member whose tag encodes to `tag_len` bytes and whose value
    /// to `value_len` bytes. Returns [`None`] if the group is too small to
    /// have held it.
    pub fn remove_member(&mut self, tag_len: usize, value_len: usize) -> Option<()> {
        let num_members = self.num_members.checked_sub(1)?;
        let members_size = self
            .members_size
            .checked_sub(member_size(tag_len, value_len))?;
        *self = Self {
            num_members,
            members_size,
        };
        Some(())
    }

    /// The length of the encoded group. Zero for a group with no members,
    /// which is not stored.
    pub fn bcs_size(&self) -> usize {
        if self.num_members == 0 {
            0
        } else {
            size_u32_as_uleb128(self.num_members) + self.members_size
        }
    }
}

/// What one member adds to its group's encoding.
fn member_size(tag_len: usize, value_len: usize) -> usize {
    tag_len + size_u32_as_uleb128(value_len) + value_len
}

/// Returns resource data from storage. Storage backend is not fixed and can be
/// implemented for different clients:
///   - tests,
///   - Block-STM,
///   - actual DB.
pub trait ResourceProvider {
    /// Returns the resource of a particular type at the specified address or
    /// a resource group member (where the group is additionally identified by
    /// an optional type, [`None`] means a regular resource).
    /// Returns [`StorageRead::DoesNotExist`] if the resource does not exist.
    /// Returns a [`ResourceProviderError`] if the backend cannot satisfy the
    /// read.
    fn get_resource(
        &self,
        key: &InMemoryStorageKey,
        group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError>;

    /// Returns the size of the resource group at `group_key`, an
    /// [`InMemoryStorageKey::ResourceGroup`], as this transaction sees it.
    /// A group that does not exist has size zero.
    fn get_resource_group_size(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<GroupSize, ResourceProviderError>;
}

/// Empty storage with no resources.
pub struct NoResourceProvider;

impl ResourceProvider for NoResourceProvider {
    fn get_resource(
        &self,
        _key: &InMemoryStorageKey,
        _group: Option<InternedType>,
    ) -> Result<StorageRead, ResourceProviderError> {
        Ok(StorageRead::DoesNotExist { version: None })
    }

    fn get_resource_group_size(
        &self,
        _group_key: &InMemoryStorageKey,
    ) -> Result<GroupSize, ResourceProviderError> {
        Ok(GroupSize::default())
    }
}
