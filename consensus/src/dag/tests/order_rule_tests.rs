// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        anchor_election::RoundRobinAnchorElection,
        dag_store::Dag,
        order_rule::OrderRule,
        tests::{dag_test::MockStorage, helpers::new_certified_node},
        types::NodeCertificate,
        CertifiedNode,
    },
    test_utils::placeholder_ledger_info,
};
use aptos_consensus_types::common::Author;
use aptos_infallible::RwLock;
use aptos_types::{
    aggregate_signature::AggregateSignature, epoch_state::EpochState,
    validator_verifier::random_validator_verifier,
};
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use proptest::prelude::*;
use std::sync::Arc;

/// Generate a virtual dag that first layer represents round
/// second layer represents nodes
/// third layer is a bitmask that represents strong links (true => linked, false => not linked)
fn generate_virtual_dag(
    num_validators: usize,
    round: u64,
) -> impl Strategy<Value = Vec<Vec<Vec<bool>>>> {
    let f = (num_validators - 1) / 3;
    let bitmask: Vec<bool> = std::iter::repeat(false)
        .take(f)
        .chain(std::iter::repeat(true).take(num_validators - f))
        .collect();
    proptest::collection::vec(
        proptest::collection::vec(Just(bitmask).prop_shuffle(), num_validators),
        round as usize,
    )
}

/// Generate `num_per` random permutations of how nodes are processed by the order rule
/// Imagine we have 4 nodes, this generates a permutation of [0, 1, 2, 3]
fn generate_permutations(
    num_perm: usize,
    total_number: usize,
) -> impl Strategy<Value = Vec<Vec<usize>>> {
    proptest::collection::vec(
        Just((0..total_number).collect::<Vec<_>>()).prop_shuffle(),
        num_perm,
    )
}

fn generate_dag_nodes(dag: &[Vec<Vec<bool>>], validators: &[Author]) -> Vec<Vec<CertifiedNode>> {
    let mut nodes = vec![];
    let mut previous_round: Vec<CertifiedNode> = vec![];
    for (round, round_nodes) in dag.iter().enumerate() {
        let mut nodes_at_round = vec![];
        for (idx, author) in validators.iter().enumerate() {
            let bitmask = &round_nodes[idx];
            let parents: Vec<_> = previous_round
                .iter()
                .enumerate()
                .filter(|(idx, _)| bitmask[*idx])
                .map(|(_, node)| {
                    NodeCertificate::new(node.metadata().clone(), AggregateSignature::empty())
                })
                .collect();
            if round > 1 {
                assert_eq!(parents.len(), NUM_VALIDATORS * 2 / 3 + 1);
            }
            nodes_at_round.push(new_certified_node((round + 1) as u64, *author, parents));
        }
        previous_round = nodes_at_round.clone();
        nodes.push(nodes_at_round);
    }
    nodes
}

fn create_order_rule(
    epoch_state: Arc<EpochState>,
    dag: Arc<RwLock<Dag>>,
) -> (OrderRule, UnboundedReceiver<Vec<Arc<CertifiedNode>>>) {
    let ledger_info = placeholder_ledger_info();
    let anchor_election = Box::new(RoundRobinAnchorElection::new(
        epoch_state.verifier.get_ordered_account_addresses(),
    ));
    let (tx, rx) = unbounded();
    (
        OrderRule::new(epoch_state, ledger_info, dag, anchor_election, tx),
        rx,
    )
}

const NUM_VALIDATORS: usize = 4;
const NUM_ROUNDS: u64 = 50;
const NUM_PERMUTATION: usize = 100;

proptest! {
    #[test]
    fn test_order_rule(
        dag in generate_virtual_dag(NUM_VALIDATORS, NUM_ROUNDS),
        sequences in generate_permutations(NUM_PERMUTATION, NUM_VALIDATORS * NUM_ROUNDS as usize)
    ) {
        let (_, validator_verifier) = random_validator_verifier(NUM_VALIDATORS, None, false);
        let validators = validator_verifier.get_ordered_account_addresses();
        let author_indexes = validator_verifier.address_to_validator_index().clone();
        let nodes = generate_dag_nodes(&dag, &validators);
        let epoch_state = Arc::new(EpochState {
            epoch: 1,
            verifier: validator_verifier,
        });
        let mut dag = Dag::new(epoch_state.clone(), Arc::new(MockStorage::new()));
        for round_nodes in &nodes {
            for node in round_nodes {
                dag.add_node(node.clone()).unwrap();
            }
        }
        let flatten_nodes: Vec<_> = nodes.into_iter().flatten().collect();
        let mut all_ordered = vec![];
        for seq in sequences {
            let dag = Arc::new(RwLock::new(dag.clone()));
            let (mut order_rule, mut receiver) = create_order_rule(epoch_state.clone(), dag.clone());
            for idx in seq {
                order_rule.process_new_node(&flatten_nodes[idx]);
            }
            let mut ordered = vec![];
            while let Ok(Some(mut ordered_nodes)) = receiver.try_next() {
                ordered.append(&mut ordered_nodes);
            }
            all_ordered.push(ordered);
        }
        let display = |node: &Arc<CertifiedNode>| {
            (node.metadata().round(), *author_indexes.get(node.metadata().author()).unwrap())
        };
        let longest: Vec<_> = all_ordered.iter().max_by(|v1, v2| v1.len().cmp(&v2.len())).unwrap().iter().map(display).collect();
        for ordered in all_ordered {
            let a: Vec<_> = ordered.iter().map(display).collect();
            assert_eq!(a, longest[..a.len()]);
        }
    }
}
