// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{chain_id::ChainId, on_chain_config::OnChainConfig};
use move_core_types::{
    ident_str,
    identifier::IdentStr,
    move_resource::{MoveResource, MoveStructType},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChainIdResource {
    chain_id: u8,
}

impl ChainIdResource {
    pub fn chain_id(&self) -> ChainId {
        ChainId::new(self.chain_id)
    }
}

impl MoveStructType for ChainIdResource {
    const MODULE_NAME: &'static IdentStr = ident_str!("chain_id");
    const STRUCT_NAME: &'static IdentStr = ident_str!("ChainId");
}

impl MoveResource for ChainIdResource {}

impl OnChainConfig for ChainIdResource {
    const MODULE_IDENTIFIER: &'static str = "chain_id";
    const TYPE_IDENTIFIER: &'static str = "ChainId";
}
