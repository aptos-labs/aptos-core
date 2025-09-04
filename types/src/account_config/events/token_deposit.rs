// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenDeposit {
    account: AccountAddress,
    id: TokenId,
    amount: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenId {
    token_data_id: TokenDataId,
    property_version: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenDataId {
    creator: AccountAddress,
    collection: String,
    name: String,
}

impl TokenDeposit {
    pub fn new(account: AccountAddress, id: TokenId, amount: u64) -> Self {
        Self {
            account,
            id,
            amount,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn account(&self) -> &AccountAddress {
        &self.account
    }

    pub fn id(&self) -> &TokenId {
        &self.id
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for TokenDeposit {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TokenDeposit");
}

impl MoveEventV2Type for TokenDeposit {}

pub static TOKEN_DEPOSIT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("TokenDeposit").to_owned(),
        type_args: vec![],
    }))
});
