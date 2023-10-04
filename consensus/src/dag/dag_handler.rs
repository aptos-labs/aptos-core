// Copyright © Aptos Foundation

use super::{
    dag_driver::DagDriver,
    dag_fetcher::{FetchRequestHandler, FetchWaiter},
    dag_state_sync::{StateSyncStatus, StateSyncTrigger},
    CertifiedNode, Node,
};
use crate::{
    dag::{dag_network::RpcHandler, rb_handler::NodeBroadcastHandler, types::DAGMessage},
    network::{IncomingDAGRequest, TConsensusMsg},
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::Author;
use aptos_logger::{debug, warn};
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
    ) -> StateSyncStatus {
        // TODO(ibalajiarun): clean up Reliable Broadcast storage periodically.
        loop {
            select! {
                msg = dag_rpc_rx.select_next_some() => {
                    match self.process_rpc(msg).await {
                        Ok(sync_status) => {
                            if matches!(sync_status, StateSyncStatus::NeedsSync(_) | StateSyncStatus::EpochEnds) {
                                return sync_status;
                            }
                        },
                        Err(e) =>  {
                            warn!(error = ?e, "error processing rpc");
                        }
                    }
                },
                Some(res) = self.node_fetch_waiter.next() => {
                    match res {
                        Ok(node) => if let Err(e) = self.node_receiver.process(node).await {
                            warn!(error = ?e, "error processing node fetch notification");
                        },
                        Err(e) => {
                            debug!("sender dropped channel: {}", e);
                        },
                    };
                },
                Some(res) = self.certified_node_fetch_waiter.next() => {
                    match res {
                        Ok(certified_node) => if let Err(e) = self.dag_driver.process(certified_node).await {
                            warn!(error = ?e, "error processing certified node fetch notification");                        },
                        Err(e) => {
                            debug!("sender dropped channel: {}", e);
                        },
                    };
                }
            }
        }
    }

    async fn process_rpc(
        &mut self,
        rpc_request: IncomingDAGRequest,
    ) -> anyhow::Result<StateSyncStatus> {
        let dag_message: DAGMessage = rpc_request.req.try_into()?;

        debug!(
            "processing rpc message {} from {}",
            dag_message.name(),
            rpc_request.sender
        );

        let response: anyhow::Result<DAGMessage> = {
            match dag_message.verify(rpc_request.sender, &self.epoch_state.verifier) {
                Ok(_) => match dag_message {
                    DAGMessage::NodeMsg(node) => {
                        self.node_receiver.process(node).await.map(|r| r.into())
                    },
                    DAGMessage::CertifiedNodeMsg(certified_node_msg) => {
                        match self.state_sync_trigger.check(certified_node_msg).await? {
                            StateSyncStatus::Synced(Some(certified_node_msg)) => self
                                .dag_driver
                                .process(certified_node_msg.certified_node())
                                .await
                                .map(|r| r.into()),
                            status @ (StateSyncStatus::NeedsSync(_)
                            | StateSyncStatus::EpochEnds) => return Ok(status),
                            _ => unreachable!(),
                        }
                    },
                    DAGMessage::FetchRequest(request) => {
                        self.fetch_receiver.process(request).await.map(|r| r.into())
                    },
                    _ => unreachable!("verification must catch this error"),
                },
                Err(err) => Err(err),
            }
        };

        debug!(
            "responding to process_rpc {:?}",
            response.as_ref().map(|r| r.name())
        );

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
            .map(|_| StateSyncStatus::Synced(None))
    }
}
