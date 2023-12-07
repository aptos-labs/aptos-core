// Copyright © Aptos Foundation

use super::{
    dkg_manager::DKGManager,
    DKGNode, types::{DKGAggNodeAck, DKGMessage, DKGNodeAck},
};
use crate::{network::{IncomingDKGRequest, TConsensusMsg}, network_interface::ConsensusMsg};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_logger::{debug, error, info, warn};
use aptos_network::protocols::network::RpcError;
use aptos_types::epoch_state::EpochState;
use async_trait::async_trait;
use bytes::Bytes;
use futures::StreamExt;
use thiserror::Error as ThisError;
use std::{sync::Arc, time::Duration};
use aptos_types::dkg::DKGAggNode;

#[derive(ThisError, Debug)]
pub enum DKGRpcHandleError {
    #[error("DKG store not initialized (can be expected)")]
    DKGStoreNotInitialized,
}

pub trait DKGRpcHandler {
    type DKGRequest;
    type DKGResponse;

    fn process(&mut self, message: Self::DKGRequest) -> anyhow::Result<Self::DKGResponse>;
}

#[async_trait]
pub trait DKGNetworkSender: Send + Sync {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg>;

    /// Given a list of potential responders, sending rpc to get response from any of them and could
    /// fallback to more in case of failures.
    async fn send_rpc_with_fallbacks(
        &self,
        responders: Vec<Author>,
        message: ConsensusMsg,
        timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg>;
}

#[derive(Clone)]
pub struct DKGNetworkHandler {
    author: Author,
    // dkg_rpc_rx: aptos_channel::Receiver<Author, IncomingDKGRequest>,
    node_receiver: DKGNodeHandler,
    agg_node_receiver: DKGAggNodeHandler,
    epoch_state: Arc<EpochState>,
}

impl DKGNetworkHandler {
    pub fn new(
        author: Author,
        // dkg_rpc_rx: aptos_channel::Receiver<Author, IncomingDKGRequest>,
        epoch_state: Arc<EpochState>,
        dkg_manager: DKGManager,
    ) -> Self {
        Self {
            author,
            // dkg_rpc_rx,
            node_receiver: DKGNodeHandler::new(dkg_manager.clone()),
            agg_node_receiver: DKGAggNodeHandler::new(dkg_manager.clone()),
            epoch_state: epoch_state.clone(),
        }
    }

    pub async fn start(self, mut dkg_rpc_rx: aptos_channel::Receiver<Author, IncomingDKGRequest>) {
        info!(
            epoch = self.epoch_state.epoch,
            author = self.author,
            "[DKG] DKGHandler started"
        );
        while let Some(msg) = dkg_rpc_rx.next().await {
            let sender = msg.sender;
            let mut handler = self.clone();

            tokio::task::spawn(async move {
                if let Err(e) = handler.process_rpc(msg).await {
                    warn!("[DKG] error processing rpc from peer {:?}: {}", sender, e);
                }
            });
        }
        info!(
            epoch = self.epoch_state.epoch,
            author = self.author,
            "[DKG] DKGHandler stopped"
        );
    }

    async fn process_rpc(&mut self, rpc_request: IncomingDKGRequest) -> anyhow::Result<()> {
        let dkg_message: DKGMessage = rpc_request.req.try_into()?;

        let response: anyhow::Result<DKGMessage> = match dkg_message {
            DKGMessage::DKGNodeMsg(node) => self.node_receiver.process(node).map(|r| r.into()),
            DKGMessage::DKGAggNodeMsg(agg_node) => {
                self.agg_node_receiver.process(agg_node).map(|r| r.into())
            },
            _ => {
                Err(anyhow::anyhow!("Unknown rpc message"))
            },
        };

        let response = response
            .and_then(|response_msg| {
                rpc_request
                    .protocol
                    .to_bytes(&response_msg.into_network_message())
                    .map(Bytes::from)
            })
            .map_err(RpcError::ApplicationError);

        rpc_request
            .response_sender
            .send(response)
            .map_err(|_| anyhow::anyhow!("Unable to respond to rpc"))
    }
}

#[derive(Clone)]
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
        debug!("[DKG] Process DKG Node from {:?}", node.author());
        // dkg todo: persist the dkg nodes
        match self.dkg_manager.add_node(node) {
            Ok(_) => Ok(DKGNodeAck::new(epoch)),
            Err(e) => Err(e),
        }
    }
}

#[derive(Clone)]
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
        debug!("[DKG] Process DKG Aggregated Node: {:?}", agg_node.metadata());
        // dkg todo: persist the dkg nodes
        match self.dkg_manager.add_agg_node(agg_node) {
            Ok(_) => Ok(DKGAggNodeAck::new(epoch)),
            Err(e) => Err(e),
        }
    }
}
