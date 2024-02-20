// Copyright © Aptos Foundation

use super::helpers::new_node;
use crate::dag::{
    tests::helpers::new_certified_node_with_empty_payload,
    types::{
        CertifiedNode, DAGNetworkMessage, DagPayload, DagSnapshotBitmask, Extensions, Node,
        NodeCertificate, NodeMetadata, RemoteFetchRequest,
    },
    NodeMessage,
};
use aptos_consensus_types::{
    common::Payload,
    dag_payload::{DecoupledPayload, PayloadInfo},
};
use aptos_crypto::HashValue;
use aptos_types::{
    aggregate_signature::AggregateSignature, validator_verifier::random_validator_verifier,
};
use claims::assert_ok;
use std::vec;

#[test]
fn test_node_verify() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let sender = signers[0].author();

    let invalid_node = Node::new_for_test(
        NodeMetadata::new_for_test(0, 0, signers[0].author(), 0, HashValue::random()),
        Payload::empty(false).into(),
        vec![],
        Extensions::empty(),
    );
    assert_eq!(
        invalid_node
            .verify(sender, &validator_verifier)
            .unwrap_err()
            .to_string(),
        "invalid digest"
    );

    // Well-formed round 1 node
    let first_round_node = new_node(
        1,
        10,
        signers[0].author(),
        Payload::empty(false).into(),
        vec![],
    );
    assert_ok!(first_round_node.verify(sender, &validator_verifier));
    // Mismatch sender
    first_round_node
        .verify(signers[1].author(), &validator_verifier)
        .unwrap_err();

    // Round 2 node without parents
    let node = new_node(
        2,
        20,
        signers[0].author(),
        Payload::empty(false).into(),
        vec![],
    );
    assert_eq!(
        node.verify(sender, &validator_verifier)
            .unwrap_err()
            .to_string(),
        "not enough parents to satisfy voting power",
    );

    // Round 1 cert
    let parent_cert = NodeCertificate::new(
        first_round_node.metadata().clone(),
        AggregateSignature::empty(),
    );
    let node = new_node(
        3,
        20,
        signers[0].author(),
        Payload::empty(false).into(),
        vec![parent_cert],
    );
    assert_eq!(
        node.verify(sender, &validator_verifier)
            .unwrap_err()
            .to_string(),
        "invalid parent round"
    );
}

#[test]
fn test_node_message_verify() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let sender = signers[0].author();

    let inline_payload_node_msg = NodeMessage::new(
        new_node(
            1,
            10,
            signers[0].author(),
            Payload::empty(false).into(),
            vec![],
        ),
        None,
    );
    assert_ok!(inline_payload_node_msg.verify(sender, &validator_verifier));

    let inline_payload_with_decoupled_payload = NodeMessage::new(
        new_node(
            1,
            10,
            signers[0].author(),
            Payload::empty(false).into(),
            vec![],
        ),
        Some(DecoupledPayload::new(
            1,
            1,
            signers[0].author(),
            Payload::empty(false),
        )),
    );
    assert_eq!(
        inline_payload_with_decoupled_payload
            .verify(sender, &validator_verifier)
            .unwrap_err()
            .to_string(),
        "decoupled payload present in Inline DagPayload mode"
    );

    let decoupled_payload = DecoupledPayload::new(1, 1, signers[0].author(), Payload::empty(false));

    let invalid_decoupled_payload_node_msg = NodeMessage::new(
        new_node(
            1,
            10,
            signers[0].author(),
            DagPayload::Decoupled(decoupled_payload.info()),
            vec![],
        ),
        None,
    );
    assert_eq!(
        invalid_decoupled_payload_node_msg
            .verify(sender, &validator_verifier)
            .unwrap_err()
            .to_string(),
        "decoupled_payload is None in Decoupled DagPayload mode"
    );

    let valid_decoupled_payload_node_msg = NodeMessage::new(
        new_node(
            1,
            10,
            signers[0].author(),
            DagPayload::Decoupled(decoupled_payload.info()),
            vec![],
        ),
        Some(decoupled_payload),
    );
    assert_ok!(valid_decoupled_payload_node_msg.verify(sender, &validator_verifier));

    let decoupled_payload = DecoupledPayload::new(1, 1, signers[0].author(), Payload::empty(false));
    let invalid_decoupled_payload_node_msg = NodeMessage::new(
        new_node(
            1,
            10,
            signers[0].author(),
            DagPayload::Decoupled(PayloadInfo::new_for_test(
                1,
                1,
                signers[0].author(),
                HashValue::zero(),
                0,
                0,
            )),
            vec![],
        ),
        Some(decoupled_payload),
    );
    assert_eq!(
        invalid_decoupled_payload_node_msg
            .verify(sender, &validator_verifier)
            .unwrap_err()
            .to_string(),
        "dag payload digest and decoupled payload digest mismatch"
    );
}

#[test]
fn test_certified_node_verify() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);

    let invalid_node = Node::new_for_test(
        NodeMetadata::new_for_test(0, 0, signers[0].author(), 0, HashValue::random()),
        Payload::empty(false).into(),
        vec![],
        Extensions::empty(),
    );
    let invalid_certified_node = CertifiedNode::new(invalid_node, AggregateSignature::empty());
    assert_eq!(
        invalid_certified_node
            .verify(&validator_verifier)
            .unwrap_err()
            .to_string(),
        "invalid digest"
    );

    let certified_node = new_certified_node_with_empty_payload(0, signers[0].author(), vec![]);

    assert_eq!(
        certified_node
            .verify(&validator_verifier)
            .unwrap_err()
            .to_string(),
        "Invalid bitvec from the multi-signature"
    );
}

#[test]
fn test_remote_fetch_request() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);

    let parents: Vec<_> = (0..3)
        .map(|idx| {
            NodeMetadata::new_for_test(1, 3, signers[idx].author(), 100, HashValue::random())
        })
        .collect();

    let request = RemoteFetchRequest::new(
        1,
        parents.clone(),
        DagSnapshotBitmask::new(1, vec![vec![false; 5]]),
    );
    assert_eq!(
        request.verify(&validator_verifier).unwrap_err().to_string(),
        "invalid bitmask: each round length is not equal to validator count"
    );

    let request = RemoteFetchRequest::new(
        1,
        vec![parents[0].clone()],
        DagSnapshotBitmask::new(1, vec![vec![false; signers.len()]]),
    );
    assert!(request
        .verify(&validator_verifier)
        .unwrap_err()
        .to_string()
        .contains("Bitmask length doesn't match"));

    let request = RemoteFetchRequest::new(
        1,
        vec![parents[0].clone()],
        DagSnapshotBitmask::new(1, vec![vec![false; signers.len()]; 3]),
    );
    assert_ok!(request.verify(&validator_verifier));

    let request = RemoteFetchRequest::new(
        1,
        parents,
        DagSnapshotBitmask::new(1, vec![vec![false; signers.len()]; 3]),
    );
    assert_ok!(request.verify(&validator_verifier));
}

#[test]
fn test_dag_snapshot_bitmask() {
    let bitmask = DagSnapshotBitmask::new(1, vec![vec![false, false, false, true]]);

    assert!(!bitmask.has(1, 0));
    assert!(bitmask.has(1, 3));
    assert!(!bitmask.has(2, 0));
    assert_eq!(bitmask.first_round(), 1);

    let bitmask = DagSnapshotBitmask::new(1, vec![vec![false, true, true, true], vec![
        false, true, false, false,
    ]]);

    assert!(!bitmask.has(1, 0));
    assert!(bitmask.has(1, 3));
    assert!(!bitmask.has(2, 0));
    assert!(bitmask.has(2, 1));
    assert!(!bitmask.has(10, 10));
    assert_eq!(bitmask.first_round(), 1);
}

#[test]
fn test_dag_network_message() {
    let short_data = vec![10; 10];
    let long_data = vec![20; 30];

    let short_message = DAGNetworkMessage::new(1, short_data);

    assert_eq!(
        format!("{:?}", short_message),
        "DAGNetworkMessage { epoch: 1, data: \"0a0a0a0a0a0a0a0a0a0a\" }"
    );

    let long_message = DAGNetworkMessage::new(2, long_data);

    assert_eq!(
        format!("{:?}", long_message),
        "DAGNetworkMessage { epoch: 2, data: \"1414141414141414141414141414141414141414\" }"
    );
}
