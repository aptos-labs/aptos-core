// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_core_types::move_resource::MoveStructType;
use serde::{Deserialize, Serialize};

/// Struct that represents a DepositPaymentEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct DepositEvent {
    amount: u64,
}

impl DepositEvent {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    /// Get the amount sent or received
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for DepositEvent {
    const MODULE_NAME: &'static str = "coin";
    const STRUCT_NAME: &'static str = "DepositEvent";
}
