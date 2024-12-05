// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_config::aggregator::AggregatorSnapshotResource,
    move_utils::move_event_v2::MoveEventV2Type,
};
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_OBJECTS_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Mint {
    collection: AccountAddress,
    index: AggregatorSnapshotResource<u64>,
    token: AccountAddress,
}

impl Mint {
    pub fn new(
        collection: AccountAddress,
        index: AggregatorSnapshotResource<u64>,
        token: AccountAddress,
    ) -> Self {
        Self {
            collection,
            index,
            token,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn collection(&self) -> &AccountAddress {
        &self.collection
    }

    pub fn index(&self) -> &AggregatorSnapshotResource<u64> {
        &self.index
    }

    pub fn token(&self) -> &AccountAddress {
        &self.token
    }
}

impl MoveStructType for Mint {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Mint");
}

impl MoveEventV2Type for Mint {}

pub static MINT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_OBJECTS_ADDRESS,
        module: ident_str!("collection").to_owned(),
        name: ident_str!("Mint").to_owned(),
        type_args: vec![],
    }))
});
