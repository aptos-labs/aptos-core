// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use move_core_types::{
    account_address::AccountAddress, ident_str, identifier::IdentStr, move_resource::MoveStructType,
};
use serde::{Deserialize, Serialize};

/// Struct that represents a Withdraw event.
#[derive(Debug, Serialize, Deserialize)]
pub struct WithdrawFAEvent {
    pub store: AccountAddress,
    pub amount: u64,
}

impl MoveEventV2Type for WithdrawFAEvent {}

impl MoveStructType for WithdrawFAEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("fungible_asset");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Withdraw");
}

/// Struct that represents a Deposit event.
#[derive(Debug, Serialize, Deserialize)]
pub struct DepositFAEvent {
    pub store: AccountAddress,
    pub amount: u64,
}

impl MoveEventV2Type for DepositFAEvent {}

impl MoveStructType for DepositFAEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("fungible_asset");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Deposit");
}
