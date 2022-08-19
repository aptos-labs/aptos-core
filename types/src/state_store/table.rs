// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::{hash::HashValueParseError, HashValue};
use move_deps::move_core_types::language_storage::TypeTag;
use serde::{Deserialize, Serialize};
use std::{convert::TryInto, str::FromStr};

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct TableHandle {
    pub low: u128,
    pub high: u128,
}

impl TableHandle {
    pub fn size(&self) -> usize {
        std::mem::size_of_val(&self)
    }
}

impl FromStr for TableHandle {
    type Err = HashValueParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let hash = HashValue::from_str(s)?;
        Ok(hash.into())
    }
}

impl From<move_deps::move_table_extension::TableHandle> for TableHandle {
    fn from(hdl: move_deps::move_table_extension::TableHandle) -> Self {
        Self {
            low: hdl.low,
            high: hdl.high,
        }
    }
}

impl From<HashValue> for TableHandle {
    fn from(hash: HashValue) -> Self {
        let bytes = hash.to_vec();
        let low = u128::from_le_bytes(bytes[0..16].try_into().unwrap());
        let high = u128::from_le_bytes(bytes[16..32].try_into().unwrap());
        Self { low, high }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct TableInfo {
    pub key_type: TypeTag,
    pub value_type: TypeTag,
}
