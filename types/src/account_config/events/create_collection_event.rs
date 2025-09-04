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
pub struct CreateCollectionEvent {
    creator: AccountAddress,
    collection_name: String,
    uri: String,
    description: String,
    maximum: u64,
}

impl CreateCollectionEvent {
    pub fn new(
        creator: AccountAddress,
        collection_name: String,
        uri: String,
        description: String,
        maximum: u64,
    ) -> Self {
        Self {
            creator,
            collection_name,
            uri,
            description,
            maximum,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn creator(&self) -> &AccountAddress {
        &self.creator
    }

    pub fn collection_name(&self) -> &String {
        &self.collection_name
    }

    pub fn uri(&self) -> &String {
        &self.uri
    }

    pub fn description(&self) -> &String {
        &self.description
    }

    pub fn maximum(&self) -> u64 {
        self.maximum
    }
}

impl MoveStructType for CreateCollectionEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token");
    const STRUCT_NAME: &'static IdentStr = ident_str!("CreateCollectionEvent");
}

impl MoveEventV1Type for CreateCollectionEvent {}

pub static CREATE_COLLECTION_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token").to_owned(),
        name: ident_str!("CreateCollectionEvent").to_owned(),
        type_args: vec![],
    }))
});
