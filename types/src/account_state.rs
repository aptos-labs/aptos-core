// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::Path,
    account_config::{AccountResource, CoinStoreResource},
    account_view::AccountView,
    state_store::{state_key::StateKey, state_value::StateValue},
};
use anyhow::{anyhow, Error, Result};
use move_deps::move_core_types::language_storage::ModuleId;
use move_deps::move_core_types::{
    account_address::AccountAddress, language_storage::StructTag, move_resource::MoveResource,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{btree_map::BTreeMap, HashMap},
    convert::TryFrom,
    fmt,
};

#[derive(Clone, Deserialize, PartialEq, Eq, Serialize)]
pub struct AccountState {
    address: AccountAddress,
    data: BTreeMap<Vec<u8>, Vec<u8>>,
}

impl AccountState {
    pub fn new(address: AccountAddress, data: BTreeMap<Vec<u8>, Vec<u8>>) -> Self {
        Self { address, data }
    }

    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.data.get(key)
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        self.data.insert(key, value)
    }

    pub fn remove(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.data.remove(key)
    }

    pub fn iter(&self) -> impl std::iter::Iterator<Item = (&Vec<u8>, &Vec<u8>)> {
        self.data.iter()
    }

    pub fn into_resource_iter(self) -> impl std::iter::Iterator<Item = (Vec<u8>, Vec<u8>)> {
        self.data.into_iter()
    }

    /// Return an iterator over the module values stored under this account
    pub fn get_modules(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.data.iter().filter_map(|(k, v)| {
            match Path::try_from(k).expect("Invalid access path") {
                Path::Code(_) => Some(v),
                Path::Resource(_) => None,
            }
        })
    }

    /// Into an iterator over the module values stored under this account
    pub fn into_modules(self) -> impl Iterator<Item = (ModuleId, Vec<u8>)> {
        self.data.into_iter().filter_map(|(k, v)| {
            match Path::try_from(&k).expect("Invalid access path") {
                Path::Code(module) => Some((module, v)),
                Path::Resource(_) => None,
            }
        })
    }

    /// Return an iterator over all resources stored under this account.
    ///
    /// Note that resource access [`Path`]s that fail to deserialize will be
    /// silently ignored.
    pub fn get_resources(&self) -> impl Iterator<Item = (StructTag, &[u8])> {
        self.data
            .iter()
            .filter_map(|(k, v)| match Path::try_from(k) {
                Ok(Path::Resource(struct_tag)) => Some((struct_tag, v.as_ref())),
                Ok(Path::Code(_)) | Err(_) => None,
            })
    }

    pub fn from_access_paths_and_values(
        account_address: AccountAddress,
        key_value_map: &HashMap<StateKey, StateValue>,
    ) -> Result<Option<Self>> {
        if key_value_map.is_empty() {
            return Ok(None);
        }
        Some(Self::try_from((account_address, key_value_map))).transpose()
    }
}

impl fmt::Debug for AccountState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // TODO: add support for other types of resources
        let account_resource_str = self
            .get_account_resource()
            .map(|account_resource_opt| format!("{:#?}", account_resource_opt))
            .unwrap_or_else(|e| format!("parse error: {:#?}", e));

        write!(
            f,
            "{{ \n \
             AccountResource {{ {} }} \n \
             }}",
            account_resource_str,
        )
    }
}

impl AccountView for AccountState {
    fn get_state_value(&self, _: &StateKey) -> Result<Option<Vec<u8>>> {
        unimplemented!()
    }

    fn get_account_address(&self) -> anyhow::Result<Option<AccountAddress>> {
        Ok(Some(self.address))
    }

    fn get_resource_impl<T: DeserializeOwned>(&self, path: Vec<u8>) -> Result<Option<T>> {
        self.data
            .get(&path)
            .map(|bytes| bcs::from_bytes(bytes))
            .transpose()
            .map_err(Into::into)
    }
}

impl TryFrom<&StateValue> for AccountState {
    type Error = Error;

    fn try_from(state_value: &StateValue) -> Result<Self> {
        AccountState::try_from(state_value.bytes()).map_err(Into::into)
    }
}

impl TryFrom<&Vec<u8>> for AccountState {
    type Error = Error;

    fn try_from(blob: &Vec<u8>) -> Result<Self> {
        bcs::from_bytes(blob).map_err(Into::into)
    }
}

impl TryFrom<&[u8]> for AccountState {
    type Error = Error;

    fn try_from(blob: &[u8]) -> Result<Self> {
        bcs::from_bytes(blob).map_err(Into::into)
    }
}

impl TryFrom<(AccountAddress, &AccountResource, &CoinStoreResource)> for AccountState {
    type Error = Error;

    fn try_from(
        (account_address, account_resource, balance_resource): (
            AccountAddress,
            &AccountResource,
            &CoinStoreResource,
        ),
    ) -> Result<Self> {
        let mut btree_map: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        btree_map.insert(
            AccountResource::resource_path(),
            bcs::to_bytes(account_resource)?,
        );
        btree_map.insert(
            CoinStoreResource::resource_path(),
            bcs::to_bytes(balance_resource)?,
        );

        Ok(Self::new(account_address, btree_map))
    }
}

impl TryFrom<(AccountAddress, &HashMap<StateKey, StateValue>)> for AccountState {
    type Error = Error;

    fn try_from(
        (account_address, key_value_map): (AccountAddress, &HashMap<StateKey, StateValue>),
    ) -> Result<Self> {
        let mut btree_map: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        for (key, value) in key_value_map {
            match key {
                StateKey::AccessPath(access_path) => {
                    btree_map.insert(access_path.path.clone(), value.bytes().to_vec());
                }
                _ => return Err(anyhow!("Encountered unexpected key type {:?}", key)),
            }
        }
        Ok(Self::new(account_address, btree_map))
    }
}
