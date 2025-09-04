// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub use velor_api_types::deserialize_from_string;
use velor_api_types::{Address, U64};
use velor_types::transaction::authenticator::AuthenticationKey;
use move_core_types::{language_storage::StructTag, parser::parse_struct_tag};
use serde::{Deserialize, Deserializer, Serialize};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub struct Resource {
    #[serde(rename = "type", deserialize_with = "deserialize_resource_type")]
    pub resource_type: StructTag,
    pub data: serde_json::Value,
}

pub fn deserialize_from_prefixed_hex_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = <String>::deserialize(deserializer)?;
    s.trim_start_matches("0x")
        .parse::<T>()
        .map_err(D::Error::custom)
}

pub fn deserialize_resource_type<'de, D>(deserializer: D) -> Result<StructTag, D::Error>
where
    D: Deserializer<'de>,
{
    use serde::de::Error;

    let s = <String>::deserialize(deserializer)?;
    parse_struct_tag(&s).map_err(D::Error::custom)
}

#[derive(Clone, Debug, Deserialize, PartialEq)]
pub struct Account {
    #[serde(deserialize_with = "deserialize_from_prefixed_hex_string")]
    pub authentication_key: AuthenticationKey,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub sequence_number: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventHandle {
    counter: U64,
    guid: EventHandleGUID,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EventHandleGUID {
    len_bytes: u8,
    guid: GUID,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct GUID {
    id: ID,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ID {
    creation_num: U64,
    addr: Address,
}
