// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{account_config::TokenId, move_utils::move_event_v1::MoveEventV1Type};
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
pub struct MutateTokenPropertyMapEvent {
    old_id: TokenId,
    new_id: TokenId,
    keys: Vec<String>,
    values: Vec<Vec<u8>>,
    types: Vec<String>,
}

impl MutateTokenPropertyMapEvent {
    pub fn new(
        old_id: TokenId,
        new_id: TokenId,
        keys: Vec<String>,
        values: Vec<Vec<u8>>,
        types: Vec<String>,
    ) -> Self {
        Self {
            old_id,
            new_id,
            keys,
            values,
            types,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn old_id(&self) -> &TokenId {
        &self.old_id
    }

    pub fn new_id(&self) -> &TokenId {
        &self.new_id
    }

    pub fn keys(&self) -> &Vec<String> {
        &self.keys
    }

    pub fn values(&self) -> &Vec<Vec<u8>> {
        &self.values
    }

    pub fn types(&self) -> &Vec<String> {
        &self.types
    }
}

impl MoveStructType for MutateTokenPropertyMapEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MutateTokenPropertyMapEvent");
}

impl MoveEventV1Type for MutateTokenPropertyMapEvent {}

pub static MUTATE_TOKEN_PROPERTY_MAP_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("MutateTokenPropertyMapEvent").to_owned(),
        type_args: vec![],
    }))
});
