// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::aggregator::AggregatorResource;
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ConcurrentSupplyResource {
    pub current: AggregatorResource<u128>,
}

impl MoveStructType for ConcurrentSupplyResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("fungible_asset");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ConcurrentSupply");
}

impl MoveResource for ConcurrentSupplyResource {}
