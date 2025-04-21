// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CoinDeposit {
    pub coin_type: String,
    pub account: AccountAddress,
    pub amount: u64,
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

impl MoveStructType for CoinDeposit {
    const MODULE_NAME: &'static IdentStr = ident_str!("coin");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinDeposit");
}

impl MoveEventV2Type for CoinDeposit {}

pub static COIN_DEPOSIT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("coin").to_owned(),
        name: ident_str!("CoinDeposit").to_owned(),
        type_args: vec![],
    }))
});
