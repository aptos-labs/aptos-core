// Copyright © Velor Foundation
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
pub struct Offer {
    account: AccountAddress,
    to_address: AccountAddress,
    token_id: TokenId,
    amount: u64,
}

impl Offer {
    pub fn new(
        account: AccountAddress,
        to_address: AccountAddress,
        token_id: TokenId,
        amount: u64,
    ) -> Self {
        Self {
            account,
            to_address,
            token_id,
            amount,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn account(&self) -> &AccountAddress {
        &self.account
    }

    pub fn to_address(&self) -> &AccountAddress {
        &self.to_address
    }

    pub fn token_id(&self) -> &TokenId {
        &self.token_id
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for Offer {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_transfers");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Offer");
}

impl MoveEventV2Type for Offer {}

pub static OFFER_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_transfers").to_owned(),
        name: ident_str!("Offer").to_owned(),
        type_args: vec![],
    }))
});
