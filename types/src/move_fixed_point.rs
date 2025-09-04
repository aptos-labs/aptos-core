// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use fixed::types::U64F64;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub struct FixedPoint64MoveStruct {
    value: u128,
}

impl FixedPoint64MoveStruct {
    pub fn as_u64f64(&self) -> U64F64 {
        U64F64::from_bits(self.value)
    }

    pub fn from_u64f64(arg: U64F64) -> Self {
        Self {
            value: arg.to_bits(),
        }
    }
}
