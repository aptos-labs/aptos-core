// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoinDeposit {
    coin_type: String,
    account: AccountAddress,
    amount: u64,
}

impl CoinDeposit {
    pub fn new(coin_type: String, account: AccountAddress, amount: u64) -> Self {
        Self {
            coin_type,
            account,
            amount,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn coin_type(&self) -> &str {
        &self.coin_type
    }

    pub fn account(&self) -> &AccountAddress {
        &self.account
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

pub const COIN_DEPOSIT_TYPE_STR: &str =
    "0000000000000000000000000000000000000000000000000000000000000001::coin::CoinDeposit";

pub static COIN_DEPOSIT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("coin").to_owned(),
        name: ident_str!("CoinDeposit").to_owned(),
        type_args: vec![],
    }))
});
