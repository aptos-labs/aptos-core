// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! Defines serialization-free data types produced by the VM session.

use crate::resolver::Resource;
use anyhow::{bail, Result};
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{AccountChangeSet, ChangeSet, Op},
    identifier::Identifier,
    language_storage::{ModuleId, StructTag},
    vm_status::StatusCode,
};
use std::collections::{btree_map::Entry, BTreeMap};

/// A collection of changes to resources and modules for an account (not serialized).
#[derive(Default, Debug, Clone)]
pub struct AccountChangeSetV2 {
    // TODO: Avoid module serialization.
    modules: BTreeMap<Identifier, Op<Vec<u8>>>,
    resources: BTreeMap<StructTag, Op<Resource>>,
}

impl AccountChangeSetV2 {
    /// Creates an empty change set for an account.
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            resources: BTreeMap::new(),
        }
    }

    /// Creates a change set for an account from the given resources and modules.
    pub fn from_modules_resources(
        modules: BTreeMap<Identifier, Op<Vec<u8>>>,
        resources: BTreeMap<StructTag, Op<Resource>>,
    ) -> Self {
        Self { modules, resources }
    }

    /// Adds a change to account's modules.
    pub fn add_module_op(&mut self, name: Identifier, op: Op<Vec<u8>>) -> Result<()> {
        match self.modules.entry(name) {
            Entry::Occupied(entry) => bail!("Module {} already exists", entry.key()),
            Entry::Vacant(entry) => {
                entry.insert(op);
            },
        }
        Ok(())
    }

    /// Adds a change to account's resources.
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
        BTreeMap<Identifier, Op<Vec<u8>>>,
        BTreeMap<StructTag, Op<Resource>>,
    ) {
        (self.modules, self.resources)
    }

    /// Returns module changes for this account.
    pub fn modules(&self) -> &BTreeMap<Identifier, Op<Vec<u8>>> {
        &self.modules
    }

    /// Returns resource changes for this account.
    pub fn resources(&self) -> &BTreeMap<StructTag, Op<Resource>> {
        &self.resources
    }

    /// Returns true if this account has no changes.
    pub fn is_empty(&self) -> bool {
        self.modules.is_empty() && self.resources.is_empty()
    }

    /// Converts all changes to this account into blobs. Used for backwards compatibility
    /// with `AccountBlobChangeSet` which operates solely on bytes.
    fn into_account_change_set(self) -> PartialVMResult<AccountChangeSet> {
        let (modules, resources) = self.into_inner();
        let mut resource_blobs = BTreeMap::new();
        for (struct_tag, op) in resources {
            let new_op = op.and_then(|resource| {
                resource
                    .serialize()
                    .ok_or_else(|| PartialVMError::new(StatusCode::INTERNAL_TYPE_ERROR))
            })?;
            resource_blobs.insert(struct_tag, new_op);
        }
        Ok(AccountChangeSet::from_modules_resources(
            modules,
            resource_blobs,
        ))
    }
}

/// A collection of non-serialized changes to the blockchain state.
#[derive(Default, Debug, Clone)]
pub struct ChangeSetV2 {
    accounts: BTreeMap<AccountAddress, AccountChangeSetV2>,
}

impl ChangeSetV2 {
    /// Creates an empty change set.
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
        }
    }

    /// Adds changes for specific account to this change set.
    pub fn add_account_changeset(
        &mut self,
        addr: AccountAddress,
        account_change_set: AccountChangeSetV2,
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

    /// Returns accounts with changes.
    pub fn accounts(&self) -> &BTreeMap<AccountAddress, AccountChangeSetV2> {
        &self.accounts
    }

    pub fn into_inner(self) -> BTreeMap<AccountAddress, AccountChangeSetV2> {
        self.accounts
    }

    fn get_or_insert_account_change_set(
        &mut self,
        addr: AccountAddress,
    ) -> &mut AccountChangeSetV2 {
        match self.accounts.entry(addr) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(AccountChangeSetV2::new()),
        }
    }

    pub fn add_module_op(&mut self, module_id: ModuleId, op: Op<Vec<u8>>) -> Result<()> {
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

    /// Converts all resources and modules in this change set to blobs. This ensures
    /// backwards compatibility with `BlobChangeSet` and legacy resolvers.
    pub fn into_change_set(self) -> VMResult<ChangeSet> {
        let accounts = self.into_inner();
        let mut change_set = ChangeSet::new();
        for (addr, account_change_set) in accounts {
            change_set
                .add_account_changeset(
                    addr,
                    account_change_set
                        .into_account_change_set()
                        .map_err(|e: PartialVMError| e.finish(Location::Undefined))?,
                )
                .expect("accounts should be unique");
        }
        Ok(change_set)
    }
}
