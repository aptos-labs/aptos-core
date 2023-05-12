// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use crate::types::Store;
use anyhow::bail;
use move_binary_format::errors::{Location, PartialVMError, PartialVMResult, VMResult};
use move_core_types::{
    account_address::AccountAddress,
    effects::{
        AccountChangeSet as SerializedAccountChangeSet, ChangeSet as SerializedChangeSet, Op,
    },
    identifier::Identifier,
    language_storage::StructTag,
    vm_status::StatusCode,
};
use std::collections::{btree_map::Entry, BTreeMap};

/// A collection of resource and module operations on a Move account.
#[derive(Default, Debug, Clone)]
pub struct AccountChangeSet<M: Store, R: Store> {
    modules: BTreeMap<Identifier, Op<M>>,
    resources: BTreeMap<StructTag, Op<R>>,
}

impl<M: Store, R: Store> AccountChangeSet<M, R> {
    pub fn new(
        modules: BTreeMap<Identifier, Op<M>>,
        resources: BTreeMap<StructTag, Op<R>>,
    ) -> Self {
        Self { modules, resources }
    }

    pub fn into_inner(self) -> (BTreeMap<Identifier, Op<M>>, BTreeMap<StructTag, Op<R>>) {
        (self.modules, self.resources)
    }

    fn into_serialized_account_change_set(self) -> PartialVMResult<SerializedAccountChangeSet> {
        let (modules, resources) = self.into_inner();
        macro_rules! into_bytes {
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

        let (mut module_bytes, mut resource_bytes) = (BTreeMap::new(), BTreeMap::new());
        into_bytes!(module_bytes, modules);
        into_bytes!(resource_bytes, resources);

        Ok(SerializedAccountChangeSet::from_modules_resources(
            module_bytes,
            resource_bytes,
        ))
    }
}

/// A collection of changes to a Move state.
#[derive(Default, Debug, Clone)]
pub struct ChangeSet<M: Store, R: Store> {
    accounts: BTreeMap<AccountAddress, AccountChangeSet<M, R>>,
}

impl<M: Store, R: Store> ChangeSet<M, R> {
    pub fn new() -> Self {
        Self {
            accounts: BTreeMap::new(),
        }
    }

    pub fn into_inner(self) -> BTreeMap<AccountAddress, AccountChangeSet<M, R>> {
        self.accounts
    }

    pub fn add_account_change_set(
        &mut self,
        addr: AccountAddress,
        account_change_set: AccountChangeSet<M, R>,
    ) -> anyhow::Result<()> {
        match self.accounts.entry(addr) {
            Entry::Occupied(_) => bail!("Account {} already exists.", addr),
            Entry::Vacant(e) => {
                e.insert(account_change_set);
            },
        }
        Ok(())
    }

    pub fn into_serialized_change_set(self) -> VMResult<SerializedChangeSet> {
        let accounts = self.into_inner();
        let mut change_set = SerializedChangeSet::new();
        for (addr, account_change_set) in accounts {
            change_set
                .add_account_changeset(
                    addr,
                    account_change_set
                        .into_serialized_account_change_set()
                        .map_err(|e: PartialVMError| e.finish(Location::Undefined))?,
                )
                .expect("accounts should be unique");
        }
        Ok(change_set)
    }
}
