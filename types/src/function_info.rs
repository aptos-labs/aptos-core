// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::{as_move_value::AsMoveValue, MemberId};
use move_core_types::{
    account_address::AccountAddress,
    value::{MoveStruct, MoveValue},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    str::FromStr,
};

/// Reflection of velor_framework::function_info::FunctionInfo
#[derive(Serialize, Deserialize, Eq, PartialEq, Debug, Clone, Hash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct FunctionInfo {
    pub module_address: AccountAddress,
    pub module_name: String,
    pub function_name: String,
}

impl FunctionInfo {
    pub fn new(module_address: AccountAddress, module_name: String, function_name: String) -> Self {
        Self {
            module_address,
            module_name,
            function_name,
        }
    }
}

impl Display for FunctionInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}::{}::{}",
            self.module_address.to_hex(),
            self.module_name,
            self.function_name
        )?;
        Ok(())
    }
}

impl From<MemberId> for FunctionInfo {
    fn from(value: MemberId) -> Self {
        Self::new(
            value.module_id.address,
            value.module_id.name.into_string(),
            value.member_id.into_string(),
        )
    }
}

impl FromStr for FunctionInfo {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(MemberId::from_str(s)?.into())
    }
}

impl AsMoveValue for FunctionInfo {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            MoveValue::Address(self.module_address),
            self.module_name.as_move_value(),
            self.function_name.as_move_value(),
        ]))
    }
}
