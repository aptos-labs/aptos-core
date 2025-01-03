// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v1::MoveEventV1Type;
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
pub struct OptInTransferEvent {
    opt_in: bool,
}

impl OptInTransferEvent {
    pub fn new(opt_in: bool) -> Self {
        Self { opt_in }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn opt_in(&self) -> &bool {
        &self.opt_in
    }
}

impl MoveStructType for OptInTransferEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("OptInTransferEvent");
}

impl MoveEventV1Type for OptInTransferEvent {}

pub static OPT_IN_TRANSFER_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        name: ident_str!("OptInTransferEvent").to_owned(),
        type_args: vec![],
    }))
});
