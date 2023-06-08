// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::dag_store::{CertifiedNode, Dag, Node, NodeCertificate, NodeMetadata};
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_types::{
    aggregate_signature::AggregateSignature, validator_verifier::random_validator_verifier,
};

#[test]
fn test_dag_insertion_succeed() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let author_to_index = validator_verifier.address_to_validator_index().clone();
    let mut dag = Dag::new(author_to_index, 0);

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_node(1, signer.author(), vec![]);
        assert!(dag.add_node(node).is_ok());
    }
    let parents = dag
        .get_unlinked_nodes_for_new_round(&validator_verifier)
        .unwrap();

    // Round 2 nodes 0, 1, 2 links to 0, 1, 2
    for signer in &signers[0..3] {
        let node = new_node(2, signer.author(), parents.clone());
        assert!(dag.add_node(node).is_ok());
    }

    let slow_node = new_node(1, signers[3].author(), vec![]);
    assert!(dag.add_node(slow_node).is_ok());

    // Round 3 nodes 1, 2 links to 0, 1, 2, 3 (weak)
    let parents = dag
        .get_unlinked_nodes_for_new_round(&validator_verifier)
        .unwrap();
    assert_eq!(parents.len(), 4);

    dag.mark_nodes_linked(&parents);
    assert!(dag
        .get_unlinked_nodes_for_new_round(&validator_verifier)
        .is_none());

    for signer in &signers[1..3] {
        let node = new_node(3, signer.author(), parents.clone());
        assert!(dag.add_node(node).is_ok());
    }

    // not enough strong links
    assert!(dag
        .get_unlinked_nodes_for_new_round(&validator_verifier)
        .is_none());
}

#[test]
fn test_dag_insertion_failure() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let author_to_index = validator_verifier.address_to_validator_index().clone();
    let mut dag = Dag::new(author_to_index, 0);

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_node(1, signer.author(), vec![]);
        assert!(dag.add_node(node.clone()).is_ok());
        // duplicate node
        assert!(dag.add_node(node).is_err());
    }

    let missing_node = new_node(1, signers[3].author(), vec![]);
    let mut parents = dag
        .get_unlinked_nodes_for_new_round(&validator_verifier)
        .unwrap();
    parents.push(missing_node.metadata());

    let node = new_node(2, signers[0].author(), parents.clone());
    // parents not exist
    assert!(dag.add_node(node).is_err());

    let node = new_node(3, signers[0].author(), vec![]);
    // round too high
    assert!(dag.add_node(node).is_err());

    let node = new_node(2, signers[0].author(), parents[0..3].to_vec());
    assert!(dag.add_node(node).is_ok());
    let node = new_node(2, signers[0].author(), vec![]);
    assert!(dag.add_node(node).is_err());
}

fn new_node(round: Round, author: Author, parents: Vec<NodeMetadata>) -> CertifiedNode {
    let node = Node::new(1, round, author, 0, Payload::empty(false), parents);
    let digest = node.digest();
    CertifiedNode::new(
        node,
        NodeCertificate::new(digest, AggregateSignature::empty()),
    )
}
