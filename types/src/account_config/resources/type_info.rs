// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{
    account_address::AccountAddress,
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

/// A Rust representation of TypeInfo.
#[derive(Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct TypeInfoResource {
    pub account_address: AccountAddress,
    pub module_name: Vec<u8>,
    pub struct_name: Vec<u8>,
}

impl MoveStructType for TypeInfoResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("type_info");
    const STRUCT_NAME: &'static IdentStr = ident_str!("TypeInfo");
}

impl MoveResource for TypeInfoResource {}
