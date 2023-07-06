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
    lowest_unordered_round: Round,
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
            lowest_unordered_round: latest_ledger_info.commit_info().round() + 1,
            dag,
            anchor_election,
            commit_sender,
        }
    }

    pub fn new_node(&mut self, node: &CertifiedNode) {
        let round = node.round();
        while self.lowest_unordered_round <= round {
            if let Some(direct_anchor) = self.find_first_anchor_with_enough_votes(round) {
                let commit_anchor = self.find_first_anchor_to_commit(direct_anchor);
                self.finalize_order(commit_anchor);
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
        let mut current_round = self.lowest_unordered_round;
        while current_round < target_round {
            let anchor_author = self.anchor_election.get_anchor(current_round);
            // I "think" it's impossible to get ordered/committed node here but to double check
            if let Some(anchor_node) =
                dag_reader.get_node_by_round_author(current_round, &anchor_author)
            {
                // f+1 or 2f+1?
                if dag_reader
                    .check_votes_for_node(anchor_node.metadata(), &self.epoch_state.verifier)
                {
                    return Some(anchor_node);
                }
            }
            current_round += 2;
        }
        None
    }

    pub fn find_first_anchor_to_commit(
        &mut self,
        current_anchor: Arc<CertifiedNode>,
    ) -> Arc<CertifiedNode> {
        let dag_reader = self.dag.read();
        let anchor_round = current_anchor.round();
        let first_anchor = dag_reader
            .reachable(&current_anchor)
            .filter(|node| node.round() >= self.lowest_unordered_round)
            // the same parity of the current anchor round
            .filter(|node| (node.round() ^ anchor_round) & 1 == 0)
            // anchor node, we can cache the election result per round
            .filter(|node| *node.author() == self.anchor_election.get_anchor(node.round()))
            .last()
            .unwrap();
        self.lowest_unordered_round = first_anchor.round() + 1;
        first_anchor.clone()
    }

    pub fn finalize_order(&mut self, anchor: Arc<CertifiedNode>) {
        let dag_writer = self.dag.write();
        let _commit_nodes: Vec<_> = dag_writer.reachable(&anchor).collect();
    }
}
