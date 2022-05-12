// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::account_address::AccountAddress;
use anyhow::Result;
use move_deps::move_core_types::{ident_str, identifier::IdentStr, move_resource::MoveStructType};
use serde::{Deserialize, Serialize};

/// Struct that represents a ReceivedPaymentEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct ReceivedEvent {
    amount: u64,
    sender: AccountAddress,
}

impl ReceivedEvent {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    /// Get the receiver of this transaction event.
    pub fn sender(&self) -> AccountAddress {
        self.sender
    }

    /// Get the amount sent or received
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for ReceivedEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("TestCoin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ReceivedEvent");
}
