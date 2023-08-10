// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

// use aptos_logger::error;
// use thiserror::Error as ThisError;

use super::{
    dkg_manager::DKGManager,
    dkg_network::DKGRpcHandler,
    types::{DKGAggNode, DKGAggNodeAck, DKGNodeAck},
    DKGNode,
};
use aptos_logger::error;

// #[derive(ThisError, Debug)]
// pub enum DKGNodeHandleError {
//     #[error("dummy error")]
//     DummyError,
// }

pub struct DKGNodeHandler {
    dkg_manager: DKGManager,
}

impl DKGNodeHandler {
    pub fn new(dkg_manager: DKGManager) -> Self {
        Self { dkg_manager }
    }
}

impl DKGRpcHandler for DKGNodeHandler {
    type DKGRequest = DKGNode;
    type DKGResponse = DKGNodeAck;

    fn process(&mut self, node: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse> {
        let epoch = node.epoch();
        // dkg todo: persist the dkg nodes
        match self.dkg_manager.add_node(node) {
            Ok(_) => Ok(DKGNodeAck::new(epoch)),
            Err(e) => {
                error!("[DKG] Error when adding DKG node: {:?}", e);
                Err(e)
            },
        }
    }
}

// #[derive(Debug, ThisError)]
// pub enum DKGAggNodeHandleError {
//     #[error("dummy error")]
//     DummyError,
// }

pub struct DKGAggNodeHandler {
    dkg_manager: DKGManager,
}

impl DKGAggNodeHandler {
    pub fn new(dkg_manager: DKGManager) -> Self {
        Self { dkg_manager }
    }
}

impl DKGRpcHandler for DKGAggNodeHandler {
    type DKGRequest = DKGAggNode;
    type DKGResponse = DKGAggNodeAck;

    fn process(&mut self, agg_node: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse> {
        let epoch = agg_node.epoch();
        // dkg todo: persist the dkg nodes
        match self.dkg_manager.add_agg_node(agg_node) {
            Ok(_) => Ok(DKGAggNodeAck::new(epoch)),
            Err(e) => {
                error!("[DKG] Error when adding DKG aggregated node: {:?}", e);
                Err(e)
            },
        }
    }
}
