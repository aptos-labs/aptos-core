// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use bytes::Bytes;
use move_binary_format::{
    deserializer::DeserializerConfig,
    errors::{PartialVMError, PartialVMResult, VMResult},
    file_format_common::{IDENTIFIER_SIZE_MAX, VERSION_MAX},
    CompiledModule,
};
use move_bytecode_utils::compiled_module_viewer::CompiledModuleView;
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChangeSet, ChangeSet, Op},
    identifier::{IdentStr, Identifier},
    language_storage::{ModuleId, StructTag},
    metadata::Metadata,
    value::MoveTypeLayout,
    vm_status::StatusCode,
};
#[cfg(feature = "table-extension")]
use move_table_extension::{TableChangeSet, TableHandle, TableResolver};
use move_vm_runtime::{RuntimeEnvironment, WithRuntimeEnvironment};
use move_vm_types::{
    code::ModuleBytesStorage,
    resolver::{resource_size, ResourceResolver},
};
use std::{
    collections::{btree_map, BTreeMap},
    fmt::Debug,
};

/// A dummy storage containing no modules or resources.
#[derive(Debug, Clone)]
pub struct BlankStorage;

impl BlankStorage {
    pub fn new() -> Self {
        Self
    }
}

impl ModuleBytesStorage for BlankStorage {
    fn fetch_module_bytes(
        &self,
        _address: &AccountAddress,
        _module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        Ok(None)
    }
}

impl ResourceResolver for BlankStorage {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        _address: &AccountAddress,
        _tag: &StructTag,
        _metadata: &[Metadata],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        Ok((None, 0))
    }
}

#[cfg(feature = "table-extension")]
impl TableResolver for BlankStorage {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        _handle: &TableHandle,
        _key: &[u8],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<Option<Bytes>> {
        Ok(None)
    }
}

/// Simple in-memory storage for modules and resources under an account.
#[derive(Debug, Clone)]
struct InMemoryAccountStorage {
    resources: BTreeMap<StructTag, Bytes>,
    modules: BTreeMap<Identifier, Bytes>,
}

/// Simple in-memory storage that can be used as a Move VM storage backend for testing purposes.
#[derive(Clone)]
pub struct InMemoryStorage {
    runtime_environment: RuntimeEnvironment,
    accounts: BTreeMap<AccountAddress, InMemoryAccountStorage>,
    #[cfg(feature = "table-extension")]
    tables: BTreeMap<TableHandle, BTreeMap<Vec<u8>, Bytes>>,
}

impl ModuleBytesStorage for InMemoryStorage {
    fn fetch_module_bytes(
        &self,
        address: &AccountAddress,
        module_name: &IdentStr,
    ) -> VMResult<Option<Bytes>> {
        Ok(self
            .accounts
            .get(address)
            .and_then(|account_storage| account_storage.modules.get(module_name).cloned()))
    }
}
impl WithRuntimeEnvironment for InMemoryStorage {
    fn runtime_environment(&self) -> &RuntimeEnvironment {
        &self.runtime_environment
    }
}

impl CompiledModuleView for InMemoryStorage {
    type Item = CompiledModule;

    fn view_compiled_module(&self, id: &ModuleId) -> anyhow::Result<Option<Self::Item>> {
        Ok(match self.fetch_module_bytes(id.address(), id.name())? {
            Some(bytes) => {
                let config = DeserializerConfig::new(VERSION_MAX, IDENTIFIER_SIZE_MAX);
                Some(CompiledModule::deserialize_with_config(&bytes, &config)?)
            },
            None => None,
        })
    }
}

fn apply_changes<K, V>(
    map: &mut BTreeMap<K, V>,
    changes: impl IntoIterator<Item = (K, Op<V>)>,
) -> PartialVMResult<()>
where
    K: Ord + Debug,
{
    use btree_map::Entry::*;
    use Op::*;

    for (k, op) in changes.into_iter() {
        match (map.entry(k), op) {
            (Occupied(entry), New(_)) => {
                return Err(
                    PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                        "Failed to apply changes -- key {:?} already exists",
                        entry.key()
                    )),
                )
            },
            (Occupied(entry), Delete) => {
                entry.remove();
            },
            (Occupied(entry), Modify(val)) => {
                *entry.into_mut() = val;
            },
            (Vacant(entry), New(val)) => {
                entry.insert(val);
            },
            (Vacant(entry), Delete | Modify(_)) => {
                return Err(
                    PartialVMError::new(StatusCode::STORAGE_ERROR).with_message(format!(
                        "Failed to apply changes -- key {:?} does not exist",
                        entry.key()
                    )),
                )
            },
        }
    }
    Ok(())
}

fn get_or_insert<K, V, F>(map: &mut BTreeMap<K, V>, key: K, make_val: F) -> &mut V
where
    K: Ord,
    F: FnOnce() -> V,
{
    use btree_map::Entry::*;

    match map.entry(key) {
        Occupied(entry) => entry.into_mut(),
        Vacant(entry) => entry.insert(make_val()),
    }
}

impl InMemoryAccountStorage {
    fn apply(&mut self, account_changeset: AccountChangeSet) -> PartialVMResult<()> {
        let resources = account_changeset.into_resources();
        apply_changes(&mut self.resources, resources)?;
        Ok(())
    }

    fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            resources: BTreeMap::new(),
        }
    }
}

impl InMemoryStorage {
    pub fn apply_extended(
        &mut self,
        changeset: ChangeSet,
        #[cfg(feature = "table-extension")] table_changes: TableChangeSet,
    ) -> PartialVMResult<()> {
        for (addr, account_changeset) in changeset.into_inner() {
            match self.accounts.entry(addr) {
                btree_map::Entry::Occupied(entry) => {
                    entry.into_mut().apply(account_changeset)?;
                },
                btree_map::Entry::Vacant(entry) => {
                    let mut account_storage = InMemoryAccountStorage::new();
                    account_storage.apply(account_changeset)?;
                    entry.insert(account_storage);
                },
            }
        }

        #[cfg(feature = "table-extension")]
        self.apply_table(table_changes)?;

        Ok(())
    }

    pub fn apply(&mut self, changeset: ChangeSet) -> PartialVMResult<()> {
        self.apply_extended(
            changeset,
            #[cfg(feature = "table-extension")]
            TableChangeSet::default(),
        )
    }

    #[cfg(feature = "table-extension")]
    fn apply_table(&mut self, changes: TableChangeSet) -> PartialVMResult<()> {
        let TableChangeSet {
            new_tables,
            removed_tables,
            changes,
        } = changes;
        self.tables.retain(|h, _| !removed_tables.contains(h));
        self.tables
            .extend(new_tables.keys().map(|h| (*h, BTreeMap::default())));
        for (h, c) in changes {
            assert!(
                self.tables.contains_key(&h),
                "inconsistent table change set: stale table handle"
            );
            let table = self.tables.get_mut(&h).unwrap();
            apply_changes(table, c.entries)?;
        }
        Ok(())
    }

    pub fn new() -> Self {
        Self {
            runtime_environment: RuntimeEnvironment::new(vec![]),
            accounts: BTreeMap::new(),
            #[cfg(feature = "table-extension")]
            tables: BTreeMap::new(),
        }
    }

    pub fn new_with_runtime_environment(runtime_environment: RuntimeEnvironment) -> Self {
        Self {
            runtime_environment,
            accounts: BTreeMap::new(),
            #[cfg(feature = "table-extension")]
            tables: BTreeMap::new(),
        }
    }

    pub fn max_binary_format_version(&self) -> u32 {
        self.runtime_environment
            .vm_config()
            .deserializer_config
            .max_binary_format_version
    }

    /// Adds serialized module bytes to this storage.
    pub fn add_module_bytes(
        &mut self,
        address: &AccountAddress,
        module_name: &IdentStr,
        bytes: Bytes,
    ) {
        let account = get_or_insert(&mut self.accounts, *address, || {
            InMemoryAccountStorage::new()
        });
        account.modules.insert(module_name.to_owned(), bytes);
    }

    pub fn publish_or_overwrite_resource(
        &mut self,
        addr: AccountAddress,
        struct_tag: StructTag,
        blob: Vec<u8>,
    ) {
        let account = get_or_insert(&mut self.accounts, addr, InMemoryAccountStorage::new);
        account.resources.insert(struct_tag, blob.into());
    }
}

impl ResourceResolver for InMemoryStorage {
    fn get_resource_bytes_with_metadata_and_layout(
        &self,
        address: &AccountAddress,
        tag: &StructTag,
        _metadata: &[Metadata],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<(Option<Bytes>, usize)> {
        if let Some(account_storage) = self.accounts.get(address) {
            let buf = account_storage.resources.get(tag).cloned();
            let buf_size = resource_size(&buf);
            return Ok((buf, buf_size));
        }
        Ok((None, 0))
    }
}

#[cfg(feature = "table-extension")]
impl TableResolver for InMemoryStorage {
    fn resolve_table_entry_bytes_with_layout(
        &self,
        handle: &TableHandle,
        key: &[u8],
        _maybe_layout: Option<&MoveTypeLayout>,
    ) -> PartialVMResult<Option<Bytes>> {
        Ok(self.tables.get(handle).and_then(|t| t.get(key).cloned()))
    }
}
