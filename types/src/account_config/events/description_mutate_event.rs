// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v1::MoveEventV1Type;
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
pub struct DescriptionMutateEvent {
    creator: AccountAddress,
    collection: String,
    token: String,
    old_description: String,
    new_description: String,
}

impl DescriptionMutateEvent {
    pub fn new(
        creator: AccountAddress,
        collection: String,
        token: String,
        old_description: String,
        new_description: String,
    ) -> Self {
        Self {
            creator,
            collection,
            token,
            old_description,
            new_description,
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

    pub fn old_description(&self) -> &String {
        &self.old_description
    }

    pub fn new_description(&self) -> &String {
        &self.new_description
    }
}

impl MoveStructType for DescriptionMutateEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("DescriptionMutateEvent");
}

impl MoveEventV1Type for DescriptionMutateEvent {}

pub static DESCRIPTION_MUTATE_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        name: ident_str!("DescriptionMutateEvent").to_owned(),
        type_args: vec![],
    }))
});
