// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use crate::experimental::ordering_state_computer::OrderingStateComputer;
use aptos_consensus_types::node::{CertifiedNode, Node};
use std::sync::Arc;
use claims::assert_some;
use tokio::sync::mpsc::Receiver;
use aptos_consensus_types::common::Round;
use aptos_types::PeerId;
use aptos_types::validator_verifier::ValidatorVerifier;
use crate::dag::anchor_election::AnchorElection;

#[allow(dead_code)]
pub struct Bullshark {
    state_computer: Arc<OrderingStateComputer>,
    dag: Vec<HashMap<PeerId, Node>>,
    lowest_ordered_anchor_wave: u64,
    proposer_election: Arc<dyn AnchorElection>,
    verifier: ValidatorVerifier,
}

#[allow(dead_code)]
impl Bullshark {
    pub fn new(state_computer: Arc<OrderingStateComputer>, proposer_election: Arc<dyn AnchorElection>, verifier: ValidatorVerifier) -> Self
    {
        Self {
            state_computer,
            dag: Vec::new(),
            lowest_ordered_anchor_wave: 0,
            proposer_election,
            verifier,
        }
    }

    fn path(&self, source: &Node, target: &Node) -> bool {
        todo!()
    }

    fn order_anchors(&self, anchor: Node) {}

    pub fn try_ordering(&mut self, node: Node) {
        let round = node.round();
        let wave = round / 2;
        let author = node.source();

        assert!(!self.dag
            .get(round as usize)
            .map_or(false, |m| m.contains_key(&author)));

        if self.dag.len() < round as usize {
            assert_some!(self.dag.get(round as usize -1));
            self.dag.push(HashMap::new());
        }

        self.dag[round as usize].insert(author, node);


        if round % 2 == 0 || wave < self.lowest_ordered_anchor_wave {
            return;
        }

        // We have one more potential vote in a wave we have not previously ordered
        let anchor_author = self.proposer_election.get_round_anchor(wave);
        let voters = self.dag
            .get(round as usize)
            .unwrap()
            .iter()
            .filter(|(_, node)| node.strong_links().contains(&anchor_author))
            .map(|(peer_id, _)| peer_id);


        if self.verifier
            .check_minority_voting_power(voters)
            .is_ok() {
            let anchor = self.dag[round as usize - 1].remove(&anchor_author).unwrap()
            self.order_anchors(anchor);
        }
    }


    pub fn pending_payload(&self) {}

    pub async fn start(self, mut rx: Receiver<CertifiedNode>) {
        loop {
            tokio::select! {
            Some(_) = rx.recv() => {

            }
                }
        }
    }
}
