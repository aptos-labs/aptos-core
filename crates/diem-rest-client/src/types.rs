// Copyright (c) The Diem Core Contributors
// SPDX-License-Identifier: Apache-2.0

use diem_api_types::U64;
use diem_types::transaction::authenticator::AuthenticationKey;
use move_core_types::{language_storage::StructTag, parser::parse_struct_tag};
use serde::{Deserialize, Deserializer};
use std::str::FromStr;

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct RestError {
    pub code: u32,
    pub message: String,
    pub diem_ledger_version: Option<U64>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct Resource {
    #[serde(rename = "type", deserialize_with = "deserialize_resource_type")]
    pub resource_type: StructTag,
    pub data: serde_json::Value,
}

pub fn deserialize_from_string<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: FromStr,
    <T as FromStr>::Err: std::fmt::Display,
{
    use serde::de::Error;

    let s = <String>::deserialize(deserializer)?;
    s.parse::<T>().map_err(D::Error::custom)
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

#[derive(Clone, Debug, Deserialize)]
pub struct DiemAccount {
    #[serde(deserialize_with = "deserialize_from_prefixed_hex_string")]
    pub authentication_key: AuthenticationKey,
    #[serde(deserialize_with = "deserialize_from_string")]
    pub sequence_number: u64,
}
