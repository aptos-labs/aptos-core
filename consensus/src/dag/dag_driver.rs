// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    dag_fetcher::FetchRequester,
    order_rule::OrderRule,
    storage::DAGStorage,
    types::{CertifiedAck, CertifiedNodeMessage, DAGMessage, Extensions},
    RpcHandler,
};
use crate::{
    dag::{
        dag_fetcher::TFetchRequester,
        dag_state_sync::DAG_WINDOW,
        dag_store::Dag,
        types::{CertificateAckState, CertifiedNode, Node, NodeCertificate, SignatureBuilder},
    },
    state_replication::PayloadClient,
};
use anyhow::bail;
use aptos_consensus_types::common::{Author, PayloadFilter};
use aptos_infallible::RwLock;
use aptos_logger::{debug, error};
use aptos_reliable_broadcast::ReliableBroadcast;
use aptos_time_service::{TimeService, TimeServiceTrait};
use aptos_types::{block_info::Round, epoch_state::EpochState};
use async_trait::async_trait;
use futures::{
    executor::block_on,
    future::{AbortHandle, Abortable},
    FutureExt,
};
use std::{sync::Arc, time::Duration};
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
        time_service: TimeService,
        storage: Arc<dyn DAGStorage>,
        order_rule: OrderRule,
        fetch_requester: Arc<FetchRequester>,
    ) -> Self {
        let pending_node = storage
            .get_pending_node()
            .expect("should be able to read dag storage");
        let highest_round = dag.read().highest_round();
        let current_round = dag
            .read()
            .get_strong_links_for_round(highest_round, &epoch_state.verifier)
            .map_or_else(|| highest_round.saturating_sub(1), |_| highest_round);

        debug!(
            "highest_round: {}, current_round: {}",
            highest_round, current_round
        );

        let mut driver = Self {
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
        };

        // If we were broadcasting the node for the round already, resume it
        if let Some(node) = pending_node.filter(|node| node.round() == current_round + 1) {
            driver.current_round = node.round();
            driver.broadcast_node(node);
        } else {
            // kick start a new round
            let strong_links = driver
                .dag
                .read()
                .get_strong_links_for_round(current_round, &driver.epoch_state.verifier)
                .unwrap_or(vec![]);
            block_on(driver.enter_new_round(current_round + 1, strong_links));
        }
        driver
    }

    pub async fn add_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let maybe_strong_links = {
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
                dag_writer
                    .get_strong_links_for_round(self.current_round, &self.epoch_state.verifier)
            } else {
                None
            }
        };

        if let Some(strong_links) = maybe_strong_links {
            self.enter_new_round(self.current_round + 1, strong_links)
                .await;
        }
        Ok(())
    }

    pub async fn enter_new_round(&mut self, new_round: Round, strong_links: Vec<NodeCertificate>) {
        debug!("entering new round {}", new_round);
        let payload_filter = {
            let dag_reader = self.dag.read();
            let highest_commit_round = dag_reader.highest_committed_anchor_round();
            if strong_links.is_empty() {
                PayloadFilter::Empty
            } else {
                PayloadFilter::from(
                    &dag_reader
                        .reachable(
                            strong_links.iter().map(|node| node.metadata()),
                            Some(highest_commit_round.saturating_sub(DAG_WINDOW as u64)),
                            |_| true,
                        )
                        .map(|node_status| node_status.as_node().payload())
                        .collect(),
                )
            }
        };
        let payload = match self
            .payload_client
            .pull_payload(
                Duration::from_secs(1),
                100,
                1000,
                payload_filter,
                Box::pin(async {}),
                false,
                0,
                0.0,
            )
            .await
        {
            Ok(payload) => payload,
            Err(e) => {
                error!("error pulling payload: {}", e);
                return;
            },
        };
        // TODO: need to wait to pass median of parents timestamp
        let timestamp = self.time_service.now_unix_time();
        self.current_round = new_round;
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
            .save_pending_node(&new_node)
            .expect("node must be saved");
        self.broadcast_node(new_node);
    }

    pub fn broadcast_node(&mut self, node: Node) {
        let rb = self.reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let signature_builder =
            SignatureBuilder::new(node.metadata().clone(), self.epoch_state.clone());
        let cert_ack_set = CertificateAckState::new(self.epoch_state.verifier.len());
        let latest_ledger_info = self
            .storage
            .get_latest_ledger_info()
            .expect("latest ledger info must exist");
        let task = self
            .reliable_broadcast
            .broadcast(node.clone(), signature_builder)
            .then(move |certificate| {
                let certified_node = CertifiedNode::new(node, certificate.signatures().to_owned());
                let certified_node_msg =
                    CertifiedNodeMessage::new(certified_node, latest_ledger_info);
                rb.broadcast(certified_node_msg, cert_ack_set)
            });
        tokio::spawn(Abortable::new(task, abort_registration));
        if let Some(prev_handle) = self.rb_abort_handle.replace(abort_handle) {
            prev_handle.abort();
        }
    }
}

#[async_trait]
impl RpcHandler for DagDriver {
    type Request = CertifiedNode;
    type Response = CertifiedAck;

    async fn process(&mut self, node: Self::Request) -> anyhow::Result<Self::Response> {
        let epoch = node.metadata().epoch();
        {
            let dag_reader = self.dag.read();
            if dag_reader.exists(node.metadata()) {
                return Ok(CertifiedAck::new(epoch));
            }
        }

        let node_metadata = node.metadata().clone();
        self.add_node(node)
            .await
            .map(|_| self.order_rule.process_new_node(&node_metadata))?;

        Ok(CertifiedAck::new(epoch))
    }
}
