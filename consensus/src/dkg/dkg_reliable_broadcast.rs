// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_infallible::Mutex;
use aptos_logger::error;
use aptos_types::epoch_state::EpochState;
use std::sync::Arc;
use thiserror::Error as ThisError;

use super::{types::{DKGNodeAck, DKGAggNode, DKGAggNodeAck}, dkg_network::DKGRpcHandler, dkg_store::DKGStore, DKGNode, dkg_manager::DKGManager};

#[derive(ThisError, Debug)]
pub enum DKGNodeHandleError {
    #[error("dummy error")]
    DummyError,
}

pub struct DKGNodeHandler {
    dkg_store: Arc<DKGStore>,
    epoch_state: Arc<EpochState>,
    dkg_manager: Arc<Mutex<DKGManager>>,
}

impl DKGNodeHandler {
    pub fn new(
        dkg_store: Arc<DKGStore>,
        epoch_state: Arc<EpochState>,
        dkg_manager: Arc<Mutex<DKGManager>>,
    ) -> Self {
        Self {
            dkg_store,
            epoch_state,
            dkg_manager,
        }
    }
}

impl DKGRpcHandler for DKGNodeHandler {
    type DKGRequest = DKGNode;
    type DKGResponse = DKGNodeAck;

    fn process(&mut self, node: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse> {
        let epoch = node.epoch();
        // dkg todo: persist the dkg nodes
        self.dkg_store.add_node(node, &self.epoch_state.verifier, self.dkg_manager.clone())?;
        Ok(DKGNodeAck::new(epoch))
    }
}

#[derive(Debug, ThisError)]
pub enum DKGAggNodeHandleError {
    #[error("dummy error")]
    DummyError,
}

pub struct DKGAggNodeHandler {
    dkg_store: Arc<DKGStore>,
    epoch_state: Arc<EpochState>,
    dkg_manager: Arc<Mutex<DKGManager>>,
}

impl DKGAggNodeHandler {
    pub fn new(dkg_store: Arc<DKGStore>, epoch_state: Arc<EpochState>, dkg_manager: Arc<Mutex<DKGManager>>) -> Self {
        Self {
            dkg_store,
            epoch_state,
            dkg_manager,
        }
    }
}

impl DKGRpcHandler for DKGAggNodeHandler {
    type DKGRequest = DKGAggNode;
    type DKGResponse = DKGAggNodeAck;

    fn process(&mut self, agg_node: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse> {
        let epoch = agg_node.epoch();
        self.dkg_store.add_agg_nodes(agg_node, &self.epoch_state.verifier, self.dkg_manager.clone())?;
        Ok(DKGAggNodeAck::new(epoch))
    }
}
