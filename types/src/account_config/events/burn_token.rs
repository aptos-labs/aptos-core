// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::TokenId, move_utils::move_event_v2::MoveEventV2Type};
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
pub struct BurnToken {
    account: AccountAddress,
    id: TokenId,
    amount: u64,
}

impl BurnToken {
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

impl MoveStructType for BurnToken {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Burn");
}

impl MoveEventV2Type for BurnToken {}

pub static BURN_TOKEN_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("Burn").to_owned(),
        type_args: vec![],
    }))
});
