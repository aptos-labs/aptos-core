// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use anyhow::{anyhow, Result};
use aptos_types::{
    state_store::{state_key::StateKey, table::TableHandle},
    vm::module_metadata::RuntimeModuleMetadataV1,
};
use bytes::Bytes;
use mono_move_core::{
    intern_struct_tag,
    storage::{
        module_provider::ModuleProvider,
        resource_provider::{InMemoryStorageKey, ResourceProvider},
    },
    struct_tag_of,
    types::InternedType,
    Interner, VMResult,
};
use move_core_types::{account_address::AccountAddress, language_storage::StructTag};
use std::collections::{BTreeMap, HashMap};
use triomphe::Arc;

/// Trait extending the runtime's [`ModuleProvider`] interface with the Aptos module metadata
/// execution consults.
pub trait AptosModuleProvider: ModuleProvider {
    /// A module's Aptos metadata, or `None` if the module is absent or carries none.
    ///
    /// Every transaction asks for the metadata of the module its entry function is in, so an
    /// implementation that deserializes the module to answer costs several times what executing the
    /// transaction does. Cache it.
    fn module_metadata(
        &self,
        address: &AccountAddress,
        name: &str,
    ) -> VMResult<Option<Arc<RuntimeModuleMetadataV1>>>;
}

/// Trait extending the runtime's [`ResourceProvider`] interface with additional capabilities to
/// handle resource groups and table items.
pub trait AptosDataProvider: ResourceProvider {
    /// The resource group a resource type belongs to, if any.
    fn group_of(&self, ty: InternedType) -> Result<Option<InternedType>>;

    /// The stored members of the group behind `group_key`, as execution read
    /// them, or `None` if no group is stored there.
    fn group_members(&self, group_key: &StateKey) -> Result<Option<Arc<GroupMembers>>>;

    /// Resolves where the value at a read-write-set key lives in state
    /// storage.
    fn locate_key(&self, key: &InMemoryStorageKey) -> Result<StorageLocation> {
        Ok(match key {
            InMemoryStorageKey::Resource { address, ty } => match self.group_of(*ty)? {
                Some(group_ty) => StorageLocation::GroupMember {
                    group: StateKey::resource_group(address, &nominal_tag(group_ty)?),
                    member: *ty,
                },
                None => StorageLocation::OwnSlot(
                    StateKey::resource(address, &nominal_tag(*ty)?)
                        .map_err(|e| anyhow!("bad state key: {e}"))?,
                ),
            },
            InMemoryStorageKey::TableItem { handle, key, .. } => {
                StorageLocation::OwnSlot(StateKey::table_item(&TableHandle(handle.address()), key))
            },
        })
    }
}

/// A resource group's members and their stored bytes. Unordered: the stored
/// encoding is canonical in struct-tag order.
//
// TODO(cleanup): consider `shared-dsa`'s `UnorderedSet` and make it explicit that
// the iteration order can be non-deterministic.
pub type GroupMembers = HashMap<InternedType, Bytes>;

/// Where a value lives in state storage.
pub enum StorageLocation {
    /// In a slot of its own.
    OwnSlot(StateKey),
    /// Inside a resource group's slot, under the member type.
    GroupMember {
        group: StateKey,
        member: InternedType,
    },
}

/// Decodes a group's stored blob, interning each member's struct tag.
pub fn decode_group_members(blob: &[u8], interner: &impl Interner) -> Result<GroupMembers> {
    let tagged: BTreeMap<StructTag, Bytes> = bcs::from_bytes(blob)?;
    tagged
        .into_iter()
        .map(|(tag, bytes)| Ok((intern_struct_tag(&tag, interner)?, bytes)))
        .collect()
}

/// The struct tag of a nominal type, for storage keys.
//
// TODO(perf): should be a cached method on the context, which would also let the
// state-view providers stop open-coding it.
pub(crate) fn nominal_tag(ty: InternedType) -> Result<StructTag> {
    struct_tag_of(ty).ok_or_else(|| anyhow!("resource type is not nominal"))
}
