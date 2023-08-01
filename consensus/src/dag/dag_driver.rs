// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{storage::DAGStorage, types::DAGMessage};
use crate::{
    dag::{
        dag_store::Dag,
        types::{CertificateAckState, CertifiedNode, Node, NodeCertificate, SignatureBuilder},
    },
    state_replication::PayloadClient,
    util::time_service::TimeService,
};
use aptos_consensus_types::common::{Author, Payload};
use aptos_infallible::RwLock;
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_types::{block_info::Round, epoch_state::EpochState};
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use std::sync::Arc;
use tokio_retry::strategy::ExponentialBackoff;

pub(crate) struct DagDriver {
    author: Author,
    epoch_state: Arc<EpochState>,
    dag: Arc<RwLock<Dag>>,
    payload_client: Arc<dyn PayloadClient>,
    reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff>>,
    current_round: Round,
    time_service: Arc<dyn TimeService>,
    rb_abort_handle: Option<AbortHandle>,
    storage: Arc<dyn DAGStorage>,
}

impl DagDriver {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        dag: Arc<RwLock<Dag>>,
        payload_client: Arc<dyn PayloadClient>,
        reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff>>,
        current_round: Round,
        time_service: Arc<dyn TimeService>,
        storage: Arc<dyn DAGStorage>,
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
        }
    }

    pub fn add_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let mut dag_writer = self.dag.write();
        let round = node.metadata().round();
        if dag_writer.all_exists(node.parents_metadata()) {
            dag_writer.add_node(node)?;
            if self.current_round == round {
                let maybe_strong_links = dag_writer
                    .get_strong_links_for_round(self.current_round, &self.epoch_state.verifier);
                drop(dag_writer);
                if let Some(strong_links) = maybe_strong_links {
                    self.enter_new_round(strong_links);
                }
            }
        }
        // TODO: handle fetching missing dependencies
        Ok(())
    }

    pub fn enter_new_round(&mut self, strong_links: Vec<NodeCertificate>) {
        // TODO: support pulling payload
        let payload = Payload::empty(false);
        // TODO: need to wait to pass median of parents timestamp
        let timestamp = self.time_service.get_current_timestamp();
        self.current_round += 1;
        let new_node = Node::new(
            self.epoch_state.epoch,
            self.current_round,
            self.author,
            timestamp.as_micros() as u64,
            payload,
            strong_links,
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
