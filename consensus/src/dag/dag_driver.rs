// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dag_fetcher::FetchRequester,
    order_rule::OrderRule,
    storage::DAGStorage,
    types::{CertifiedAck, DAGMessage, Extensions},
    RpcHandler,
};
use crate::{
    dag::{
        dag_store::Dag,
        types::{CertificateAckState, CertifiedNode, Node, NodeCertificate, SignatureBuilder},
    },
    state_replication::PayloadClient,
};
use anyhow::{bail, Ok};
use aptos_consensus_types::common::{Author, Payload};
use aptos_infallible::RwLock;
use aptos_logger::error;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{block_info::Round, epoch_state::EpochState};
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use std::sync::Arc;
use thiserror::Error as ThisError;
use tokio_retry::strategy::ExponentialBackoff;

#[derive(Debug, ThisError)]
pub enum DagDriverError {
    #[error("missing parents")]
    MissingParents,
}

pub(crate) struct DagDriver {
    author: Author,
    epoch_state: Arc<EpochState>,
    dag: Arc<RwLock<Dag>>,
    payload_client: Arc<dyn PayloadClient>,
    reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff>>,
    current_round: Round,
    time_service: TimeService,
    rb_abort_handle: Option<AbortHandle>,
    storage: Arc<dyn DAGStorage>,
    order_rule: OrderRule,
    fetch_requester: Arc<FetchRequester>,
}

impl DagDriver {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        dag: Arc<RwLock<Dag>>,
        payload_client: Arc<dyn PayloadClient>,
        reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff>>,
        current_round: Round,
        time_service: TimeService,
        storage: Arc<dyn DAGStorage>,
        order_rule: OrderRule,
        fetch_requester: Arc<FetchRequester>,
    ) -> Self {
        // TODO: rebroadcast nodes after recovery
        Self {
            author,
            epoch_state,
            dag,
            payload_client,
            reliable_broadcast,
            current_round,
            time_service,
            rb_abort_handle: None,
            storage,
            order_rule,
            fetch_requester,
        }
    }

    pub fn try_enter_new_round(&mut self) {
        // In case of a new epoch, kickstart building the DAG by entering the next round
        // without any parents.
        if self.current_round == 0 {
            self.enter_new_round(vec![]);
        }
        // TODO: add logic to handle building DAG from the middle, etc.
    }

    pub fn add_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let mut dag_writer = self.dag.write();
        let round = node.metadata().round();

        if !dag_writer.all_exists(node.parents_metadata()) {
            if let Err(err) = self.fetch_requester.request_for_certified_node(node) {
                error!("request to fetch failed: {}", err);
            }
            bail!(DagDriverError::MissingParents);
        }

        dag_writer.add_node(node)?;
        if self.current_round == round {
            let maybe_strong_links = dag_writer
                .get_strong_links_for_round(self.current_round, &self.epoch_state.verifier);
            drop(dag_writer);
            if let Some(strong_links) = maybe_strong_links {
                self.enter_new_round(strong_links);
            }
        }
        Ok(())
    }

    pub fn enter_new_round(&mut self, strong_links: Vec<NodeCertificate>) {
        // TODO: support pulling payload
        let payload = Payload::empty(false);
        // TODO: need to wait to pass median of parents timestamp
        let timestamp = self.time_service.now_unix_time();
        self.current_round += 1;
        let new_node = Node::new(
            self.epoch_state.epoch,
            self.current_round,
            self.author,
            timestamp.as_micros() as u64,
            payload,
            strong_links,
            Extensions::empty(),
        );
        self.storage
            .save_node(&new_node)
            .expect("node must be saved");
        self.broadcast_node(new_node);
    }

    pub fn broadcast_node(&mut self, node: Node) {
        let rb = self.reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let signature_builder =
            SignatureBuilder::new(node.metadata().clone(), self.epoch_state.clone());
        let cert_ack_set = CertificateAckState::new(self.epoch_state.verifier.len());
        let task = self
            .reliable_broadcast
            .broadcast(node.clone(), signature_builder)
            .then(move |certificate| {
                let certified_node = CertifiedNode::new(node, certificate.signatures().to_owned());
                rb.broadcast(certified_node, cert_ack_set)
            });
        tokio::spawn(Abortable::new(task, abort_registration));
        if let Some(prev_handle) = self.rb_abort_handle.replace(abort_handle) {
            prev_handle.abort();
        }
    }
}

impl RpcHandler for DagDriver {
    type Request = CertifiedNode;
    type Response = CertifiedAck;

    fn process(&mut self, node: Self::Request) -> anyhow::Result<Self::Response> {
        let epoch = node.metadata().epoch();
        {
            let dag_reader = self.dag.read();
            if dag_reader.exists(node.metadata()) {
                return Ok(CertifiedAck::new(epoch));
            }
        }

        let node_metadata = node.metadata().clone();
        self.add_node(node)
            .map(|_| self.order_rule.process_new_node(&node_metadata))?;

        Ok(CertifiedAck::new(epoch))
    }
}
