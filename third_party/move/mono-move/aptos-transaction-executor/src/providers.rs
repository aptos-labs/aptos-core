// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_types::{on_chain_config::OnChainConfig, state_store::state_key::StateKey};
use bytes::Bytes;
use mono_move_core::{
    storage::resource_provider::{ResourceProvider, ResourceProviderError},
    types::InternedType,
};
use mono_move_global_context::ExecutionGuard;
use mono_move_runtime::{serialize, InterpreterContext};
use move_core_types::language_storage::StructTag;
use std::collections::{BTreeMap, HashMap};

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups.
pub trait AptosDataProvider: ResourceProvider {
    /// The stored members of the group behind `group_key`, as execution read
    /// them, or `None` if no group is stored there.
    fn group_members(
        &self,
        group_key: &StateKey,
    ) -> Result<Option<GroupMembers>, ResourceProviderError>;
}

/// Reads the on-chain config `T`, whose interned type is `ty` (see
/// `FrameworkSymbols`), through `interp`, so the read is recorded like one
/// made by Move code.
//
// TODO(perf): the value goes through BCS; decode the flat value directly.
pub(crate) fn read_config<T: OnChainConfig>(
    interp: &mut InterpreterContext<'_>,
    guard: &ExecutionGuard<'_>,
    ty: InternedType,
) -> Result<Option<T>, ResourceProviderError> {
    let Some(ptr) = interp
        .read_resource(*T::address(), ty)
        .map_err(|e| ResourceProviderError::InvariantViolation(e.to_string()))?
    else {
        return Ok(None);
    };
    // SAFETY: the read-write set pins `ptr` for as long as `interp` lives, and
    // `read_resource` published the layout of `ty`.
    let bytes = unsafe { serialize(guard, ptr.as_ptr(), ty) }
        .map_err(|e| ResourceProviderError::InvariantViolation(e.to_string()))?;
    T::deserialize_into_config(&bytes)
        .map(Some)
        .map_err(|e| ResourceProviderError::InvariantViolation(format!("{e:#}")))
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
