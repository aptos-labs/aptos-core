// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::move_core_types::{
    account_address::{AccountAddress, AccountAddressParseError},
    language_storage::TypeTag,
};
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct TableHandle(pub AccountAddress);

impl TableHandle {
    pub fn size(&self) -> usize {
        std::mem::size_of_val(&self.0)
    }
}

impl FromStr for TableHandle {
    type Err = AccountAddressParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let handle = AccountAddress::from_str(s)?;
        Ok(Self(handle))
    }
}

impl From<move_deps::move_table_extension::TableHandle> for TableHandle {
    fn from(hdl: move_deps::move_table_extension::TableHandle) -> Self {
        Self(hdl.0)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(proptest_derive::Arbitrary))]
pub struct TableInfo {
    pub key_type: TypeTag,
    pub value_type: TypeTag,
}
