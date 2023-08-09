// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_store::Dag,
    reliable_broadcast::{
        CertifiedNodeHandleError, CertifiedNodeHandler, NodeBroadcastHandleError,
        NodeBroadcastHandler,
    },
    storage::DAGStorage,
    tests::{
        dag_test::MockStorage,
        helpers::{new_certified_node, new_node},
    },
    types::{CertifiedAck, NodeCertificate},
    NodeId, RpcHandler, Vote,
};
use aptos_infallible::RwLock;
use aptos_types::{
    aggregate_signature::PartialSignatures, epoch_state::EpochState,
    validator_verifier::random_validator_verifier,
};
use claims::{assert_ok, assert_ok_eq};
use std::{collections::BTreeMap, sync::Arc};

#[tokio::test]
async fn test_node_broadcast_receiver_succeed() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let storage = Arc::new(MockStorage::new());
    let dag = Arc::new(RwLock::new(Dag::new(epoch_state.clone(), storage.clone())));

    let wellformed_node = new_node(0, 10, signers[0].author(), vec![]);
    let equivocating_node = new_node(0, 20, signers[0].author(), vec![]);

    assert_ne!(wellformed_node.digest(), equivocating_node.digest());

    let mut rb_receiver = NodeBroadcastHandler::new(dag, signers[3].clone(), epoch_state, storage);

    let expected_result = Vote::new(
        wellformed_node.metadata().clone(),
        wellformed_node.sign_vote(&signers[3]).unwrap(),
    );
    // expect an ack for a valid message
    assert_ok_eq!(rb_receiver.process(wellformed_node), expected_result);
    // expect the original ack for any future message from same author
    assert_ok_eq!(rb_receiver.process(equivocating_node), expected_result);
}

#[tokio::test]
async fn test_node_broadcast_receiver_failure() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.clone(),
    });

    let mut rb_receivers: Vec<_> = signers
        .iter()
        .map(|signer| {
            let storage = Arc::new(MockStorage::new());
            let dag = Arc::new(RwLock::new(Dag::new(epoch_state.clone(), storage.clone())));

            NodeBroadcastHandler::new(dag, signer.clone(), epoch_state.clone(), storage)
        })
        .collect();

    // Round 0
    let node = new_node(0, 10, signers[0].author(), vec![]);
    let vote = rb_receivers[1].process(node.clone()).unwrap();

    // Round 1 with invalid parent
    let partial_sigs = PartialSignatures::new(BTreeMap::from([(
        signers[1].author(),
        vote.signature().clone(),
    )]));
    let node_cert = NodeCertificate::new(
        node.metadata().clone(),
        validator_verifier
            .aggregate_signatures(&partial_sigs)
            .unwrap(),
    );
    let node = new_node(1, 20, signers[0].author(), vec![node_cert]);
    assert_eq!(
        rb_receivers[1].process(node).unwrap_err().to_string(),
        NodeBroadcastHandleError::InvalidParent.to_string(),
    );

    // Round 0 - add all nodes
    let node_certificates: Vec<_> = signers
        .iter()
        .map(|signer| {
            let node = new_node(0, 10, signer.author(), vec![]);
            let mut partial_sigs = PartialSignatures::empty();
            rb_receivers
                .iter_mut()
                .zip(&signers)
                .for_each(|(rb_receiver, signer)| {
                    let sig = rb_receiver.process(node.clone()).unwrap();
                    partial_sigs.add_signature(signer.author(), sig.signature().clone())
                });
            NodeCertificate::new(
                node.metadata().clone(),
                validator_verifier
                    .aggregate_signatures(&partial_sigs)
                    .unwrap(),
            )
        })
        .collect();

    // Add Round 1 node with proper certificates
    let node = new_node(1, 20, signers[0].author(), node_certificates);
    assert_eq!(
        rb_receivers[0].process(node).unwrap_err().to_string(),
        NodeBroadcastHandleError::MissingParents.to_string()
    );
}

#[test]
fn test_node_broadcast_receiver_storage() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let storage = Arc::new(MockStorage::new());
    let dag = Arc::new(RwLock::new(Dag::new(epoch_state.clone(), storage.clone())));

    let node = new_node(1, 10, signers[0].author(), vec![]);

    let mut rb_receiver = NodeBroadcastHandler::new(
        dag.clone(),
        signers[3].clone(),
        epoch_state.clone(),
        storage.clone(),
    );
    let sig = rb_receiver.process(node).expect("must succeed");

    assert_ok_eq!(storage.get_votes(), vec![(
        NodeId::new(0, 1, signers[0].author()),
        sig
    )],);

    let mut rb_receiver =
        NodeBroadcastHandler::new(dag, signers[3].clone(), epoch_state, storage.clone());
    assert_ok!(rb_receiver.gc_before_round(2));
    assert_eq!(storage.get_votes().unwrap().len(), 0);
}

#[test]
fn test_certified_node_receiver() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let storage = Arc::new(MockStorage::new());
    let dag = Arc::new(RwLock::new(Dag::new(epoch_state, storage)));

    let zeroth_round_node = new_certified_node(0, signers[0].author(), vec![]);

    let mut rb_receiver = CertifiedNodeHandler::new(dag);

    // expect an ack for a valid message
    assert_ok!(rb_receiver.process(zeroth_round_node.clone()));
    // expect an ack if the same message is sent again
    assert_ok_eq!(rb_receiver.process(zeroth_round_node), CertifiedAck::new(1));

    let parent_node = new_certified_node(0, signers[1].author(), vec![]);
    let invalid_node = new_certified_node(1, signers[0].author(), vec![parent_node.certificate()]);
    assert_eq!(
        rb_receiver.process(invalid_node).unwrap_err().to_string(),
        CertifiedNodeHandleError::MissingParents.to_string()
    );
}
