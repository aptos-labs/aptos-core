// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{
    on_chain_config::{CurrentTimeMicroseconds, OnChainConfig},
    state_store::state_value::StateValueMetadata,
};
use bytes::Bytes;
use mono_move_core::{
    storage::resource_provider::{InMemoryStorageKey, ResourceProvider, ResourceProviderError},
    types::InternedType,
    FrameworkSymbols,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{serialize, InterpreterContext};
use move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, move_resource::MoveStructType,
};
use serde::de::DeserializeOwned;
use std::collections::BTreeMap;

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups.
pub trait AptosDataProvider: ResourceProvider {
    /// The group stored behind `group_key`, as execution read it, or `None`
    /// if no group is stored there. `group_key` must be an
    /// [`InMemoryStorageKey::ResourceGroup`].
    fn stored_group(
        &self,
        group_key: &InMemoryStorageKey,
    ) -> Result<Option<StoredGroup>, ResourceProviderError>;
}

/// Checks that `group_key` names a resource group's slot, as
/// [`AptosDataProvider::group_members`] requires.
pub fn check_group_key(group_key: &InMemoryStorageKey) -> Result<(), ResourceProviderError> {
    match group_key {
        InMemoryStorageKey::ResourceGroup { .. } => Ok(()),
        InMemoryStorageKey::Resource { .. } | InMemoryStorageKey::TableItem { .. } => {
            Err(ResourceProviderError::InvariantViolation(format!(
                "{group_key:?} is not a resource group"
            )))
        },
    }
}

/// The BCS bytes of the resource of interned type `ty` at `address`, read
/// through `interp` so the read is recorded like one made by Move code.
/// `None` if there is none. The types are interned once, in `FrameworkSymbols`.
//
// TODO(perf): the value goes through BCS; decode the flat value directly.
fn read_resource_bytes(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    address: AccountAddress,
    ty: InternedType,
) -> Result<Option<Vec<u8>>, ResourceProviderError> {
    let Some(ptr) = interp
        .read_resource(address, ty)
        .map_err(|e| ResourceProviderError::InvariantViolation(e.to_string()))?
    else {
        return Ok(None);
    };
    // SAFETY: the read-write set pins `ptr` for as long as `interp` lives, and
    // `read_resource` published the layout of `ty`.
    unsafe { serialize(guard, ptr.as_ptr(), ty) }
        .map(Some)
        .map_err(|e| ResourceProviderError::InvariantViolation(e.to_string()))
}

/// Reads the on-chain config `T`, whose interned type is `ty`, through `interp`.
pub(crate) fn read_config<T: OnChainConfig>(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    ty: InternedType,
) -> Result<Option<T>, ResourceProviderError> {
    read_resource_bytes(interp, guard, *T::address(), ty)?
        .map(|bytes| {
            T::deserialize_into_config(&bytes)
                .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:#}")))
        })
        .transpose()
}

/// The chain's time, which stamps the slots a session creates, or zero if the
/// chain has no time yet. Read unmetered. As a session's first read, it is the
/// time before the transaction.
pub(crate) fn read_creation_time_usecs(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    symbols: &FrameworkSymbols,
) -> Result<u64, ResourceProviderError> {
    let time = interp.unmetered(|interp| {
        read_config::<CurrentTimeMicroseconds>(interp, guard, symbols.current_time_microseconds)
    })?;
    Ok(time.map_or(0, |time| time.microseconds))
}

/// Reads the resource `T`, whose interned type is `ty`, at `address` through
/// `interp`.
pub(crate) fn read_resource<T: MoveStructType + DeserializeOwned>(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    ty: InternedType,
    address: AccountAddress,
) -> Result<Option<T>, ResourceProviderError> {
    read_resource_bytes(interp, guard, address, ty)?
        .map(|bytes| {
            bcs::from_bytes(&bytes)
                .map_err(|e| ResourceProviderError::InvariantViolation(e.to_string()))
        })
        .transpose()
}

/// A resource group's members and their stored bytes.
//
// TODO(cleanup): consider using interned types and unordered map.
pub type GroupMembers = BTreeMap<StructTag, Bytes>;

/// A resource group as storage holds it.
#[derive(Clone)]
pub struct StoredGroup {
    pub members: GroupMembers,
    /// Metadata of the group's slot.
    pub metadata: StateValueMetadata,
}

/// Decodes a group's stored blob.
pub fn decode_group_members(blob: &[u8]) -> Result<GroupMembers, ResourceProviderError> {
    bcs::from_bytes(blob).map_err(|e| {
        ResourceProviderError::InvariantViolation(format!("malformed resource group: {e}"))
    })
}
