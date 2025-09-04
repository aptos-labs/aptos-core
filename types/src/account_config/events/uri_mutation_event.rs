// Copyright © Velor Foundation
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
pub struct UriMutationEvent {
    creator: AccountAddress,
    collection: String,
    token: String,
    old_uri: String,
    new_uri: String,
}

impl UriMutationEvent {
    pub fn new(
        creator: AccountAddress,
        collection: String,
        token: String,
        old_uri: String,
        new_uri: String,
    ) -> Self {
        Self {
            creator,
            collection,
            token,
            old_uri,
            new_uri,
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

    pub fn old_uri(&self) -> &String {
        &self.old_uri
    }

    pub fn new_uri(&self) -> &String {
        &self.new_uri
    }
}

impl MoveStructType for UriMutationEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("UriMutationEvent");
}

impl MoveEventV1Type for UriMutationEvent {}

pub static URI_MUTATION_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        name: ident_str!("UriMutationEvent").to_owned(),
        type_args: vec![],
    }))
});
