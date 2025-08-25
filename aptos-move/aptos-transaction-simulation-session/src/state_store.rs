// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::bail;
use aptos_types::{
    access_path::Path,
    state_store::{
        state_key::{inner::StateKeyInner, StateKey},
        table::TableHandle,
    },
};
use move_core_types::{
    account_address::AccountAddress, identifier::IdentStr, language_storage::StructTag,
};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::str::FromStr;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HumanReadable<T>(pub T);

impl<'a> Serialize for HumanReadable<&'a StateKey> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as hex string
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for HumanReadable<StateKey> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::from_str(&String::deserialize(deserializer)?).map_err(serde::de::Error::custom)
    }
}

impl<T> HumanReadable<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<'a> std::fmt::Display for HumanReadable<&'a StateKey> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.inner() {
            StateKeyInner::AccessPath(access_path) => {
                let mut s = String::new();

                let header;

                s.push_str(&format!("{}::", access_path.address));
                match access_path.get_path() {
                    Path::Code(module_id) => {
                        header = "code";
                        s.push_str(&format!("{}", module_id.name));
                    },
                    Path::Resource(struct_tag) => {
                        header = "resource";
                        s.push_str(&struct_tag.to_canonical_string());
                    },
                    Path::ResourceGroup(struct_tag) => {
                        header = "resource_group";
                        s.push_str(&struct_tag.to_canonical_string());
                    },
                }

                write!(f, "{}::{}", header, s)
            },
            StateKeyInner::TableItem { handle, key } => {
                write!(f, "table_item::{}::{}", handle.0, hex::encode(key))
            },
            StateKeyInner::Raw(bytes) => write!(f, "raw::{}", hex::encode(bytes)),
        }
    }
}

impl FromStr for HumanReadable<StateKey> {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split("::").collect();

        match parts.as_slice() {
            ["code", address, name] => {
                let address = AccountAddress::from_str(address)?;
                Ok(HumanReadable(StateKey::module(
                    &address,
                    IdentStr::new(name)?,
                )))
            },
            ["resource", address, module_address, module_name, rest @ ..] => {
                let address = AccountAddress::from_str(address)?;

                let struct_tag = StructTag::from_str(&format!(
                    "{}::{}::{}",
                    module_address,
                    module_name,
                    rest.join("::")
                ))?;

                Ok(HumanReadable(StateKey::resource(&address, &struct_tag)?))
            },
            ["resource_group", address, module_address, module_name, rest @ ..] => {
                let address = AccountAddress::from_str(address)?;

                let struct_tag = StructTag::from_str(&format!(
                    "{}::{}::{}",
                    module_address,
                    module_name,
                    rest.join("::")
                ))?;

                Ok(HumanReadable(StateKey::resource_group(
                    &address,
                    &struct_tag,
                )))
            },
            ["table_item", handle, key] => {
                let handle = TableHandle(AccountAddress::from_str(handle)?);
                let key = hex::decode(key)?;
                Ok(HumanReadable(StateKey::table_item(&handle, &key)))
            },
            ["raw", bytes] => {
                let bytes = hex::decode(bytes)?;
                Ok(HumanReadable(StateKey::raw(&bytes)))
            },
            _ => bail!("Unknown StateKey format: {}", s),
        }
    }
}
