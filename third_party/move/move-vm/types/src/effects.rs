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

    /// Returns a serialized version of account change set.
    fn into_serialized_account_change_set(self) -> PartialVMResult<SerializedAccountChangeSet> {
        let (module_ops, resource_ops) = self.into_inner();

        // Serialize modules.
        let mut serialized_module_ops = BTreeMap::new();
        for (identifier, module_op) in module_ops {
            let blob_op = module_op.and_then(|m| m.into_bytes())?;
            serialized_module_ops.insert(identifier, blob_op);
        }

        // Serialize resources.
        let mut serialized_resource_ops = BTreeMap::new();
        for (tag, resource_op) in resource_ops {
            let blob_op = resource_op.and_then(|r| r.into_bytes())?;
            serialized_resource_ops.insert(tag, blob_op);
        }

        Ok(SerializedAccountChangeSet::from_modules_resources(
            serialized_module_ops,
            serialized_resource_ops,
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

    /// Returns a serialized version of a change set.
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
                .expect("All accounts should be unique.");
        }
        Ok(change_set)
    }
}
