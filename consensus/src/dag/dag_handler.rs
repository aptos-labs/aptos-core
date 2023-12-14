// Copyright © Aptos Foundation

use crate::{
    dag::{
        dag_driver::DagDriver,
        dag_fetcher::{FetchRequestHandler, FetchWaiter},
        dag_network::RpcHandler,
        dag_state_sync::{StateSyncTrigger, SyncOutcome},
        errors::{
            DAGError, DAGRpcError, DagDriverError, FetchRequestHandleError,
            NodeBroadcastHandleError,
        },
        rb_handler::NodeBroadcastHandler,
        types::{DAGMessage, DAGRpcResult},
        CertifiedNode, Node,
    },
    monitor,
    network::{IncomingDAGRequest, RpcResponder},
};
use aptos_bounded_executor::{concurrent_map, BoundedExecutor};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::{debug, error, warn};
use aptos_types::epoch_state::EpochState;
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
        executor: BoundedExecutor,
        _buffer: Vec<DAGMessage>,
    ) -> SyncOutcome {
        // TODO: process buffer
        let epoch_state = self.epoch_state.clone();

        let mut verified_msg_stream = concurrent_map(
            dag_rpc_rx,
            executor,
            move |rpc_request: IncomingDAGRequest| {
                let epoch_state = epoch_state.clone();
                async move {
                    let result = rpc_request
                        .req
                        .try_into()
                        .and_then(|dag_message: DAGMessage| {
                            monitor!(
                                "dag_message_verify",
                                dag_message.verify(rpc_request.sender, &epoch_state.verifier)
                            )?;
                            Ok(dag_message)
                        });
                    (result, rpc_request.sender, rpc_request.responder)
                }
            },
        );

        loop {
            select! {
                (msg, author, responder) = verified_msg_stream.select_next_some() => {
                    debug!("receive verified message");
                    match self.process_verified_message(msg, author, responder).await {
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

    async fn process_verified_message(
        &mut self,
        dag_message_result: anyhow::Result<DAGMessage>,
        author: Author,
        responder: RpcResponder,
    ) -> anyhow::Result<SyncOutcome> {
        let response: Result<DAGMessage, DAGError> = {
            match dag_message_result {
                Ok(dag_message) => {
                    debug!(
                        author = author,
                        message = dag_message,
                        "process verified message"
                    );
                    match dag_message {
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
                                status @ (SyncOutcome::NeedsSync(_) | SyncOutcome::EpochEnds) => {
                                    return Ok(status)
                                },
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
                    }
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

        let response: DAGRpcResult = response
            .map_err(|e| DAGRpcError::new(self.epoch_state.epoch, e))
            .into();
        responder.respond(response)?;

        Ok(SyncOutcome::Synced(None))
    }
}
