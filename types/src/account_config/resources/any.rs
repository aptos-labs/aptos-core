// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::{StructTag, TypeTag, CORE_CODE_ADDRESS},
    move_resource::{MoveResource, MoveStructType},
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AnyResource {
    type_name: String,
    data: Vec<u8>,
}

impl AnyResource {
    pub fn try_from_bytes(bytes: &[u8]) -> Result<Self> {
        bcs::from_bytes(bytes).map_err(Into::into)
    }
}

impl MoveStructType for AnyResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("any");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Any");
}

impl MoveResource for AnyResource {}

pub static ANY_RESOURCE_TYPE: Lazy<TypeTag> = Lazy::new(|| {
    TypeTag::Struct(Box::new(StructTag {
        address: CORE_CODE_ADDRESS,
        module: ident_str!("any").to_owned(),
        name: ident_str!("Any").to_owned(),
        type_args: vec![],
    }))
});
