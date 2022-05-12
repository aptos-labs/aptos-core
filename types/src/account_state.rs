// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    access_path::Path,
    account_address::AccountAddress,
    account_config::{AccountResource, BalanceResource},
    account_view::AccountView,
    state_store::{state_key::StateKey, state_value::StateValue},
};
use anyhow::{anyhow, Error, Result};
use move_deps::move_core_types::{language_storage::StructTag, move_resource::MoveResource};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    collections::{btree_map::BTreeMap, HashMap},
    convert::TryFrom,
    fmt,
};

#[derive(Clone, Default, Deserialize, PartialEq, Serialize)]
pub struct AccountState(BTreeMap<Vec<u8>, Vec<u8>>);

impl AccountState {
    pub fn get(&self, key: &[u8]) -> Option<&Vec<u8>> {
        self.0.get(key)
    }

    pub fn insert(&mut self, key: Vec<u8>, value: Vec<u8>) -> Option<Vec<u8>> {
        self.0.insert(key, value)
    }

    pub fn remove(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        self.0.remove(key)
    }

    pub fn iter(&self) -> impl std::iter::Iterator<Item = (&Vec<u8>, &Vec<u8>)> {
        self.0.iter()
    }

    /// Return an iterator over the module values stored under this account
    pub fn get_modules(&self) -> impl Iterator<Item = &Vec<u8>> {
        self.0.iter().filter_map(
            |(k, v)| match Path::try_from(k).expect("Invalid access path") {
                Path::Code(_) => Some(v),
                Path::Resource(_) => None,
            },
        )
    }

    /// Into an iterator over the module values stored under this account
    pub fn into_modules(self) -> impl Iterator<Item = Vec<u8>> {
        self.0.into_iter().filter_map(|(k, v)| {
            match Path::try_from(&k).expect("Invalid access path") {
                Path::Code(_) => Some(v),
                Path::Resource(_) => None,
            }
        })
    }

    /// Return an iterator over all resources stored under this account.
    ///
    /// Note that resource access [`Path`]s that fail to deserialize will be
    /// silently ignored.
    pub fn get_resources(&self) -> impl Iterator<Item = (StructTag, &[u8])> {
        self.0.iter().filter_map(|(k, v)| match Path::try_from(k) {
            Ok(Path::Resource(struct_tag)) => Some((struct_tag, v.as_ref())),
            Ok(Path::Code(_)) | Err(_) => None,
        })
    }

    pub fn from_access_paths_and_values(
        key_value_map: &HashMap<StateKey, StateValue>,
    ) -> Result<Option<Self>> {
        if key_value_map.is_empty() {
            return Ok(None);
        }
        Some(Self::try_from(key_value_map)).transpose()
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
        match self.get_resource::<AccountResource>()? {
            x @ Some(_) => Ok(x),
            None => Ok(None),
        }
        .map(|opt_ar| opt_ar.map(|ar| ar.address()))
    }

    fn get_resource_impl<T: DeserializeOwned>(&self, path: Vec<u8>) -> Result<Option<T>> {
        self.0
            .get(&path)
            .map(|bytes| bcs::from_bytes(bytes))
            .transpose()
            .map_err(Into::into)
    }
}

impl TryFrom<&StateValue> for AccountState {
    type Error = Error;

    fn try_from(state_value: &StateValue) -> Result<Self> {
        let bytes = state_value
            .maybe_bytes
            .as_ref()
            .ok_or_else(|| anyhow!("Empty state value passed"))?;

        AccountState::try_from(bytes).map_err(Into::into)
    }
}

impl TryFrom<&Vec<u8>> for AccountState {
    type Error = Error;

    fn try_from(blob: &Vec<u8>) -> Result<Self> {
        bcs::from_bytes(blob).map_err(Into::into)
    }
}

impl TryFrom<(&AccountResource, &BalanceResource)> for AccountState {
    type Error = Error;

    fn try_from(
        (account_resource, balance_resource): (&AccountResource, &BalanceResource),
    ) -> Result<Self> {
        let mut btree_map: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        btree_map.insert(
            AccountResource::resource_path(),
            bcs::to_bytes(account_resource)?,
        );
        btree_map.insert(
            BalanceResource::resource_path(),
            bcs::to_bytes(balance_resource)?,
        );

        Ok(Self(btree_map))
    }
}

impl TryFrom<&HashMap<StateKey, StateValue>> for AccountState {
    type Error = Error;

    fn try_from(key_value_map: &HashMap<StateKey, StateValue>) -> Result<Self> {
        let mut btree_map: BTreeMap<Vec<u8>, Vec<u8>> = BTreeMap::new();
        for (key, value) in key_value_map {
            match key {
                StateKey::AccessPath(access_path) => {
                    if let Some(bytes) = &value.maybe_bytes {
                        btree_map.insert(access_path.path.clone(), bytes.clone());
                    }
                }
                _ => return Err(anyhow!("Encountered unexpected key type {:?}", key)),
            }
        }
        Ok(Self(btree_map))
    }
}
