// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{move_utils::as_move_value::AsMoveValue, on_chain_config::OnChainConfig};
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct RequiredGasDeposit {
    pub gas_amount: Option<u64>,
}

impl RequiredGasDeposit {
    pub fn default_for_genesis() -> Self {
        Self { gas_amount: None }
    }

    pub fn default_if_missing() -> Self {
        Self { gas_amount: None }
    }
}

impl OnChainConfig for RequiredGasDeposit {
    const MODULE_IDENTIFIER: &'static str = "randomness_api_v0_config";
    const TYPE_IDENTIFIER: &'static str = "RequiredGasDeposit";
}

impl AsMoveValue for RequiredGasDeposit {
    fn as_move_value(&self) -> MoveValue {
        match self.gas_amount {
            Some(gas_amount) => MoveValue::Struct(MoveStruct::RuntimeVariant(1, vec![MoveValue::U64(gas_amount)])),
            None => MoveValue::Struct(MoveStruct::RuntimeVariant(0, vec![])),
        }
    }
}

#[derive(Deserialize, Serialize)]
pub struct AllowCustomMaxGasFlag {
    pub value: bool,
}

impl AllowCustomMaxGasFlag {
    pub fn default_for_genesis() -> Self {
        Self { value: false }
    }

    pub fn default_if_missing() -> Self {
        Self { value: false }
    }
}

impl OnChainConfig for AllowCustomMaxGasFlag {
    const MODULE_IDENTIFIER: &'static str = "randomness_api_v0_config";
    const TYPE_IDENTIFIER: &'static str = "AllowCustomMaxGasFlag";
}

impl AsMoveValue for AllowCustomMaxGasFlag {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.value.as_move_value()]))
    }
}
