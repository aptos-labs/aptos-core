// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
        observability::counters::{INCOMING_MSG_PROCESSING, RPC_PROCESS_DURATION},
        rb_handler::NodeBroadcastHandler,
        types::{DAGMessage, DAGRpcResult},
        CertifiedNode, Node,
    },
    monitor,
    network::{IncomingDAGRequest, RpcResponder},
};
use anyhow::anyhow;
use aptos_bounded_executor::{BoundedExecutor, ConcurrentStream};
use aptos_channels::aptos_channel;
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::{
    debug, error,
    prelude::{sample, SampleRate},
    warn,
};
use aptos_types::epoch_state::EpochState;
use dashmap::DashMap;
use futures::{stream::FuturesUnordered, FutureExt, StreamExt};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{runtime::Handle, select};

pub(crate) struct NetworkHandler {
    epoch_state: Arc<EpochState>,
    node_receiver: Arc<NodeBroadcastHandler>,
    dag_driver: Arc<DagDriver>,
    node_fetch_waiter: FetchWaiter<Node>,
    certified_node_fetch_waiter: FetchWaiter<CertifiedNode>,
    new_round_event: tokio::sync::mpsc::UnboundedReceiver<Round>,
    verified_msg_processor: Arc<VerifiedMessageProcessor>,
    missing_parents_rx: tokio::sync::mpsc::UnboundedReceiver<Node>,
    next_dag_tx: tokio::sync::mpsc::UnboundedSender<Instant>,
    prev_dag_rx: tokio::sync::mpsc::UnboundedReceiver<Instant>,
    dag_id: u8,
}

impl NetworkHandler {
    pub(super) fn new(
        dag_id: u8,
        epoch_state: Arc<EpochState>,
        mut node_receiver: NodeBroadcastHandler,
        dag_driver: DagDriver,
        fetch_receiver: FetchRequestHandler,
        node_fetch_waiter: FetchWaiter<Node>,
        certified_node_fetch_waiter: FetchWaiter<CertifiedNode>,
        state_sync_trigger: StateSyncTrigger,
        new_round_event: tokio::sync::mpsc::UnboundedReceiver<Round>,
        next_dag_tx: tokio::sync::mpsc::UnboundedSender<Instant>,
        prev_dag_rx: tokio::sync::mpsc::UnboundedReceiver<Instant>,
        payload_store: Arc<DashMap<(Round, Author), Node>>,
    ) -> Self {
        let (missing_parents_tx, missing_parents_rx) = tokio::sync::mpsc::unbounded_channel();
        node_receiver.set_missing_parent_tx(missing_parents_tx);
        let node_receiver = Arc::new(node_receiver);
        let dag_driver = Arc::new(dag_driver);
        Self {
            epoch_state: epoch_state.clone(),
            node_receiver: node_receiver.clone(),
            dag_driver: dag_driver.clone(),
            node_fetch_waiter,
            certified_node_fetch_waiter,
            new_round_event,
            missing_parents_rx,
            dag_id,
            verified_msg_processor: Arc::new(VerifiedMessageProcessor {
                dag_id,
                node_receiver,
                dag_driver,
                fetch_receiver,
                state_sync_trigger,
                epoch_state,
                payload_store,
            }),
            next_dag_tx,
            prev_dag_rx,
        }
    }

    pub async fn run(
        self,
        dag_rpc_rx: &mut aptos_channel::Receiver<Author, IncomingDAGRequest>,
        executor: BoundedExecutor,
        _buffer: Vec<DAGMessage>,
    ) -> SyncOutcome {
        // TODO: process buffer
        let NetworkHandler {
            epoch_state,
            node_receiver,
            dag_driver,
            mut node_fetch_waiter,
            mut certified_node_fetch_waiter,
            mut new_round_event,
            verified_msg_processor,
            mut missing_parents_rx,
            mut next_dag_tx,
            mut prev_dag_rx,
            dag_id,
            ..
        } = self;

        let monitor = tokio_metrics_collector::TaskMonitor::new();
        tokio_metrics_collector::default_task_collector()
            .add("dag_handler", monitor.clone())
            .ok();

        let dag_driver_clone = dag_driver.clone();
        let node_receiver_clone = node_receiver.clone();
        let handle = tokio::spawn(async move {
            while let Some(new_round) = new_round_event.recv().await {
                let driver = dag_driver_clone.clone();
                tokio::task::spawn_blocking(move || {
                    monitor!("dag_sort_peers", {
                        driver.peers_by_latency.write().sort();
                    })
                });
                monitor!("dag_on_new_round_event", {
                    dag_driver_clone
                        .enter_new_round_ext(new_round, &mut prev_dag_rx, &mut next_dag_tx)
                        .await;
                });
                monitor!("dag_node_receiver_gc", {
                    node_receiver_clone.gc();
                });
            }
        });
        defer!(handle.abort());

        let task_node_receiver = node_receiver.clone();
        let handle = tokio::spawn(async move {
            let mut pending_parents = BTreeMap::new();
            let mut interval = tokio::time::interval(Duration::from_millis(10));
            loop {
                select! {
                    Some(msg) = missing_parents_rx.recv() => {
                        pending_parents.insert((msg.round(), *msg.author()), msg);
                        while let Some(Some(msg)) = missing_parents_rx.recv().now_or_never() {
                            pending_parents.insert((msg.round(), *msg.author()), msg);
                        }
                    },
                    _ = interval.tick() => {
                        for (k, node) in pending_parents.into_iter() {
                            let receiver = task_node_receiver.clone();
                            tokio::task::spawn(async move {
                                (k, receiver.process(node).await)
                            });
                        }
                        pending_parents = BTreeMap::new();
                    },
                }
            }
        });
        defer!(handle.abort());

        let mut futures = FuturesUnordered::new();
        // A separate executor to ensure the message verification sender (above) and receiver (below) are
        // not blocking each other.
        // TODO: make this configurable
        let executor = BoundedExecutor::new(200, Handle::current());
        let (verified_msg_stream_tx, mut verified_msg_stream) =
            tokio::sync::mpsc::unbounded_channel::<(
                anyhow::Result<DAGMessage>,
                u64,
                Author,
                RpcResponder,
                Instant,
            )>();
        let (ret_tx, mut ret_rx) = tokio::sync::oneshot::channel();

        let h = tokio::task::spawn(async move {
            loop {
                select! {
                    Some((msg, epoch, author, responder, start)) = verified_msg_stream.recv() => {
                        RPC_PROCESS_DURATION
                            .with_label_values(&["post_verify_recv"])
                            .observe(start.elapsed().as_secs_f64());
                        let verified_msg_processor = verified_msg_processor.clone();
                        let f = executor.spawn(monitor.instrument(async move {
                            monitor!("dag_on_verified_msg", {
                                match verified_msg_processor.process_verified_message(msg, epoch, author, responder, start).await {
                                    Ok(sync_status) => {
                                        if matches!(
                                            sync_status,
                                            SyncOutcome::NeedsSync(_) | SyncOutcome::EpochEnds
                                        ) {
                                            return Some(sync_status);
                                        }
                                    },
                                    Err(e) => {
                                        warn!(error = ?e, "error processing rpc");
                                    },
                                };
                                None
                            })
                        })).await;
                        futures.push(f);
                    },
                    Some(status) = futures.next() => {
                        if let Some(status) = status.expect("future must not panic") {
                            ret_tx.send(status).expect("must send");
                            return;
                        }
                    },
                    Some(result) = certified_node_fetch_waiter.next() => {
                        let dag_driver_clone = dag_driver.clone();
                        executor.spawn(async move {
                            monitor!("dag_on_cert_node_fetch", match result {
                                Ok(certified_node) => {
                                    if let Err(e) = dag_driver_clone.process(certified_node).await {
                                        warn!(error = ?e, "error processing certified node fetch notification");
                                    } else {
                                        tokio::task::spawn_blocking(move || dag_driver_clone.fetch_callback());
                                    }
                                },
                                Err(e) => {
                                    debug!("sender dropped channel: {}", e);
                                },
                            });
                        }).await;
                    },
                    Some(result) = node_fetch_waiter.next() => {
                        let node_receiver_clone = node_receiver.clone();
                        let dag_driver_clone = dag_driver.clone();
                        executor.spawn(async move {
                            monitor!("dag_on_node_fetch", match result {
                                Ok(node) => {
                                    if let Err(e) = node_receiver_clone.process(node).await {
                                        warn!(error = ?e, "error processing node fetch notification");
                                    } else {
                                        tokio::task::spawn_blocking(move || dag_driver_clone.fetch_callback());
                                    }
                                },
                                Err(e) => {
                                    debug!("sender dropped channel: {}", e);
                                },
                            });
                        }).await;
                    },
                }
            }
        });
        defer!(h.abort());

        loop {
            select! {
                rpc_request = dag_rpc_rx.select_next_some() => {
                    RPC_PROCESS_DURATION
                        .with_label_values(&["dag_handler"])
                        .observe(rpc_request.start.elapsed().as_secs_f64());
                    let tx = verified_msg_stream_tx.clone();
                    let epoch_state = epoch_state.clone();
                    tokio::task::spawn_blocking(move || {
                        let timer = INCOMING_MSG_PROCESSING.start_timer();
                        defer!(drop(timer));
                        let epoch = rpc_request.req.epoch();
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
                        tx.send((
                            result,
                            epoch,
                            rpc_request.sender,
                            rpc_request.responder,
                            rpc_request.start,
                        ))
                    });
                },
                Ok(ret) = &mut ret_rx => {
                    return ret;
                }
            }
        }
    }
}

struct VerifiedMessageProcessor {
    dag_id: u8,
    node_receiver: Arc<NodeBroadcastHandler>,
    dag_driver: Arc<DagDriver>,
    fetch_receiver: FetchRequestHandler,
    state_sync_trigger: StateSyncTrigger,
    epoch_state: Arc<EpochState>,
    payload_store: Arc<DashMap<(Round, Author), Node>>,
}

impl VerifiedMessageProcessor {
    async fn process_verified_message(
        &self,
        dag_message_result: anyhow::Result<DAGMessage>,
        epoch: u64,
        author: Author,
        responder: RpcResponder,
        start: Instant,
    ) -> anyhow::Result<SyncOutcome> {
        RPC_PROCESS_DURATION
            .with_label_values(&["dag_process"])
            .observe(start.elapsed().as_secs_f64());

        let response: Result<DAGMessage, DAGError> = {
            match dag_message_result {
                Ok(dag_message) => {
                    debug!(
                        dag_id = self.dag_id,
                        epoch = epoch,
                        author = author,
                        message = dag_message,
                        "Verified DAG message"
                    );
                    match dag_message {
                        DAGMessage::NodeMsg(node) => monitor!(
                            "dag_on_node_msg",
                            self.node_receiver
                                .process(node)
                                .await
                                .map(|r| r.into())
                                .map_err(|err| {
                                    err.downcast::<NodeBroadcastHandleError>()
                                        .map_or(DAGError::Unknown, |err| {
                                            DAGError::NodeBroadcastHandleError(err)
                                        })
                                })
                        ),
                        DAGMessage::NodeCertificateMsg(node_certificate_msg) => {
                            monitor!("dag_on_cert_node_msg", {
                                match monitor!(
                                    "dag_state_sync_trigger_check",
                                    self.state_sync_trigger.check(node_certificate_msg).await
                                )? {
                                    SyncOutcome::Synced(Some(certified_node_msg)) => {
                                        let node_metadata = certified_node_msg.metadata();
                                        if let Some((_, node)) = self.payload_store.remove(&(
                                            node_metadata.round(),
                                            *node_metadata.author(),
                                        )) {
                                            let certified_node = CertifiedNode::new(
                                                node,
                                                certified_node_msg.signatures().clone(),
                                            );
                                            self.dag_driver.process(certified_node).await.map(|r| {
                                                let driver = self.dag_driver.clone();
                                                tokio::task::spawn_blocking(move || {
                                                    driver.check_new_round();
                                                });
                                                r.into()
                                            })
                                        } else {
                                            Err(anyhow!(DagDriverError::PayloadNotFound))
                                        }
                                        .map_err(|err| {
                                            err.downcast::<DagDriverError>()
                                                .map_or(DAGError::Unknown, |err| {
                                                    DAGError::DagDriverError(err)
                                                })
                                        })
                                    },
                                    status @ (SyncOutcome::NeedsSync(_)
                                    | SyncOutcome::EpochEnds) => return Ok(status),
                                    _ => unreachable!(),
                                }
                            })
                        },
                        DAGMessage::FetchRequest(request) => monitor!(
                            "dag_on_fetch_request",
                            self.fetch_receiver
                                .process(request)
                                .await
                                .map(|r| r.into())
                                .map_err(|err| {
                                    err.downcast::<FetchRequestHandleError>().map_or(
                                        DAGError::Unknown,
                                        DAGError::FetchRequestHandleError,
                                    )
                                })
                        ),
                        _ => unreachable!("verification must catch this error"),
                    }
                },
                Err(err) => {
                    sample!(
                        SampleRate::Duration(Duration::from_millis(100)),
                        error!(error = ?err, "DAG message verification failed")
                    );
                    Err(DAGError::MessageVerificationError)
                },
            }
        };

        debug!(
            dag_id = self.dag_id,
            epoch = epoch,
            sender = author,
            response = response.as_ref().map(|r| r.name()),
            "RPC response"
        );

        let response: DAGRpcResult = response
            .map_err(|e| DAGRpcError::new(self.dag_id, self.epoch_state.epoch, e))
            .into();
        responder.respond(response)?;
        RPC_PROCESS_DURATION
            .with_label_values(&["dag_reply"])
            .observe(start.elapsed().as_secs_f64());

        Ok(SyncOutcome::Synced(None))
    }
}
