// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::{MoveResource, MoveStructType},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// A Rust representation of TypeInfo.
#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct TypeInfoResource {
    pub account_address: AccountAddress,
    pub module_name: Vec<u8>,
    pub struct_name: Vec<u8>,
}

impl TypeInfoResource {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

impl MoveStructType for TypeInfoResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("type_info");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TypeInfo");
}

impl MoveResource for TypeInfoResource {}

pub static TYPE_INFO_RESOURCE_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("type_info").to_owned(),
        name: ident_str!("TypeInfo").to_owned(),
        type_args: vec![],
    }))
});
