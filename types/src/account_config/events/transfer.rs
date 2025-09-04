// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v2::MoveEventV2Type;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::MoveStructType,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Transfer {
    object: AccountAddress,
    from: AccountAddress,
    to: AccountAddress,
}

impl Transfer {
    pub fn new(object: AccountAddress, from: AccountAddress, to: AccountAddress) -> Self {
        Self { object, from, to }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> anyhow::Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn object(&self) -> &AccountAddress {
        &self.object
    }

    pub fn from(&self) -> &AccountAddress {
        &self.from
    }

    pub fn to(&self) -> &AccountAddress {
        &self.to
    }
}

impl MoveStructType for Transfer {
    const MODULE_NAME: &'static IdentStr = ident_str!("object");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Transfer");
}

impl MoveEventV2Type for Transfer {}

pub static TRANSFER_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("object").to_owned(),
        name: ident_str!("Transfer").to_owned(),
        type_args: vec![],
    }))
});
