// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::account_config::CORE_CODE_ADDRESS;
use crate::event::EventKey;
use core::num::ParseIntError;
use move_deps::move_core_types::language_storage::TypeTag;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct TableHandle(pub u128);

impl FromStr for TableHandle {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let handle = u128::from_str(s)?;
        Ok(Self(handle))
    }
}

impl From<move_deps::move_table_extension::TableHandle> for TableHandle {
    fn from(hdl: move_deps::move_table_extension::TableHandle) -> Self {
        Self(hdl.0)
    }
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct TableInfo {
    pub key_type: TypeTag,
    pub value_type: TypeTag,
}

pub fn new_table_event_key() -> EventKey {
    EventKey::new(0, CORE_CODE_ADDRESS)
}

/// Rust representation of the on-chain NewTableEvent
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NewTableEvent {
    pub handle: u128,
    pub key_type: String,
    pub value_type: String,
}
