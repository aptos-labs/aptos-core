// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![forbid(unsafe_code)]

use crate::{build_information::collect_build_info, system_information::collect_system_info};
use std::collections::BTreeMap;

/// Inserts an optional value into the given map iff the value exists
pub(crate) fn insert_optional_value(
    map: &mut BTreeMap<String, String>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), value);
    }
}

/// Used to expose system and build information
pub fn get_system_and_build_information(chain_id: Option<String>) -> BTreeMap<String, String> {
    let mut information: BTreeMap<String, String> = BTreeMap::new();
    collect_build_info(chain_id, &mut information);
    collect_system_info(&mut information);
    information
}
