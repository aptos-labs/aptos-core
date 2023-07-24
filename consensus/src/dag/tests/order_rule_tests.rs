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
use aptos_infallible::{Mutex, RwLock};
use aptos_types::{
    aggregate_signature::AggregateSignature, epoch_state::EpochState,
    validator_verifier::random_validator_verifier,
};
use futures_channel::mpsc::{unbounded, UnboundedReceiver};
use proptest::prelude::*;
use std::sync::Arc;

/// Generate a virtual dag that first layer represents round
/// second layer represents nodes, Some => node exist, None => not exist
/// third layer is a bitmask that represents compressed strong links (true => linked, false => not linked),
/// the bitmask ignores non-existing nodes
fn generate_virtual_dag(
    num_nodes: usize,
    num_holes: usize,
    round: u64,
) -> impl Strategy<Value = Vec<Vec<Option<Vec<bool>>>>> {
    let num_strong_links = num_nodes * 2 / 3 + 1;
    assert!(num_holes <= num_nodes - num_strong_links);
    // This only has length of num_nodes - num_holes which ignores holes
    let strong_links: Vec<bool> = std::iter::repeat(false)
        .take(num_nodes - num_holes - num_strong_links)
        .chain(std::iter::repeat(true).take(num_strong_links))
        .collect();
    // This has length of num_nodes
    let nodes: Vec<bool> = std::iter::repeat(false)
        .take(num_holes)
        .chain(std::iter::repeat(true).take(num_nodes - num_holes))
        .collect();
    // For every round, we shuffle the nodes bitmask to generate holes
    // For every node, we shuffle the compressed strong links if the node is not a hole
    proptest::collection::vec(
        Just(nodes).prop_shuffle().prop_flat_map(move |nodes| {
            nodes
                .into_iter()
                .map(|exist| {
                    if exist {
                        Just(strong_links.clone())
                            .prop_shuffle()
                            .prop_map(Some)
                            .boxed()
                    } else {
                        Just(None).boxed()
                    }
                })
                .collect::<Vec<_>>()
        }),
        round as usize,
    )
}

/// Generate `num_perm` random permutations of how nodes are processed by the order rule
/// Imagine we have 4 nodes, this generates `num_perm` permutations of [0, 1, 2, 3]
fn generate_permutations(
    num_perm: usize,
    total_number: usize,
) -> impl Strategy<Value = Vec<Vec<usize>>> {
    proptest::collection::vec(
        Just((0..total_number).collect::<Vec<_>>()).prop_shuffle(),
        num_perm,
    )
}

/// Generate certified nodes for dag given the virtual dag
fn generate_dag_nodes(
    dag: &[Vec<Option<Vec<bool>>>],
    validators: &[Author],
) -> Vec<Vec<Option<CertifiedNode>>> {
    let mut nodes = vec![];
    let mut previous_round: Vec<Option<CertifiedNode>> = vec![];
    for (round, round_nodes) in dag.iter().enumerate() {
        let mut nodes_at_round = vec![];
        for (idx, author) in validators.iter().enumerate() {
            if let Some(bitmask) = &round_nodes[idx] {
                // the bitmask is compressed (without the holes), we need to flatten the previous round nodes
                // to match the index
                let parents: Vec<_> = previous_round
                    .iter()
                    .flatten()
                    .enumerate()
                    .filter(|(idx, _)| *bitmask.get(*idx).unwrap_or(&false))
                    .map(|(_, node)| {
                        NodeCertificate::new(node.metadata().clone(), AggregateSignature::empty())
                    })
                    .collect();
                if round > 1 {
                    assert_eq!(parents.len(), NUM_VALIDATORS * 2 / 3 + 1);
                }
                nodes_at_round.push(Some(new_certified_node(
                    (round + 1) as u64,
                    *author,
                    parents,
                )));
            } else {
                nodes_at_round.push(None);
            }
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

const NUM_HOLES: usize = 1;
const NUM_VALIDATORS: usize = 4;
const NUM_ROUNDS: u64 = 50;
const NUM_PERMUTATION: usize = 100;

proptest! {
    #[test]
    fn test_order_rule(
        mut dag_with_holes in generate_virtual_dag(NUM_VALIDATORS, NUM_HOLES, NUM_ROUNDS),
        mut dag in generate_virtual_dag(NUM_VALIDATORS, 0, NUM_ROUNDS),
        sequences in generate_permutations(NUM_PERMUTATION, (NUM_VALIDATORS - NUM_HOLES) * NUM_ROUNDS as usize)
    ) {
        let (_, validator_verifier) = random_validator_verifier(NUM_VALIDATORS, None, false);
        let validators = validator_verifier.get_ordered_account_addresses();
        let author_indexes = validator_verifier.address_to_validator_index().clone();
        dag.append(&mut dag_with_holes);
        let nodes = generate_dag_nodes(&dag, &validators);
        let epoch_state = Arc::new(EpochState {
            epoch: 1,
            verifier: validator_verifier,
        });
        let mut dag = Dag::new(epoch_state.clone(), Arc::new(MockStorage::new()));
        for round_nodes in &nodes {
            for node in round_nodes.iter().flatten() {
                dag.add_node(node.clone()).unwrap();
            }
        }
        let flatten_nodes: Vec<_> = nodes.into_iter().flatten().flatten().collect();
        let all_ordered = Arc::new(Mutex::new(vec![]));
        rayon::scope(|s| {
            for seq in sequences {
                s.spawn(|_| {
                    let dag = Arc::new(RwLock::new(dag.clone()));
                    let (mut order_rule, mut receiver) = create_order_rule(epoch_state.clone(), dag);
                    for idx in seq {
                        order_rule.process_new_node(&flatten_nodes[idx]);
                    }
                    let mut ordered = vec![];
                    while let Ok(Some(mut ordered_nodes)) = receiver.try_next() {
                        ordered.append(&mut ordered_nodes);
                    }
                    all_ordered.lock().push(ordered);
                });
            }
        });
        let display = |node: &Arc<CertifiedNode>| {
            (node.metadata().round(), *author_indexes.get(node.metadata().author()).unwrap())
        };
        let longest: Vec<_> = all_ordered.lock().iter().max_by(|v1, v2| v1.len().cmp(&v2.len())).unwrap().iter().map(display).collect();
        for ordered in all_ordered.lock().iter() {
            let a: Vec<_> = ordered.iter().map(display).collect();
            assert_eq!(a, longest[..a.len()]);
        }
    }
}
