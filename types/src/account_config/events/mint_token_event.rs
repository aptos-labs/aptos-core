// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::TokenDataId, move_utils::move_event_v1::MoveEventV1Type};
use anyhow::Result;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct MintTokenEvent {
    id: TokenDataId,
    amount: u64,
}

impl MintTokenEvent {
    pub fn new(id: TokenDataId, amount: u64) -> Self {
        Self { id, amount }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn id(&self) -> &TokenDataId {
        &self.id
    }

    pub fn amount(&self) -> u64 {
        self.amount
    }
}

impl MoveStructType for MintTokenEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MintTokenEvent");
}

impl MoveEventV1Type for MintTokenEvent {}

pub static MINT_TOKEN_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("MintTokenEvent").to_owned(),
        type_args: vec![],
    }))
});
