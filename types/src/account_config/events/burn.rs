// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, TOKEN_OBJECTS_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Burn {
    collection: AccountAddress,
    index: u64,
    token: AccountAddress,
    previous_owner: AccountAddress,
}

impl Burn {
    pub fn new(
        collection: AccountAddress,
        index: u64,
        token: AccountAddress,
        previous_owner: AccountAddress,
    ) -> Self {
        Self {
            collection,
            index,
            token,
            previous_owner,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn collection(&self) -> &AccountAddress {
        &self.collection
    }

    pub fn index(&self) -> &u64 {
        &self.index
    }

    pub fn token(&self) -> &AccountAddress {
        &self.token
    }

    pub fn previous_owner(&self) -> &AccountAddress {
        &self.previous_owner
    }
}

impl MoveStructType for Burn {
    const MODULE_NAME: &'static IdentStr = ident_str!("collection");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Burn");
}

impl MoveEventV2Type for Burn {}

pub static BURN_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_OBJECTS_ADDRESS,
        module: ident_str!("collection").to_owned(),
        name: ident_str!("Burn").to_owned(),
        type_args: vec![],
    }))
});
