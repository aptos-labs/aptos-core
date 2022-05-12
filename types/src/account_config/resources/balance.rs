// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use move_deps::move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};

/// The balance resource held under an account.
#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct BalanceResource {
    coin: u64,
}

impl BalanceResource {
    pub fn new(coin: u64) -> Self {
        Self { coin }
    }

    pub fn coin(&self) -> u64 {
        self.coin
    }
}

impl MoveStructType for BalanceResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("TestCoin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Balance");
}

impl MoveResource for BalanceResource {}
