// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    adapter::OrderedNotifier,
    anchor_election::RoundRobinAnchorElection,
    dag_store::{DagStore, InMemDag},
    order_rule::OrderRule,
    tests::{
        dag_test::MockStorage,
        helpers::{generate_dag_nodes, MockPayloadManager, TEST_DAG_WINDOW},
    },
    types::NodeMetadata,
    CertifiedNode,
};
use velor_consensus_types::common::{Author, Round};
use velor_infallible::Mutex;
use velor_types::{epoch_state::EpochState, validator_verifier::random_validator_verifier};
use async_trait::async_trait;
use futures_channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
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

pub struct TestNotifier {
    pub tx: UnboundedSender<Vec<Arc<CertifiedNode>>>,
}

#[async_trait]
impl OrderedNotifier for TestNotifier {
    fn send_ordered_nodes(
        &self,
        ordered_nodes: Vec<Arc<CertifiedNode>>,
        _failed_authors: Vec<(Round, Author)>,
    ) {
        self.tx.unbounded_send(ordered_nodes).unwrap()
    }
}

fn create_order_rule(
    epoch_state: Arc<EpochState>,
    dag: Arc<DagStore>,
) -> (OrderRule, UnboundedReceiver<Vec<Arc<CertifiedNode>>>) {
    let anchor_election = Arc::new(RoundRobinAnchorElection::new(
        epoch_state.verifier.get_ordered_account_addresses(),
    ));
    let (tx, rx) = unbounded();
    (
        OrderRule::new(
            epoch_state,
            1,
            dag,
            anchor_election,
            Arc::new(TestNotifier { tx }),
            TEST_DAG_WINDOW as Round,
            None,
        ),
        rx,
    )
}

const NUM_HOLES: usize = 1;
const NUM_VALIDATORS: usize = 5;
const NUM_ROUNDS: u64 = 50;
const NUM_PERMUTATION: usize = 100;

proptest! {
    #[test]
    fn test_order_rule_safety(
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
            verifier: validator_verifier.into(),
        });
        let mut dag = InMemDag::new_empty(epoch_state.clone(), 0, TEST_DAG_WINDOW);
        for round_nodes in &nodes {
            for node in round_nodes.iter().flatten() {
                dag.add_node_for_test(node.clone()).unwrap();
            }
        }
        let flatten_nodes: Vec<_> = nodes.into_iter().flatten().flatten().collect();
        let all_ordered = Arc::new(Mutex::new(vec![]));
        rayon::scope(|s| {
            for seq in sequences {
                s.spawn(|_| {
                    let dag = Arc::new(DagStore::new_for_test(dag.clone(),Arc::new(MockStorage::new()), Arc::new(MockPayloadManager {})));
                    let (mut order_rule, mut receiver) = create_order_rule(epoch_state.clone(), dag);
                    for idx in seq {
                        order_rule.process_new_node(flatten_nodes[idx].metadata());
                    }
                    let mut ordered = vec![];
                    while let Ok(Some(mut ordered_nodes)) = receiver.try_next() {
                        ordered.append(&mut ordered_nodes);
                    }
                    all_ordered.lock().push(ordered);
                });
            }
        });
        // order produced by process_all
        let dag = Arc::new(DagStore::new_for_test(dag.clone(),Arc::new(MockStorage::new()), Arc::new(MockPayloadManager {})));
        let (mut order_rule, mut receiver) = create_order_rule(epoch_state.clone(), dag);
        order_rule.process_all();
        let mut ordered = vec![];
        while let Ok(Some(mut ordered_nodes)) = receiver.try_next() {
            ordered.append(&mut ordered_nodes);
        }
        let display = |node: &Arc<CertifiedNode>| {
            (node.metadata().round(), *author_indexes.get(node.metadata().author()).unwrap())
        };
        let longest: Vec<_> = ordered.iter().map(display).collect();

        for ordered in all_ordered.lock().iter() {
            let a: Vec<_> = ordered.iter().map(display).collect();
            assert_eq!(a, longest[..a.len()]);
        }
    }
}

#[test]
fn test_order_rule_basic() {
    let dag = vec![
        vec![Some(vec![]), Some(vec![]), Some(vec![]), Some(vec![])],
        vec![
            Some(vec![false, true, true, true]),
            Some(vec![true, true, true, false]),
            Some(vec![false, true, true, true]),
            None,
        ],
        vec![
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
        ],
        vec![
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            Some(vec![true, false, true, true]),
            None,
        ],
        vec![
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            None,
        ],
        vec![
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            Some(vec![true, true, true, false]),
            None,
        ],
    ];
    let (_, validator_verifier) = random_validator_verifier(4, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let author_indexes = validator_verifier.address_to_validator_index().clone();
    let nodes = generate_dag_nodes(&dag, &validators);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.into(),
    });
    let mut dag = InMemDag::new_empty(epoch_state.clone(), 0, TEST_DAG_WINDOW);
    for round_nodes in &nodes {
        for node in round_nodes.iter().flatten() {
            dag.add_node_for_test(node.clone()).unwrap();
        }
    }
    let display = |node: &NodeMetadata| (node.round(), *author_indexes.get(node.author()).unwrap());
    let dag = Arc::new(DagStore::new_for_test(
        dag.clone(),
        Arc::new(MockStorage::new()),
        Arc::new(MockPayloadManager {}),
    ));
    let (mut order_rule, mut receiver): (OrderRule, UnboundedReceiver<Vec<Arc<CertifiedNode>>>) =
        create_order_rule(epoch_state, dag);
    for node in nodes.iter().flatten().flatten() {
        order_rule.process_new_node(node.metadata());
    }
    let expected_order = [
        // anchor (1, 0) has 1 votes, anchor (3, 1) has 2 votes and a path to (1, 0)
        vec![(1, 0)],
        // anchor (2, 1) has 3 votes
        vec![(1, 2), (1, 1), (2, 1)],
        // anchor (3, 1) has 2 votes
        vec![(1, 3), (2, 2), (2, 0), (3, 1)],
        // anchor (4, 2) has 3 votes
        vec![(3, 3), (3, 2), (3, 0), (4, 2)],
        // anchor (5, 2) has 3 votes
        vec![(4, 1), (4, 0), (5, 2)],
    ];
    let mut batch = 0;
    while let Ok(Some(ordered_nodes)) = receiver.try_next() {
        assert_eq!(
            ordered_nodes
                .iter()
                .map(|node| display(node.metadata()))
                .collect::<Vec<_>>(),
            expected_order[batch]
        );
        batch += 1;
    }
}
