// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};

pub use crate::aptos_framework_sdk_builder::*;
pub use crate::aptos_token_sdk_builder as aptos_token_stdlib;

pub fn aptos_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    coin_transfer(
        aptos_types::utility_coin::APTOS_COIN_TYPE.clone(),
        to,
        amount,
    )
}
