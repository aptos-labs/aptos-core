// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{dag_store::DagStore, DAGRpcResult};
use crate::dag::{
    dag_network::{RpcResultWithResponder, TDAGNetworkSender},
    errors::FetchRequestHandleError,
    observability::logging::{LogEvent, LogSchema},
    types::{CertifiedNode, FetchResponse, Node, NodeMetadata, RemoteFetchRequest},
    RpcHandler, RpcWithFallback,
};
use anyhow::{anyhow, ensure};
use aptos_bitvec::BitVec;
use aptos_config::config::DagFetcherConfig;
use aptos_consensus_types::common::Author;
use aptos_logger::{debug, error, info};
use aptos_time_service::TimeService;
use aptos_types::epoch_state::EpochState;
use async_trait::async_trait;
use futures::{
    stream::{FusedStream, FuturesUnordered},
    Stream, StreamExt,
};
use std::{
    collections::HashMap,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    oneshot,
};

pub struct FetchWaiter<T> {
    rx: Receiver<oneshot::Receiver<T>>,
    futures: Pin<Box<FuturesUnordered<oneshot::Receiver<T>>>>,
    rx_terminated: bool,
}

impl<T> FetchWaiter<T> {
    fn new(rx: Receiver<oneshot::Receiver<T>>) -> Self {
        Self {
            rx,
            futures: Box::pin(FuturesUnordered::new()),
            rx_terminated: false,
        }
    }
}

impl<T> Stream for FetchWaiter<T> {
    type Item = Result<T, oneshot::error::RecvError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.is_terminated() {
            return Poll::Ready(None);
        }

        if self.rx_terminated {
            return self.futures.as_mut().poll_next(cx);
        }

        match self.rx.poll_recv(cx) {
            Poll::Ready(Some(rx)) => {
                self.futures.push(rx);
            },
            Poll::Ready(None) => {
                self.rx_terminated = true;
                if self.futures.is_empty() {
                    return Poll::Ready(None);
                }
            },
            Poll::Pending => {
                if self.futures.is_empty() {
                    return Poll::Pending;
                }
            },
        };

        debug_assert!(!self.futures.is_empty());
        self.futures.as_mut().poll_next(cx)
    }
}

impl<T> FusedStream for FetchWaiter<T> {
    fn is_terminated(&self) -> bool {
        self.rx_terminated && self.futures.is_terminated()
    }
}

pub trait TFetchRequester: Send + Sync {
    fn request_for_node(&self, node: Node) -> anyhow::Result<()>;
    fn request_for_certified_node(&self, node: CertifiedNode) -> anyhow::Result<()>;
}

pub struct FetchRequester {
    request_tx: Sender<LocalFetchRequest>,
    node_waiter_tx: Sender<oneshot::Receiver<Node>>,
    certified_node_waiter_tx: Sender<oneshot::Receiver<CertifiedNode>>,
}

impl TFetchRequester for FetchRequester {
    fn request_for_node(&self, node: Node) -> anyhow::Result<()> {
        let (res_tx, res_rx) = oneshot::channel();
        let fetch_req = LocalFetchRequest::Node(node, res_tx);
        self.request_tx
            .try_send(fetch_req)
            .map_err(|e| anyhow::anyhow!("unable to send node fetch request to channel: {}", e))?;
        self.node_waiter_tx.try_send(res_rx)?;
        Ok(())
    }

    fn request_for_certified_node(&self, node: CertifiedNode) -> anyhow::Result<()> {
        let (res_tx, res_rx) = oneshot::channel();
        let fetch_req = LocalFetchRequest::CertifiedNode(node, res_tx);
        self.request_tx.try_send(fetch_req).map_err(|e| {
            anyhow::anyhow!(
                "unable to send certified node fetch request to channel: {}",
                e
            )
        })?;
        self.certified_node_waiter_tx.try_send(res_rx)?;
        Ok(())
    }
}

#[derive(Debug)]
pub enum LocalFetchRequest {
    Node(Node, oneshot::Sender<Node>),
    CertifiedNode(CertifiedNode, oneshot::Sender<CertifiedNode>),
}

impl LocalFetchRequest {
    pub fn responders(&self, validators: &[Author]) -> Vec<Author> {
        match self {
            LocalFetchRequest::Node(node, _) => vec![*node.author()],
            LocalFetchRequest::CertifiedNode(node, _) => {
                node.signatures().get_signers_addresses(validators)
            },
        }
    }

    pub fn notify(self) {
        if match self {
            LocalFetchRequest::Node(node, sender) => sender.send(node).map_err(|_| ()),
            LocalFetchRequest::CertifiedNode(node, sender) => sender.send(node).map_err(|_| ()),
        }
        .is_err()
        {
            error!("Failed to send node back");
        }
    }

    pub fn node(&self) -> &Node {
        match self {
            LocalFetchRequest::Node(node, _) => node,
            LocalFetchRequest::CertifiedNode(node, _) => node,
        }
    }
}

pub struct DagFetcherService {
    inner: DagFetcher,
    dag: Arc<DagStore>,
    request_rx: Receiver<LocalFetchRequest>,
    ordered_authors: Vec<Author>,
}

impl DagFetcherService {
    pub fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn TDAGNetworkSender>,
        dag: Arc<DagStore>,
        time_service: TimeService,
        config: DagFetcherConfig,
    ) -> (
        Self,
        FetchRequester,
        FetchWaiter<Node>,
        FetchWaiter<CertifiedNode>,
    ) {
        let (request_tx, request_rx) = tokio::sync::mpsc::channel(16);
        let (node_tx, node_rx) = tokio::sync::mpsc::channel(100);
        let (certified_node_tx, certified_node_rx) = tokio::sync::mpsc::channel(100);
        let ordered_authors = epoch_state.verifier.get_ordered_account_addresses();
        (
            Self {
                inner: DagFetcher::new(epoch_state, network, time_service, config),
                dag,
                request_rx,
                ordered_authors,
            },
            FetchRequester {
                request_tx,
                node_waiter_tx: node_tx,
                certified_node_waiter_tx: certified_node_tx,
            },
            FetchWaiter::new(node_rx),
            FetchWaiter::new(certified_node_rx),
        )
    }

    pub async fn start(mut self) {
        while let Some(local_request) = self.request_rx.recv().await {
            match self
                .fetch(
                    local_request.node(),
                    local_request.responders(&self.ordered_authors),
                )
                .await
            {
                Ok(_) => local_request.notify(),
                Err(err) => error!("unable to complete fetch successfully: {}", err),
            }
        }
    }

    pub(super) async fn fetch(
        &mut self,
        node: &Node,
        responders: Vec<Author>,
    ) -> anyhow::Result<()> {
        let remote_request = {
            let dag_reader = self.dag.read();
            ensure!(
                node.round() > dag_reader.lowest_incomplete_round(),
                "Already synced beyond requested round {}, lowest incomplete round {}",
                node.round(),
                dag_reader.lowest_incomplete_round()
            );

            let missing_parents: Vec<NodeMetadata> = dag_reader
                .filter_missing(node.parents_metadata())
                .cloned()
                .collect();

            if missing_parents.is_empty() {
                return Ok(());
            }

            RemoteFetchRequest::new(
                node.metadata().epoch(),
                missing_parents,
                dag_reader.bitmask(node.round().saturating_sub(1)),
            )
        };
        self.inner
            .fetch(remote_request, responders, self.dag.clone())
            .await
    }
}

#[async_trait]
pub trait TDagFetcher: Send {
    async fn fetch(
        &self,
        remote_request: RemoteFetchRequest,
        responders: Vec<Author>,
        dag: Arc<DagStore>,
    ) -> anyhow::Result<()>;
}

pub(crate) struct DagFetcher {
    network: Arc<dyn TDAGNetworkSender>,
    time_service: TimeService,
    epoch_state: Arc<EpochState>,
    config: DagFetcherConfig,
}

impl DagFetcher {
    pub(crate) fn new(
        epoch_state: Arc<EpochState>,
        network: Arc<dyn TDAGNetworkSender>,
        time_service: TimeService,
        config: DagFetcherConfig,
    ) -> Self {
        Self {
            network,
            time_service,
            epoch_state,
            config,
        }
    }
}

#[async_trait]
impl TDagFetcher for DagFetcher {
    async fn fetch(
        &self,
        remote_request: RemoteFetchRequest,
        responders: Vec<Author>,
        dag: Arc<DagStore>,
    ) -> anyhow::Result<()> {
        debug!(
            LogSchema::new(LogEvent::FetchNodes),
            start_round = remote_request.start_round(),
            target_round = remote_request.target_round(),
            lens = remote_request.exists_bitmask().len(),
            missing_nodes = remote_request.exists_bitmask().num_missing(),
        );
        let mut rpc = RpcWithFallback::new(
            responders,
            remote_request.clone().into(),
            Duration::from_millis(self.config.retry_interval_ms),
            Duration::from_millis(self.config.rpc_timeout_ms),
            self.network.clone(),
            self.time_service.clone(),
            self.config.min_concurrent_responders,
            self.config.max_concurrent_responders,
        );

        while let Some(RpcResultWithResponder { responder, result }) = rpc.next().await {
            match result {
                Ok(DAGRpcResult(Ok(response))) => {
                    match FetchResponse::try_from(response).and_then(|response| {
                        response.verify(&remote_request, &self.epoch_state.verifier)
                    }) {
                        Ok(fetch_response) => {
                            let certified_nodes = fetch_response.certified_nodes();
                            // TODO: support chunk response or fallback to state sync
                            {
                                for node in certified_nodes.into_iter().rev() {
                                    if let Err(e) = dag.add_node(node) {
                                        error!(error = ?e, "failed to add node");
                                    }
                                }
                            }

                            if dag.read().all_exists(remote_request.targets()) {
                                return Ok(());
                            }
                        },
                        Err(err) => {
                            info!(error = ?err, "failure parsing/verifying fetch response from {}", responder);
                        },
                    };
                },
                Ok(DAGRpcResult(Err(dag_rpc_error))) => {
                    info!(error = ?dag_rpc_error, responder = responder, "fetch failure: target {} returned error", responder);
                },
                Err(err) => {
                    info!(error = ?err, responder = responder, "rpc failed to {}", responder);
                },
            }
        }
        Err(anyhow!("Fetch with fallback failed"))
    }
}

pub struct FetchRequestHandler {
    dag: Arc<DagStore>,
    author_to_index: HashMap<Author, usize>,
}

impl FetchRequestHandler {
    pub fn new(dag: Arc<DagStore>, epoch_state: Arc<EpochState>) -> Self {
        Self {
            dag,
            author_to_index: epoch_state.verifier.address_to_validator_index().clone(),
        }
    }
}

#[async_trait]
impl RpcHandler for FetchRequestHandler {
    type Request = RemoteFetchRequest;
    type Response = FetchResponse;

    async fn process(&self, message: Self::Request) -> anyhow::Result<Self::Response> {
        let dag_reader = self.dag.read();

        // `Certified Node`: In the good case, there should exist at least one honest validator that
        // signed the Certified Node that has the all the parents to fulfil this
        // request.
        // `Node`: In the good case, the sender of the Node should have the parents in its local DAG
        // to satisfy this request.
        debug!(
            LogSchema::new(LogEvent::ReceiveFetchNodes).round(dag_reader.highest_round()),
            start_round = message.start_round(),
            target_round = message.target_round(),
        );
        ensure!(
            dag_reader.lowest_round() <= message.start_round(),
            FetchRequestHandleError::GarbageCollected(
                message.start_round(),
                dag_reader.lowest_round()
            ),
        );

        let missing_targets: BitVec = message
            .targets()
            .map(|node| !dag_reader.exists(node))
            .collect();
        ensure!(
            missing_targets.all_zeros(),
            FetchRequestHandleError::TargetsMissing(missing_targets)
        );

        let certified_nodes: Vec<_> = dag_reader
            .reachable(
                message.targets(),
                Some(message.exists_bitmask().first_round()),
                |_| true,
            )
            .filter_map(|node_status| {
                let arc_node = node_status.as_node();
                self.author_to_index
                    .get(arc_node.author())
                    .and_then(|author_idx| {
                        if !message.exists_bitmask().has(arc_node.round(), *author_idx) {
                            Some(arc_node.as_ref().clone())
                        } else {
                            None
                        }
                    })
            })
            .collect();

        // TODO: decide if the response is too big and act accordingly.

        Ok(FetchResponse::new(message.epoch(), certified_nodes))
    }
}
