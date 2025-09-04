// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
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
pub struct DefaultPropertyMutate {
    creator: AccountAddress,
    collection: String,
    token: String,
    keys: Vec<String>,
    old_values: Vec<OptionType<PropertyValue>>,
    new_values: Vec<PropertyValue>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OptionType<T> {
    vec: Vec<T>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PropertyValue {
    value: Vec<u8>,
    typ: String,
}

impl DefaultPropertyMutate {
    pub fn new(
        creator: AccountAddress,
        collection: String,
        token: String,
        keys: Vec<String>,
        old_values: Vec<OptionType<PropertyValue>>,
        new_values: Vec<PropertyValue>,
    ) -> Self {
        Self {
            creator,
            collection,
            token,
            keys,
            old_values,
            new_values,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn creator(&self) -> &AccountAddress {
        &self.creator
    }

    pub fn collection(&self) -> &String {
        &self.collection
    }

    pub fn token(&self) -> &String {
        &self.token
    }

    pub fn keys(&self) -> &Vec<String> {
        &self.keys
    }

    pub fn old_values(&self) -> &Vec<OptionType<PropertyValue>> {
        &self.old_values
    }

    pub fn new_values(&self) -> &Vec<PropertyValue> {
        &self.new_values
    }
}

impl MoveStructType for DefaultPropertyMutate {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DefaultPropertyMutate");
}

impl MoveEventV2Type for DefaultPropertyMutate {}

pub static DEFAULT_PROPERTY_MUTATE_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        name: ident_str!("DefaultPropertyMutate").to_owned(),
        type_args: vec![],
    }))
});
