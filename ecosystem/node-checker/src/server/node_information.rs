// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0
use crate::configuration::NodeAddress;
use aptos_config::config::RoleType;
use aptos_sdk::types::chain_id::ChainId;

/// This struct captures all the relevant information needed to address a node
/// and make assertions about its identity.
#[derive(Clone, Debug)]
pub struct NodeInformation {
    pub node_address: NodeAddress,
    pub chain_id: ChainId,
    pub role_type: RoleType,
}
