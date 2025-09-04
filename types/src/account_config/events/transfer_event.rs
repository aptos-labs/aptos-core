// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::move_utils::move_event_v1::MoveEventV1Type;
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
pub struct TransferEvent {
    object: AccountAddress,
    from: AccountAddress,
    to: AccountAddress,
}

impl TransferEvent {
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

impl MoveStructType for TransferEvent {
    const MODULE_NAME: &'static IdentStr = ident_str!("object");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TransferEvent");
}

impl MoveEventV1Type for TransferEvent {}

pub static TRANSFER_EVENT_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("object").to_owned(),
        name: ident_str!("TransferEvent").to_owned(),
        type_args: vec![],
    }))
});
