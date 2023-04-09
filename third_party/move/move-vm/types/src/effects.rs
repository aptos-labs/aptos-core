// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines serialization-free data types produced by the VM session.

use crate::resolver::{Module, Resource};
use anyhow::{bail, Result};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChangeSet as LegacyAccountChangeSet, ChangeSet as LegacyChangeSet, Op},
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use std::collections::{btree_map::Entry, BTreeMap};

#[derive(Default, Debug, Clone)]
pub struct AccountChangeSet {
    modules: BTreeMap<Identifier, Op<Module>>,
    resources: BTreeMap<StructTag, Op<Resource>>,
}

impl AccountChangeSet {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            resources: BTreeMap::new(),
        }
    }

    pub fn from_modules_resources(
        modules: BTreeMap<Identifier, Op<Module>>,
        resources: BTreeMap<StructTag, Op<Resource>>,
    ) -> Self {
        Self { modules, resources }
    }

    pub fn add_module_op(&mut self, name: Identifier, op: Op<Module>) -> Result<()> {
        match self.modules.entry(name) {
            Entry::Occupied(entry) => bail!("Module {} already exists", entry.key()),
            Entry::Vacant(entry) => {
                entry.insert(op);
            },
        }
        Ok(())
    }

    pub fn add_resource_op(&mut self, struct_tag: StructTag, op: Op<Resource>) -> Result<()> {
        match self.resources.entry(struct_tag) {
            Entry::Occupied(entry) => bail!("Resource {} already exists", entry.key()),
            Entry::Vacant(entry) => {
                entry.insert(op);
            },
        }
        Ok(())
    }

    pub fn into_inner(
        self,
    ) -> (
        BTreeMap<Identifier, Op<Module>>,
        BTreeMap<StructTag, Op<Resource>>,
    ) {
        (self.modules, self.resources)
    }

    pub fn modules(&self) -> &BTreeMap<Identifier, Op<Module>> {
        &self.modules
    }

    pub fn resources(&self) -> &BTreeMap<StructTag, Op<Resource>> {
        &self.resources
    }

    pub fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.resources.is_empty()
    }

    fn into_legacy_account_change_set(self) -> PartialVMResult<LegacyAccountChangeSet> {
        let (modules, resources) = self.into_inner();

        macro_rules! into_blobs {
            ($new_changes:ident, $old_changes:ident) => {{
                for (key, op) in $old_changes {
                    let new_op = op.and_then(|change| {
                        change
                            .into_bytes()
                            .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
                    })?;
                    $new_changes.insert(key, new_op);
                }
            }};
        }

        let mut modules_as_blobs = BTreeMap::new();
        into_blobs!(modules_as_blobs, modules);
        let mut resources_as_blobs = BTreeMap::new();
        into_blobs!(resources_as_blobs, resources);

        Ok(LegacyAccountChangeSet::from_modules_resources(
            modules_as_blobs,
            resources_as_blobs,
        ))
    }
}

#[derive(Default, Debug, Clone)]
pub struct ChangeSet {
    accounts: BTreeMap<AccountAddress, AccountChangeSet>,
}

impl ChangeSet {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
        }
    }

    /// Adds changes for specific account to this change set.
    pub fn add_account_changeset(
        &mut self,
        addr: AccountAddress,
        account_change_set: AccountChangeSet,
    ) -> Result<()> {
        match self.accounts.entry(addr) {
            Entry::Occupied(_) => bail!(
                "Failed to add account change set. Account {} already exists.",
                addr
            ),
            Entry::Vacant(entry) => {
                entry.insert(account_change_set);
            },
        }
        Ok(())
    }

    pub fn accounts(&self) -> &BTreeMap<AccountAddress, AccountChangeSet> {
        &self.accounts
    }

    pub fn into_inner(self) -> BTreeMap<AccountAddress, AccountChangeSet> {
        self.accounts
    }

    fn get_or_insert_account_change_set(&mut self, addr: AccountAddress) -> &mut AccountChangeSet {
        match self.accounts.entry(addr) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(AccountChangeSet::new()),
        }
    }

    pub fn add_module_op(&mut self, module_id: ModuleId, op: Op<Module>) -> Result<()> {
        let account = self.get_or_insert_account_change_set(*module_id.address());
        account.add_module_op(module_id.name().to_owned(), op)
    }

    pub fn add_resource_op(
        &mut self,
        addr: AccountAddress,
        struct_tag: StructTag,
        op: Op<Resource>,
    ) -> Result<()> {
        let account = self.get_or_insert_account_change_set(addr);
        account.add_resource_op(struct_tag, op)
    }

    pub fn into_legacy_change_set(self) -> VMResult<LegacyChangeSet> {
        let accounts = self.into_inner();
        let mut change_set = LegacyChangeSet::new();
        for (addr, account_change_set) in accounts {
            change_set
                .add_account_changeset(
                    addr,
                    account_change_set
                        .into_legacy_account_change_set()
                        .map_err(|e: PartialVMError| e.finish(Location::Undefined))?,
                )
                .expect("accounts should be unique");
        }
        Ok(change_set)
    }
}
