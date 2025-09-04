// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v1::MoveEventV1Type;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_OBJECTS_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct BurnEvent {
    index: u64,
    token: AccountAddress,
}

impl BurnEvent {
    pub fn new(index: u64, token: AccountAddress) -> Self {
        Self { index, token }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn index(&self) -> &u64 {
        &self.index
    }

    pub fn token(&self) -> &AccountAddress {
        &self.token
    }
}

impl MoveStructType for BurnEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("BurnEvent");
}

impl MoveEventV1Type for BurnEvent {}

pub static BURN_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_OBJECTS_ADDRESS,
        module: ident_str!("collection").to_owned(),
        name: ident_str!("BurnEvent").to_owned(),
        type_args: vec![],
    }))
});
