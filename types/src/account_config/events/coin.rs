// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::TypeInfoResource,
    move_utils::{move_event_v1::MoveEventV1Type, move_event_v2::MoveEventV2Type},
};
use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};
use serde::{Deserialize, Serialize};

/// Struct that represents a SentPaymentEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawEvent {
    pub amount: u64,
}

impl WithdrawEvent {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    /// Get the amount sent or received
    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for WithdrawEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("WithdrawEvent");
}

impl MoveEventV1Type for WithdrawEvent {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinWithdraw {
    pub coin_type: String,
    pub account: AccountAddress,
    pub amount: u64,
}

impl CoinWithdraw {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

impl MoveStructType for CoinWithdraw {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinWithdraw");
}

impl MoveEventV2Type for CoinWithdraw {}

/// Struct that represents a DepositPaymentEvent.
#[derive(Debug, Serialize, Deserialize)]
pub struct DepositEvent {
    pub amount: u64,
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
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DepositEvent");
}

impl MoveEventV1Type for DepositEvent {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinDeposit {
    pub coin_type: String,
    pub account: AccountAddress,
    pub amount: u64,
}

impl CoinDeposit {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

impl MoveStructType for CoinDeposit {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinDeposit");
}

impl MoveEventV2Type for CoinDeposit {}

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinRegister {
    pub account: AccountAddress,
    pub type_info: TypeInfoResource,
}

impl MoveStructType for CoinRegister {
    const MODULE_NAME: &'static IdentStr = ident_str!("account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinRegister");
}

impl MoveEventV2Type for CoinRegister {}
