// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{move_utils::as_move_value::AsMoveValue, on_chain_config::OnChainConfig};
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequiredDeposit {
    pub amount: Option<u64>,
}

impl RequiredDeposit {
    pub fn default_for_genesis() -> Self {
        Self {
            amount: Some(1_000_000), // in octa, which is 0.01 APT
        }
    }

    pub fn default_if_missing() -> Self {
        Self { amount: None }
    }
}

impl OnChainConfig for RequiredDeposit {
    const MODULE_IDENTIFIER: &'static str = "randomness_api_v0_config";
    const TYPE_IDENTIFIER: &'static str = "RequiredDeposit";
}

impl AsMoveValue for RequiredDeposit {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.amount.as_move_value()]))
    }
}
