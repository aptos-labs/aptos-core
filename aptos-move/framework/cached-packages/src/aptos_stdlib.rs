// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

use aptos_types::transaction::authenticator::AuthenticationKey;
use aptos_types::{account_address::AccountAddress, transaction::TransactionPayload};

pub use crate::aptos_framework_sdk_builder::*;
pub use crate::aptos_token_sdk_builder as token_lib;

pub fn aptos_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    coin_transfer(
        aptos_types::utility_coin::APTOS_COIN_TYPE.clone(),
        to,
        amount,
    )
}

pub fn encode_create_resource_account(
    seed: &str,
    authentication_key: Option<AuthenticationKey>,
) -> TransactionPayload {
    let seed: Vec<u8> = bcs::to_bytes(seed).unwrap();
    let authentication_key: Vec<u8> = if let Some(key) = authentication_key {
        bcs::to_bytes(&key).unwrap()
    } else {
        vec![]
    };
    resource_account_create_resource_account(seed, authentication_key)
}
