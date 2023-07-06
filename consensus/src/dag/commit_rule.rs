// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{anchor_election::AnchorElection, dag_store::Dag, CertifiedNode},
    experimental::buffer_manager::OrderedBlocks,
};
use aptos_consensus_types::common::Round;
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_types::{epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures};
use futures_channel::mpsc::UnboundedSender;
use std::sync::Arc;

pub struct CommitRule {
    epoch_state: Arc<EpochState>,
    ordered_block_id: HashValue,
    ordered_round: Round,
    dag: Arc<RwLock<Dag>>,
    anchor_election: Box<dyn AnchorElection>,
    commit_sender: UnboundedSender<OrderedBlocks>,
}

impl CommitRule {
    pub fn new(
        epoch_state: Arc<EpochState>,
        latest_ledger_info: LedgerInfoWithSignatures,
        dag: Arc<RwLock<Dag>>,
        anchor_election: Box<dyn AnchorElection>,
        commit_sender: UnboundedSender<OrderedBlocks>,
    ) -> Self {
        // TODO: we need to initialize the anchor election based on the dag
        Self {
            epoch_state,
            ordered_block_id: latest_ledger_info.commit_info().id(),
            ordered_round: latest_ledger_info.commit_info().round(),
            dag,
            anchor_election,
            commit_sender,
        }
    }

    pub fn new_node(&mut self, node: &CertifiedNode) {
        let round = node.round();
        while self.ordered_round <= round {
            if let Some(direct_anchor) = self.find_first_anchor_with_enough_votes(round) {
                let commit_anchor = self.find_first_anchor_to_commit(direct_anchor);
                let commit_nodes = self.order_anchor(commit_anchor);
                self.push_to_execution(commit_nodes);
            } else {
                break;
            }
        }
    }

    pub fn find_first_anchor_with_enough_votes(
        &self,
        target_round: Round,
    ) -> Option<Arc<CertifiedNode>> {
        let dag_reader = self.dag.read();
        let mut current_round = self.ordered_round;
        while current_round < target_round {
            let anchor_author = self.anchor_election.get_anchor(current_round);
        }
        None
    }

    pub fn find_first_anchor_to_commit(&mut self, node: Arc<CertifiedNode>) -> Arc<CertifiedNode> {
        node
    }

    pub fn order_anchor(&mut self, anchor: Arc<CertifiedNode>) -> Vec<Arc<CertifiedNode>> {
        vec![anchor]
    }

    pub fn push_to_execution(&mut self, nodes: Vec<Arc<CertifiedNode>>) {}
}
