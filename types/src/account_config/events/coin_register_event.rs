// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::TypeInfoResource, move_utils::move_event_v1::MoveEventV1Type};
use anyhow::Result;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CoinRegisterEvent {
    type_info: TypeInfoResource,
}

impl CoinRegisterEvent {
    pub fn new(type_info: TypeInfoResource) -> Self {
        Self { type_info }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

impl MoveStructType for CoinRegisterEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CoinRegisterEvent");
}

impl MoveEventV1Type for CoinRegisterEvent {}

pub static COIN_REGISTER_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("account").to_owned(),
        name: ident_str!("CoinRegisterEvent").to_owned(),
        type_args: vec![],
    }))
});
