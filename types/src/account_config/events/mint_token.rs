// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::TokenDataId, move_utils::move_event_v2::MoveEventV2Type};
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
pub struct MintToken {
    creator: AccountAddress,
    id: TokenDataId,
    amount: u64,
}

impl MintToken {
    pub fn new(creator: AccountAddress, id: TokenDataId, amount: u64) -> Self {
        Self {
            creator,
            id,
            amount,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn creator(&self) -> &AccountAddress {
        &self.creator
    }

    pub fn id(&self) -> &TokenDataId {
        &self.id
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for MintToken {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Mint");
}

impl MoveEventV2Type for MintToken {}

pub static MINT_TOKEN_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("Mint").to_owned(),
        type_args: vec![],
    }))
});
