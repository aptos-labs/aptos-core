// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use super::{
    adapter::TLedgerInfoProvider,
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
        observability::{
            counters,
            logging::{LogEvent, LogSchema},
            tracing::{observe_node, observe_round, NodeStage, RoundStage},
        },
        round_state::RoundState,
        types::{CertificateAckState, CertifiedNode, Node, SignatureBuilder},
    },
    payload_manager::PayloadManager,
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
    payload_manager: Arc<PayloadManager>,
    payload_client: Arc<dyn PayloadClient>,
    reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff>>,
    time_service: TimeService,
    rb_abort_handle: Option<(AbortHandle, u64)>,
    storage: Arc<dyn DAGStorage>,
    order_rule: OrderRule,
    fetch_requester: Arc<FetchRequester>,
    ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
    round_state: RoundState,
}

impl DagDriver {
    pub fn new(
        author: Author,
        epoch_state: Arc<EpochState>,
        dag: Arc<RwLock<Dag>>,
        payload_manager: Arc<PayloadManager>,
        payload_client: Arc<dyn PayloadClient>,
        reliable_broadcast: Arc<ReliableBroadcast<DAGMessage, ExponentialBackoff>>,
        time_service: TimeService,
        storage: Arc<dyn DAGStorage>,
        order_rule: OrderRule,
        fetch_requester: Arc<FetchRequester>,
        ledger_info_provider: Arc<dyn TLedgerInfoProvider>,
        round_state: RoundState,
    ) -> Self {
        let pending_node = storage
            .get_pending_node()
            .expect("should be able to read dag storage");
        let highest_strong_links_round =
            dag.read().highest_strong_links_round(&epoch_state.verifier);

        let mut driver = Self {
            author,
            epoch_state,
            dag,
            payload_manager,
            payload_client,
            reliable_broadcast,
            time_service,
            rb_abort_handle: None,
            storage,
            order_rule,
            fetch_requester,
            ledger_info_provider,
            round_state,
        };

        // If we were broadcasting the node for the round already, resume it
        if let Some(node) =
            pending_node.filter(|node| node.round() == highest_strong_links_round + 1)
        {
            debug!(
                LogSchema::new(LogEvent::NewRound).round(node.round()),
                "Resume round"
            );
            driver.round_state.set_current_round(node.round());
            driver.broadcast_node(node);
        } else {
            // kick start a new round
            block_on(driver.enter_new_round(highest_strong_links_round + 1));
        }
        driver
    }

    async fn add_node(&mut self, node: CertifiedNode) -> anyhow::Result<()> {
        let (highest_strong_link_round, strong_links) = {
            let mut dag_writer = self.dag.write();

            if !dag_writer.all_exists(node.parents_metadata()) {
                if let Err(err) = self.fetch_requester.request_for_certified_node(node) {
                    error!("request to fetch failed: {}", err);
                }
                bail!(DagDriverError::MissingParents);
            }

            self.payload_manager
                .prefetch_payload_data(node.payload(), node.metadata().timestamp());
            dag_writer.add_node(node)?;

            let highest_strong_links_round =
                dag_writer.highest_strong_links_round(&self.epoch_state.verifier);
            (
                highest_strong_links_round,
                // unwrap is for round 0
                dag_writer
                    .get_strong_links_for_round(
                        highest_strong_links_round,
                        &self.epoch_state.verifier,
                    )
                    .unwrap_or(vec![]),
            )
        };
        self.round_state
            .check_for_new_round(highest_strong_link_round, strong_links)
            .await;
        Ok(())
    }

    pub async fn enter_new_round(&mut self, new_round: Round) {
        if self.round_state.current_round() >= new_round {
            return;
        }
        debug!(LogSchema::new(LogEvent::NewRound).round(new_round));
        self.round_state.set_current_round(new_round);
        counters::CURRENT_ROUND.set(new_round as i64);
        let strong_links = self
            .dag
            .read()
            .get_strong_links_for_round(new_round - 1, &self.epoch_state.verifier)
            .unwrap_or_else(|| {
                assert_eq!(new_round, 1, "Only expect empty strong links for round 1");
                vec![]
            });
        let payload_filter = {
            let dag_reader = self.dag.read();
            let highest_commit_round = self
                .ledger_info_provider
                .get_highest_committed_anchor_round();
            if strong_links.is_empty() {
                PayloadFilter::Empty
            } else {
                PayloadFilter::from(
                    &dag_reader
                        .reachable(
                            strong_links.iter().map(|node| node.metadata()),
                            Some(highest_commit_round.saturating_sub(DAG_WINDOW)),
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
                1000,
                10 * 1024 * 1024,
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
                // TODO: return empty payload instead
                panic!("error pulling payload: {}", e);
            },
        };
        // TODO: need to wait to pass median of parents timestamp
        let highest_parent_timestamp = strong_links
            .iter()
            .map(|node| node.metadata().timestamp())
            .max()
            .unwrap_or(0);
        let timestamp = std::cmp::max(
            self.time_service.now_unix_time().as_micros() as u64,
            highest_parent_timestamp + 1,
        );
        let new_node = Node::new(
            self.epoch_state.epoch,
            new_round,
            self.author,
            timestamp,
            payload,
            strong_links,
            Extensions::empty(),
        );
        self.storage
            .save_pending_node(&new_node)
            .expect("node must be saved");
        self.broadcast_node(new_node);
    }

    fn broadcast_node(&mut self, node: Node) {
        let rb = self.reliable_broadcast.clone();
        let rb2 = self.reliable_broadcast.clone();
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let signature_builder =
            SignatureBuilder::new(node.metadata().clone(), self.epoch_state.clone());
        let cert_ack_set = CertificateAckState::new(self.epoch_state.verifier.len());
        let latest_ledger_info = self.ledger_info_provider.clone();

        let round = node.round();
        let node_clone = node.clone();
        let timestamp = node.timestamp();
        let node_broadcast = async move {
            debug!(LogSchema::new(LogEvent::BroadcastNode), id = node.id());

            defer!( observe_round(timestamp, RoundStage::NodeBroadcasted); );
            rb.broadcast(node, signature_builder).await
        };
        let core_task = node_broadcast.then(move |certificate| {
            debug!(
                LogSchema::new(LogEvent::BroadcastCertifiedNode),
                id = node_clone.id()
            );

            defer!( observe_round(timestamp, RoundStage::CertifiedNodeBroadcasted); );
            let certified_node =
                CertifiedNode::new(node_clone, certificate.signatures().to_owned());
            let certified_node_msg = CertifiedNodeMessage::new(
                certified_node,
                latest_ledger_info.get_latest_ledger_info(),
            );
            rb2.broadcast(certified_node_msg, cert_ack_set)
        });
        let task = async move {
            debug!("Start reliable broadcast for round {}", round);
            core_task.await;
            debug!("Finish reliable broadcast for round {}", round);
        };
        tokio::spawn(Abortable::new(task, abort_registration));
        if let Some((prev_handle, prev_round_timestamp)) =
            self.rb_abort_handle.replace((abort_handle, timestamp))
        {
            observe_round(prev_round_timestamp, RoundStage::Finished);
            prev_handle.abort();
        }
    }
}

#[async_trait]
impl RpcHandler for DagDriver {
    type Request = CertifiedNode;
    type Response = CertifiedAck;

    async fn process(&mut self, certified_node: Self::Request) -> anyhow::Result<Self::Response> {
        let epoch = certified_node.metadata().epoch();
        debug!(LogSchema::new(LogEvent::ReceiveCertifiedNode)
            .remote_peer(*certified_node.author())
            .round(certified_node.round()));
        {
            let dag_reader = self.dag.read();
            if dag_reader.exists(certified_node.metadata()) {
                return Ok(CertifiedAck::new(epoch));
            }
        }
        observe_node(certified_node.timestamp(), NodeStage::CertifiedNodeReceived);

        let node_metadata = certified_node.metadata().clone();
        self.add_node(certified_node)
            .await
            .map(|_| self.order_rule.process_new_node(&node_metadata))?;

        Ok(CertifiedAck::new(epoch))
    }
}
