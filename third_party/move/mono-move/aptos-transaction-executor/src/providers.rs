// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{on_chain_config::OnChainConfig, state_store::state_key::StateKey};
use bytes::Bytes;
use mono_move_core::{
    intern_type_tag,
    storage::resource_provider::{InMemoryStorageKey, ResourceProvider, ResourceProviderError},
};
use mono_move_global_context::ExecutionGuard;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{StructTag, TypeTag},
    move_resource::MoveStructType,
};
use serde::de::DeserializeOwned;
use std::collections::{BTreeMap, HashMap};

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups and byte-level reads.
pub trait AptosDataProvider: ResourceProvider {
    /// The stored members of the group behind `group_key`, as execution read
    /// them, or `None` if no group is stored there.
    fn group_members(
        &self,
        group_key: &StateKey,
    ) -> Result<Option<GroupMembers>, ResourceProviderError>;

    /// The stored bytes of the plain resource at `key`, as execution would
    /// read them, or `None` if none is stored there.
    fn resource_bytes(
        &self,
        key: &InMemoryStorageKey,
    ) -> Result<Option<Bytes>, ResourceProviderError>;
}

/// The key of the resource `tag` at `address`.
//
// TODO(perf): the type is interned on every call; intern it once.
fn resource_key(
    guard: &ExecutionGuard<'_>,
    address: AccountAddress,
    tag: StructTag,
) -> Result<InMemoryStorageKey, ResourceProviderError> {
    let ty = intern_type_tag(&TypeTag::Struct(Box::new(tag)), guard)
        .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:#}")))?;
    Ok(InMemoryStorageKey::resource(address, ty))
}

/// Reads the on-chain config `T` as execution would see it.
//
// TODO(perf): the value goes through BCS; decode the flat value directly.
pub(crate) fn read_config<T: OnChainConfig>(
    guard: &ExecutionGuard<'_>,
    provider: &dyn AptosDataProvider,
) -> Result<Option<T>, ResourceProviderError> {
    let key = resource_key(guard, *T::address(), T::struct_tag())?;
    provider
        .resource_bytes(&key)?
        .map(|bytes| {
            T::deserialize_into_config(&bytes)
                .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:#}")))
        })
        .transpose()
}

/// Reads the resource `T` at `address` as execution would see it.
//
// TODO(perf): the value goes through BCS; decode the flat value directly.
pub(crate) fn read_resource<T: MoveStructType + DeserializeOwned>(
    guard: &ExecutionGuard<'_>,
    provider: &dyn AptosDataProvider,
    address: AccountAddress,
) -> Result<Option<T>, ResourceProviderError> {
    let key = resource_key(guard, address, T::struct_tag())?;
    provider
        .resource_bytes(&key)?
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

/// The resource groups a transaction assembled during materialization, keyed by
/// group slot. `None` marks a group the transaction deleted.
pub type MaterializedGroups = HashMap<StateKey, Option<GroupMembers>>;

/// Decodes a group's stored blob.
pub fn decode_group_members(blob: &[u8]) -> Result<GroupMembers, ResourceProviderError> {
    bcs::from_bytes(blob).map_err(|e| {
        ResourceProviderError::InvariantViolation(format!("malformed resource group: {e}"))
    })
}
