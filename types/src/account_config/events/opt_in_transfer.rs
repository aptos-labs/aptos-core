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
pub struct OptInTransfer {
    account_address: AccountAddress,
    opt_in: bool,
}

impl OptInTransfer {
    pub fn new(account_address: AccountAddress, opt_in: bool) -> Self {
        Self {
            account_address,
            opt_in,
        }
    }

    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }

    pub fn account_address(&self) -> &AccountAddress {
        &self.account_address
    }

    pub fn opt_in(&self) -> &bool {
        &self.opt_in
    }
}

impl MoveStructType for OptInTransfer {
    const MODULE_NAME: &'static IdentStr = ident_str!("token_event_store");
    const STRUCT_NAME: &'static IdentStr = ident_str!("OptInTransfer");
}

impl MoveEventV2Type for OptInTransfer {}

pub static OPT_IN_TRANSFER_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: TOKEN_ADDRESS,
        module: ident_str!("token_event_store").to_owned(),
        name: ident_str!("OptInTransfer").to_owned(),
        type_args: vec![],
    }))
});
