// Copyright © Aptos Foundation

use super::helpers::new_node;
use crate::dag::{
    tests::helpers::new_certified_node,
    types::{CertifiedNode, Node, NodeCertificate, NodeMetadata, TDAGMessage},
};
use aptos_consensus_types::common::Payload;
use aptos_crypto::HashValue;
use aptos_types::{
    aggregate_signature::AggregateSignature, validator_verifier::random_validator_verifier,
};
use claims::assert_ok;
use std::vec;

#[test]
fn test_node_verify() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);

    let invalid_node = Node::new_for_test(
        NodeMetadata::new_for_test(0, 0, signers[0].author(), 0, HashValue::random()),
        Payload::empty(false),
        vec![],
    );
    assert_eq!(
        invalid_node
            .verify(&validator_verifier)
            .unwrap_err()
            .to_string(),
        "invalid digest"
    );

    // Well-formed round 0 node
    let zeroth_round_node = new_node(0, 10, signers[0].author(), vec![]);
    assert_ok!(zeroth_round_node.verify(&validator_verifier));

    // Round 1 node without parents
    let node = new_node(2, 20, signers[0].author(), vec![]);
    assert_eq!(
        node.verify(&validator_verifier).unwrap_err().to_string(),
        "not enough parents to satisfy voting power",
    );

    // Round 1
    let parent_cert = NodeCertificate::new(
        zeroth_round_node.metadata().clone(),
        AggregateSignature::empty(),
    );
    let node = new_node(3, 20, signers[0].author(), vec![parent_cert]);
    assert_eq!(
        node.verify(&validator_verifier).unwrap_err().to_string(),
        "invalid parent round"
    );
}

#[test]
fn test_certified_node_verify() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);

    let invalid_node = Node::new_for_test(
        NodeMetadata::new_for_test(0, 0, signers[0].author(), 0, HashValue::random()),
        Payload::empty(false),
        vec![],
    );
    let invalid_certified_node = CertifiedNode::new(invalid_node, AggregateSignature::empty());
    assert_eq!(
        invalid_certified_node
            .verify(&validator_verifier)
            .unwrap_err()
            .to_string(),
        "invalid digest"
    );

    let certified_node = new_certified_node(0, signers[0].author(), vec![]);

    assert_eq!(
        certified_node
            .verify(&validator_verifier)
            .unwrap_err()
            .to_string(),
        "unable to verify: Invalid bitvec from the multi-signature"
    );
}
