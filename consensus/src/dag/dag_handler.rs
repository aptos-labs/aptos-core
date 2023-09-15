// Copyright © Aptos Foundation

use super::{
    dag_driver::DagDriver,
    dag_fetcher::{FetchRequestHandler, FetchWaiter},
    dag_state_sync::{
        StateSyncStatus::{self, NeedsSync, Synced},
        StateSyncTrigger,
    },
    types::{CertifiedNodeMessage, TDAGMessage},
    CertifiedNode, Node,
};
use crate::{
    dag::{dag_network::RpcHandler, rb_handler::NodeBroadcastHandler, types::DAGMessage},
    network::{IncomingDAGRequest, TConsensusMsg},
};
use anyhow::bail;
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_logger::{error, warn};
use aptos_network::protocols::network::RpcError;
use aptos_types::epoch_state::EpochState;
use bytes::Bytes;
use futures::StreamExt;
use std::sync::Arc;
use tokio::select;

pub(crate) struct NetworkHandler {
    epoch_state: Arc<EpochState>,
    node_receiver: NodeBroadcastHandler,
    dag_driver: DagDriver,
    fetch_receiver: FetchRequestHandler,
    node_fetch_waiter: FetchWaiter<Node>,
    certified_node_fetch_waiter: FetchWaiter<CertifiedNode>,
    state_sync_trigger: StateSyncTrigger,
}

impl NetworkHandler {
    pub(super) fn new(
        epoch_state: Arc<EpochState>,
        node_receiver: NodeBroadcastHandler,
        dag_driver: DagDriver,
        fetch_receiver: FetchRequestHandler,
        node_fetch_waiter: FetchWaiter<Node>,
        certified_node_fetch_waiter: FetchWaiter<CertifiedNode>,
        state_sync_trigger: StateSyncTrigger,
    ) -> Self {
        Self {
            epoch_state,
            node_receiver,
            dag_driver,
            fetch_receiver,
            node_fetch_waiter,
            certified_node_fetch_waiter,
            state_sync_trigger,
        }
    }

    pub async fn run(
        mut self,
        dag_rpc_rx: &mut aptos_channel::Receiver<Author, IncomingDAGRequest>,
    ) -> CertifiedNodeMessage {
        // TODO(ibalajiarun): clean up Reliable Broadcast storage periodically.
        loop {
            select! {
                Some(msg) = dag_rpc_rx.next() => {
                    match self.process_rpc(msg).await {
                        Ok(sync_status) => {
                            if let StateSyncStatus::NeedsSync(certified_node_msg) = sync_status {
                                return certified_node_msg;
                            }
                        },
                        Err(e) =>  {
                            warn!(error = ?e, "error processing rpc");
                        }
                    }
                },
                Some(res) = self.node_fetch_waiter.next() => {
                    if let Err(e) = res.map_err(|e| anyhow::anyhow!("recv error: {}", e)).and_then(|node| self.node_receiver.process(node)) {
                        warn!(error = ?e, "error processing node fetch notification");
                    }
                },
                Some(res) = self.certified_node_fetch_waiter.next() => {
                    if let Err(e) = res.map_err(|e| anyhow::anyhow!("recv error: {}", e)).and_then(|certified_node| self.dag_driver.process(certified_node)) {
                        warn!(error = ?e, "error processing certified node fetch notification");
                    }
                }
            }
        }
    }

    async fn process_rpc(
        &mut self,
        rpc_request: IncomingDAGRequest,
    ) -> anyhow::Result<StateSyncStatus> {
        let dag_message: DAGMessage = rpc_request.req.try_into()?;

        let author = dag_message
            .author()
            .map_err(|_| anyhow::anyhow!("unexpected rpc message {:?}", dag_message))?;
        if author != rpc_request.sender {
            bail!("message author and network author mismatch");
        }

        let response: anyhow::Result<DAGMessage> = match dag_message {
            DAGMessage::NodeMsg(node) => node
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.node_receiver.process(node))
                .map(|r| r.into()),
            DAGMessage::CertifiedNodeMsg(certified_node_msg) => {
                match certified_node_msg.verify(&self.epoch_state.verifier) {
                    Ok(_) => match self.state_sync_trigger.check(certified_node_msg).await {
                        ret @ (NeedsSync(_), None) => return Ok(ret.0),
                        (Synced, Some(certified_node_msg)) => self
                            .dag_driver
                            .process(certified_node_msg.certified_node())
                            .map(|r| r.into()),
                        _ => unreachable!(),
                    },
                    Err(e) => Err(e),
                }
            },
            DAGMessage::FetchRequest(request) => request
                .verify(&self.epoch_state.verifier)
                .and_then(|_| self.fetch_receiver.process(request))
                .map(|r| r.into()),
            _ => {
                error!("unknown rpc message {:?}", dag_message);
                Err(anyhow::anyhow!("unknown rpc message"))
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
            .map_err(|_| anyhow::anyhow!("unable to respond to rpc"))
            .map(|_| StateSyncStatus::Synced)
    }
}
