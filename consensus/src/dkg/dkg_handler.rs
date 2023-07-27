// Copyright © Aptos Foundation

use anyhow::bail;
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_logger::{error, warn};
use aptos_network::protocols::network::RpcError;
use aptos_types::epoch_state::EpochState;
use bytes::Bytes;
use futures::StreamExt;
use std::sync::Arc;

use crate::network::{IncomingDKGRequest, TConsensusMsg};

use super::{dkg_reliable_broadcast::{DKGNodeHandler, DKGAggNodeHandler}, dkg_store::DKGStore, types::{DKGMessage, TDKGMessage}, dkg_network::DKGRpcHandler, dkg_manager::DKGManager};

struct DKGNetworkHandler {
    dkg_rpc_rx: aptos_channel::Receiver<Author, IncomingDKGRequest>,
    node_receiver: DKGNodeHandler,
    agg_node_receiver: DKGAggNodeHandler,
    epoch_state: Arc<EpochState>,
}

impl DKGNetworkHandler {
    fn new(
        dkg_rpc_rx: aptos_channel::Receiver<Author, IncomingDKGRequest>,
        epoch_state: Arc<EpochState>,
        dkg_store: Arc<DKGStore>,
        dkg_manager: Arc<Mutex<DKGManager>>,
    ) -> Self {
        Self {
            dkg_rpc_rx,
            node_receiver: DKGNodeHandler::new(
                dkg_store.clone(),
                epoch_state.clone(),
                dkg_manager.clone(),
            ),
            agg_node_receiver: DKGAggNodeHandler::new(
                dkg_store.clone(),
                epoch_state.clone(),
                dkg_manager.clone(),
            ),
            epoch_state: epoch_state.clone(),
        }
    }

    async fn start(mut self) {
        while let Some(msg) = self.dkg_rpc_rx.next().await {
            if let Err(e) = self.process_rpc(msg).await {
                warn!(error = ?e, "[DKG] error processing rpc");
            }
        }
    }

    async fn process_rpc(&mut self, rpc_request: IncomingDKGRequest) -> anyhow::Result<()> {
        let dkg_message: DKGMessage = rpc_request.req.try_into()?;

        let author = dkg_message
            .author()
            .map_err(|_| anyhow::anyhow!("[DKG] unexpected rpc message {:?}", dkg_message))?;
        if author != rpc_request.sender {
            bail!("[DKG] message author and network author mismatch");
        }

        let response: anyhow::Result<DKGMessage> = match dkg_message {
            DKGMessage::DKGNodeMsg(node) => node
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.node_receiver.process(node))
                .map(|r| r.into()),
            DKGMessage::DKGAggNodeMsg(agg_node) => agg_node
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.agg_node_receiver.process(agg_node))
                .map(|r| r.into()),
            _ => {
                error!("[DKG] unknown rpc message {:?}", dkg_message);
                Err(anyhow::anyhow!("[DKG] unknown rpc message"))
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
            .map_err(|_| anyhow::anyhow!("[DKG] unable to respond to rpc"))
    }
}
