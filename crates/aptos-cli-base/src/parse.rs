// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::types::{CliError, CliTypedResult};
use itertools::Itertools;
use serde::de::DeserializeOwned;
use serde::Serialize;
use std::collections::BTreeMap;
use std::str::FromStr;

/// Error message for parsing a map
const PARSE_MAP_SYNTAX_MSG: &str = "Invalid syntax for map. Example: Name=Value,Name2=Value";

/// Parses an inline map of values
///
/// Example: Name=Value,Name2=Value
pub fn parse_map<K: FromStr + Ord, V: FromStr>(str: &str) -> CliTypedResult<BTreeMap<K, V>>
where
    K::Err: 'static + std::error::Error + Send + Sync,
    V::Err: 'static + std::error::Error + Send + Sync,
{
    let mut map = BTreeMap::new();

    // Split pairs by commas
    for pair in str.split_terminator(',') {
        // Split pairs by = then trim off any spacing
        let (first, second): (&str, &str) = pair
            .split_terminator('=')
            .collect_tuple()
            .ok_or_else(|| CliError::CommandArgumentError(PARSE_MAP_SYNTAX_MSG.to_string()))?;
        let first = first.trim();
        let second = second.trim();
        if first.is_empty() || second.is_empty() {
            return Err(CliError::CommandArgumentError(
                PARSE_MAP_SYNTAX_MSG.to_string(),
            ));
        }

        // At this point, we just give error messages appropriate to parsing
        let key: K =
            K::from_str(first).map_err(|err| CliError::CommandArgumentError(err.to_string()))?;
        let value: V =
            V::from_str(second).map_err(|err| CliError::CommandArgumentError(err.to_string()))?;
        map.insert(key, value);
    }
    Ok(map)
}

pub fn to_yaml<T: Serialize + ?Sized>(input: &T) -> CliTypedResult<String> {
    Ok(serde_yaml::to_string(input)?)
}

pub fn from_yaml<T: DeserializeOwned>(input: &str) -> CliTypedResult<T> {
    Ok(serde_yaml::from_str(input)?)
}
