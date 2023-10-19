// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use move_core_types::{
    ident_str,
    identifier::IdentStr,
    language_storage::TypeTag,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConcurrentSupply {
    pub current: Aggregator,
}

impl MoveStructType for ConcurrentSupply {
    const MODULE_NAME: &'static IdentStr = ident_str!("fungible_asset");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ConcurrentSupply");
}

impl MoveResource for ConcurrentSupply {}

/// Rust representation of Aggregator Move struct.
#[derive(Debug, Serialize, Deserialize)]
pub struct Aggregator {
    pub value: u128,
    pub max_value: u128,
}

impl Aggregator {
    pub fn new(value: u128) -> Self {
        Self {
            value,
            max_value: u128::MAX,
        }
    }
}

impl MoveStructType for Aggregator {
    const MODULE_NAME: &'static IdentStr = ident_str!("aggregator_v2");
    const STRUCT_NAME: &'static IdentStr = ident_str!("Aggregator");

    fn type_args() -> Vec<TypeTag> {
        vec![TypeTag::U128]
    }
}

impl MoveResource for Aggregator {}
