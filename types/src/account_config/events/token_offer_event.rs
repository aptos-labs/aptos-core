// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::TokenId, move_utils::move_event_v1::MoveEventV1Type};
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
pub struct TokenOfferEvent {
    to_address: AccountAddress,
    token_id: TokenId,
    amount: u64,
}

impl TokenOfferEvent {
    pub fn new(to_address: AccountAddress, token_id: TokenId, amount: u64) -> Self {
        Self {
            to_address,
            token_id,
            amount,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
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

impl MoveStructType for TokenOfferEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_transfers");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TokenOfferEvent");
}

impl MoveEventV1Type for TokenOfferEvent {}

pub static TOKEN_OFFER_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_transfers").to_owned(),
        name: ident_str!("TokenOfferEvent").to_owned(),
        type_args: vec![],
    }))
});
