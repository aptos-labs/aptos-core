// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

#![allow(unused_imports)]

include!(concat!(
    concat!(env!("OUT_DIR"), "/framework"),
    "/aptos_sdk_builder.rs",
));

pub fn aptos_coin_transfer(to: AccountAddress, amount: u64) -> TransactionPayload {
    coin_transfer(
        aptos_types::utility_coin::APTOS_COIN_TYPE.clone(),
        to,
        amount,
    )
}
