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
pub struct CollectionMaximumMutateEvent {
    creator_addr: AccountAddress,
    collection_name: String,
    old_maximum: u64,
    new_maximum: u64,
}

impl CollectionMaximumMutateEvent {
    pub fn new(
        creator_addr: AccountAddress,
        collection_name: String,
        old_maximum: u64,
        new_maximum: u64,
    ) -> Self {
        Self {
            creator_addr,
            collection_name,
            old_maximum,
            new_maximum,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn creator_addr(&self) -> &AccountAddress {
        &self.creator_addr
    }

    pub fn collection_name(&self) -> &String {
        &self.collection_name
    }

    pub fn old_maximum(&self) -> &u64 {
        &self.old_maximum
    }

    pub fn new_maximum(&self) -> &u64 {
        &self.new_maximum
    }
}

impl MoveStructType for CollectionMaximumMutateEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    // The struct name in the Move code contains a typo.
    const STRUCT_NAME: &'static IdentStr = ident_str!("CollectionMaxiumMutateEvent");
}

impl MoveEventV1Type for CollectionMaximumMutateEvent {}

pub static COLLECTION_MAXIMUM_MUTATE_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        // The struct name in the Move code contains a typo.
        name: ident_str!("CollectionMaxiumMutateEvent").to_owned(),
        type_args: vec![],
    }))
});
