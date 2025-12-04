// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::move_utils::move_event_v1::MoveEventV1Type;
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
pub struct KeyRotationEvent {
    old_authentication_key: Vec<u8>,
    new_authentication_key: Vec<u8>,
}

impl KeyRotationEvent {
    pub fn new(old_authentication_key: Vec<u8>, new_authentication_key: Vec<u8>) -> Self {
        Self {
            old_authentication_key,
            new_authentication_key,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn old_authentication_key(&self) -> &Vec<u8> {
        &self.old_authentication_key
    }

    pub fn new_authentication_key(&self) -> &Vec<u8> {
        &self.new_authentication_key
    }
}

impl MoveStructType for KeyRotationEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("account");
    const STRUCT_NAME: &'static IdentStr = ident_str!("KeyRotationEvent");
}

impl MoveEventV1Type for KeyRotationEvent {}

pub static KEY_ROTATION_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("account").to_owned(),
        name: ident_str!("KeyRotationEvent").to_owned(),
        type_args: vec![],
    }))
});
