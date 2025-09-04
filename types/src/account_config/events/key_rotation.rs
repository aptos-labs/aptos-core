// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyRotation {
    account: AccountAddress,
    old_authentication_key: Vec<u8>,
    new_authentication_key: Vec<u8>,
}

impl KeyRotation {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn account(&self) -> &AccountAddress {
        &self.account
    }

    pub fn old_authentication_key(&self) -> &Vec<u8> {
        &self.old_authentication_key
    }

    pub fn new_authentication_key(&self) -> &Vec<u8> {
        &self.new_authentication_key
    }
}

impl MoveStructType for KeyRotation {
    const MODULE_NAME: &'static IdentStr = ident_str!("account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("KeyRotation");
}

impl MoveEventV2Type for KeyRotation {}

pub static KEY_ROTATION_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("account").to_owned(),
        name: ident_str!("KeyRotation").to_owned(),
        type_args: vec![],
    }))
});
