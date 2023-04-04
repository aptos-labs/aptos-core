// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use aptos_types::access_path::AccessPath;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::{ModuleId, StructTag},
};
use std::collections::btree_map::{self, BTreeMap};

pub trait AccessPathCache {
    fn get_module_path(&mut self, module_id: ModuleId) -> AccessPath;
    fn get_resource_path(&mut self, address: AccountAddress, struct_tag: StructTag) -> AccessPath;
    fn get_resource_group_path(
        &mut self,
        address: AccountAddress,
        struct_tag: StructTag,
    ) -> AccessPath;
}

impl AccessPathCache for () {
    fn get_module_path(&mut self, module_id: ModuleId) -> AccessPath {
        AccessPath::from(&module_id)
    }

    fn get_resource_path(&mut self, address: AccountAddress, struct_tag: StructTag) -> AccessPath {
        AccessPath::resource_access_path(address, struct_tag)
            .unwrap_or_else(|_| AccessPath::undefined())
    }

    fn get_resource_group_path(
        &mut self,
        address: AccountAddress,
        struct_tag: StructTag,
    ) -> AccessPath {
        AccessPath::resource_group_access_path(address, struct_tag)
    }
}

#[derive(Clone)]
pub struct BTreeAccessPathCache {
    modules: BTreeMap<ModuleId, Vec<u8>>,
    resources: BTreeMap<StructTag, Vec<u8>>,
}

impl AccessPathCache for BTreeAccessPathCache {
    fn get_module_path(&mut self, module_id: ModuleId) -> AccessPath {
        let addr = *module_id.address();
        let access_vec = match self.modules.entry(module_id) {
            btree_map::Entry::Vacant(entry) => {
                let module_id = entry.key().clone();
                entry.insert(AccessPath::code_path_vec(module_id)).clone()
            },
            btree_map::Entry::Occupied(entry) => entry.get().clone(),
        };
        AccessPath::new(addr, access_vec)
    }

    fn get_resource_path(&mut self, address: AccountAddress, struct_tag: StructTag) -> AccessPath {
        let access_vec = match self.resources.entry(struct_tag) {
            btree_map::Entry::Vacant(entry) => {
                let struct_tag = entry.key().clone();
                entry
                    .insert(AccessPath::resource_path_vec(struct_tag).unwrap_or_default())
                    .clone()
            },
            btree_map::Entry::Occupied(entry) => entry.get().clone(),
        };
        AccessPath::new(address, access_vec)
    }

    fn get_resource_group_path(
        &mut self,
        address: AccountAddress,
        struct_tag: StructTag,
    ) -> AccessPath {
        let access_vec = match self.resources.entry(struct_tag) {
            btree_map::Entry::Vacant(entry) => {
                let struct_tag = entry.key().clone();
                entry
                    .insert(AccessPath::resource_group_path_vec(struct_tag))
                    .clone()
            },
            btree_map::Entry::Occupied(entry) => entry.get().clone(),
        };
        AccessPath::new(address, access_vec)
    }
}

impl BTreeAccessPathCache {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
            resources: BTreeMap::new(),
        }
    }
}
