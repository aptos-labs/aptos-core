// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::anchor_election::AnchorElection,
    experimental::ordering_state_computer::OrderingStateComputer,
};
use aptos_consensus_types::node::{CertifiedNode, Node};
use aptos_types::{validator_verifier::ValidatorVerifier, PeerId};
use claims::assert_some;
use itertools::Itertools;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::mpsc::Receiver;

#[allow(dead_code)]
pub struct Bullshark {
    state_computer: Arc<OrderingStateComputer>,
    dag: Vec<HashMap<PeerId, Node>>,
    lowest_unordered_anchor_wave: u64,
    proposer_election: Arc<dyn AnchorElection>,
    verifier: ValidatorVerifier,
}

#[allow(dead_code)]
impl Bullshark {
    pub fn new(
        state_computer: Arc<OrderingStateComputer>,
        proposer_election: Arc<dyn AnchorElection>,
        verifier: ValidatorVerifier,
    ) -> Self {
        Self {
            state_computer,
            dag: Vec::new(),
            lowest_unordered_anchor_wave: 0,
            proposer_election,
            verifier,
        }
    }

    fn strong_path(&self, source: &Node, target: &Node) -> bool {
        let target_round = target.round();
        let mut round = source.round();

        let mut reachable_nodes = HashMap::new();
        reachable_nodes.insert(source.digest(), source);

        while round > target_round {
            let mut new_reachable_nodes = HashMap::new();
            reachable_nodes.iter().for_each(|(_, n)| {
                n.strong_links().iter().for_each(|peer_id| {
                    if let Some(node) = self.dag[round as usize - 1].get(&peer_id) {
                        new_reachable_nodes.insert(node.digest(), node);
                    }
                })
            });
            reachable_nodes = new_reachable_nodes;
            round -= 1;
        }

        reachable_nodes.keys().contains(&target.digest())
    }

    fn order_anchors(&mut self, anchor: Node) {
        let mut anchor_stack = Vec::new();
        let mut round = anchor.round();
        assert_eq!(round % 2, 0);
        let mut wave = anchor.round() / 2;
        let new_ordered_wave = wave;

        wave -= 1;
        round -= 2;
        let mut current_anchor = anchor;

        while wave >= self.lowest_unordered_anchor_wave {
            let prev_anchor_peer_id = self.proposer_election.get_round_anchor_peer_id(wave);

            let is_path =
                if let Some(prev_anchor) = self.dag[round as usize].get(&prev_anchor_peer_id) {
                    self.strong_path(&current_anchor, prev_anchor)
                } else {
                    false
                };

            if is_path {
                anchor_stack.push(std::mem::replace(
                    &mut current_anchor,
                    self.dag[round as usize]
                        .remove(&prev_anchor_peer_id)
                        .unwrap(),
                ));
            }

            wave -= 1;
            round -= 2;
        }

        anchor_stack.push(current_anchor);
        self.lowest_unordered_anchor_wave = new_ordered_wave + 1;
        self.order_history(anchor_stack);
    }

    fn order_history(&mut self, mut anchor_stack: Vec<Node>) {
        let mut ordered_history = Vec::new();

        while let Some(anchor) = anchor_stack.pop() {
            ordered_history.extend(self.order_anchor_causal_history(anchor));
        }

        // TODO: push to execution
    }

    fn order_anchor_causal_history(&mut self, anchor: Node) -> Vec<Node> {
        let mut ordered_history = Vec::new();

        let mut reachable_nodes = HashMap::new();
        reachable_nodes.insert(anchor.digest(), anchor);

        while !reachable_nodes.is_empty() {
            let mut new_reachable_nodes = HashMap::new();
            reachable_nodes
                .into_iter()
                .for_each(|(_, node)| {
                    node.parents()
                        .iter()
                        .for_each(|metadata| {
                            if let Some(parent) = self.dag[metadata.round() as usize].remove(&metadata.source()) {
                                new_reachable_nodes.insert(parent.digest(), parent);
                            }
                        });
                    ordered_history.push(node);
                });
            reachable_nodes = new_reachable_nodes;
        }
        ordered_history
    }

    pub fn try_ordering(&mut self, node: Node) {
        let round = node.round();
        let wave = round / 2;
        let author = node.source();

        assert!(!self
            .dag
            .get(round as usize)
            .map_or(false, |m| m.contains_key(&author)));

        if self.dag.len() < round as usize {
            assert_some!(self.dag.get(round as usize - 1));
            self.dag.push(HashMap::new());
        }

        self.dag[round as usize].insert(author, node);

        if round % 2 == 0 || wave < self.lowest_unordered_anchor_wave {
            return;
        }

        // We have one more potential vote in a wave we have not previously ordered
        let anchor_author = self.proposer_election.get_round_anchor_peer_id(wave);
        let voters = self
            .dag
            .get(round as usize)
            .unwrap()
            .iter()
            .filter(|(_, node)| node.strong_links().contains(&anchor_author))
            .map(|(peer_id, _)| peer_id);

        if self.verifier.check_minority_voting_power(voters).is_ok() {
            let anchor = self.dag[round as usize - 1].remove(&anchor_author).unwrap();
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
