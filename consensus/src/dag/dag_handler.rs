// Copyright © Aptos Foundation

use crate::{
    dag::{
        dag_driver::DagDriver,
        dag_fetcher::{FetchRequestHandler, FetchWaiter},
        dag_network::RpcHandler,
        dag_state_sync::{SyncOutcome, StateSyncTrigger},
        errors::{
            DAGError, DAGRpcError, DagDriverError, FetchRequestHandleError,
            NodeBroadcastHandleError,
        },
        rb_handler::NodeBroadcastHandler,
        types::{DAGMessage, DAGRpcResult},
        CertifiedNode, Node,
    },
    network::{IncomingDAGRequest, TConsensusMsg},
};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::{debug, error, warn};
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
    new_round_event: tokio::sync::mpsc::Receiver<Round>,
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
        new_round_event: tokio::sync::mpsc::Receiver<Round>,
    ) -> Self {
        Self {
            epoch_state,
            node_receiver,
            dag_driver,
            fetch_receiver,
            node_fetch_waiter,
            certified_node_fetch_waiter,
            state_sync_trigger,
            new_round_event,
        }
    }

    pub async fn run(
        mut self,
        dag_rpc_rx: &mut aptos_channel::Receiver<Author, IncomingDAGRequest>,
        _buffer: Vec<DAGMessage>,
    ) -> SyncOutcome {
        // TODO: process buffer
        loop {
            select! {
                msg = dag_rpc_rx.select_next_some() => {
                    match self.process_rpc(msg).await {
                        Ok(sync_status) => {
                            if matches!(sync_status, SyncOutcome::NeedsSync(_) | SyncOutcome::EpochEnds) {
                                return sync_status;
                            }
                        },
                        Err(e) =>  {
                            warn!(error = ?e, "error processing rpc");
                        }
                    }
                },
                Some(new_round) = self.new_round_event.recv() => {
                    self.dag_driver.enter_new_round(new_round).await;
                    self.node_receiver.gc();
                }
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
    ) -> anyhow::Result<SyncOutcome> {
        let dag_message: DAGMessage = rpc_request.req.try_into()?;
        let epoch = dag_message.epoch();

        debug!(
            "processing rpc message {} from {}",
            dag_message.name(),
            rpc_request.sender
        );

        let response: Result<DAGMessage, DAGError> = {
            match dag_message.verify(rpc_request.sender, &self.epoch_state.verifier) {
                Ok(_) => match dag_message {
                    DAGMessage::NodeMsg(node) => self
                        .node_receiver
                        .process(node)
                        .await
                        .map(|r| r.into())
                        .map_err(|err| {
                            err.downcast::<NodeBroadcastHandleError>()
                                .map_or(DAGError::Unknown, |err| {
                                    DAGError::NodeBroadcastHandleError(err)
                                })
                        }),
                    DAGMessage::CertifiedNodeMsg(certified_node_msg) => {
                        match self.state_sync_trigger.check(certified_node_msg).await? {
                            SyncOutcome::Synced(Some(certified_node_msg)) => self
                                .dag_driver
                                .process(certified_node_msg.certified_node())
                                .await
                                .map(|r| r.into())
                                .map_err(|err| {
                                    err.downcast::<DagDriverError>()
                                        .map_or(DAGError::Unknown, |err| {
                                            DAGError::DagDriverError(err)
                                        })
                                }),
                            status @ (SyncOutcome::NeedsSync(_)
                            | SyncOutcome::EpochEnds) => return Ok(status),
                            _ => unreachable!(),
                        }
                    },
                    DAGMessage::FetchRequest(request) => self
                        .fetch_receiver
                        .process(request)
                        .await
                        .map(|r| r.into())
                        .map_err(|err| {
                            err.downcast::<FetchRequestHandleError>()
                                .map_or(DAGError::Unknown, DAGError::FetchRequestHandleError)
                        }),
                    _ => unreachable!("verification must catch this error"),
                },
                Err(err) => {
                    error!(error = ?err, "error verifying message");
                    Err(DAGError::MessageVerificationError)
                },
            }
        };

        debug!(
            "responding to process_rpc {:?}",
            response.as_ref().map(|r| r.name())
        );

        let response: DAGRpcResult = response.map_err(|e| DAGRpcError::new(epoch, e)).into();

        let rpc_response = rpc_request
            .protocol
            .to_bytes(&response.into_network_message())
            .map(Bytes::from)
            .map_err(RpcError::Error);

        rpc_request
            .response_sender
            .send(rpc_response)
            .map_err(|_| anyhow::anyhow!("unable to respond to rpc"))?;

        Ok(SyncOutcome::Synced(None))
    }
}
