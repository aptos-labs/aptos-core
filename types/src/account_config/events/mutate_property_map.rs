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
pub struct MutatePropertyMap {
    account: AccountAddress,
    old_id: TokenId,
    new_id: TokenId,
    keys: Vec<String>,
    values: Vec<Vec<u8>>,
    types: Vec<String>,
}

impl MutatePropertyMap {
    pub fn new(
        account: AccountAddress,
        old_id: TokenId,
        new_id: TokenId,
        keys: Vec<String>,
        values: Vec<Vec<u8>>,
        types: Vec<String>,
    ) -> Self {
        Self {
            account,
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

    pub fn account(&self) -> &AccountAddress {
        &self.account
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

impl MoveStructType for MutatePropertyMap {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("MutatePropertyMap");
}

impl MoveEventV2Type for MutatePropertyMap {}

pub static MUTATE_PROPERTY_MAP_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("MutatePropertyMap").to_owned(),
        type_args: vec![],
    }))
});
