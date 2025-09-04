// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_fetcher::TFetchRequester,
    dag_store::DagStore,
    errors::NodeBroadcastHandleError,
    health::{HealthBackoff, NoChainHealth, NoPipelineBackpressure},
    rb_handler::NodeBroadcastHandler,
    storage::DAGStorage,
    tests::{
        dag_test::MockStorage,
        helpers::{new_node, MockOrderRule, MockPayloadManager, TEST_DAG_WINDOW},
    },
    types::NodeCertificate,
    NodeId, RpcHandler, Vote,
};
use velor_config::config::DagPayloadConfig;
use velor_types::{
    aggregate_signature::PartialSignatures,
    epoch_state::EpochState,
    on_chain_config::{OnChainJWKConsensusConfig, OnChainRandomnessConfig, ValidatorTxnConfig},
    validator_verifier::random_validator_verifier,
};
use claims::{assert_ok, assert_ok_eq};
use futures::executor::block_on;
use std::{collections::BTreeMap, sync::Arc};

struct MockFetchRequester {}

impl TFetchRequester for MockFetchRequester {
    fn request_for_node(&self, _node: crate::dag::Node) -> anyhow::Result<()> {
        Ok(())
    }

    fn request_for_certified_node(&self, _node: crate::dag::CertifiedNode) -> anyhow::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn test_node_broadcast_receiver_succeed() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.into(),
    });
    let signers: Vec<_> = signers.into_iter().map(Arc::new).collect();

    // Scenario: Start DAG from beginning
    let storage = Arc::new(MockStorage::new());
    let dag = Arc::new(DagStore::new(
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        0,
        TEST_DAG_WINDOW,
    ));
    let order_rule = Arc::new(MockOrderRule {});

    let health_backoff = HealthBackoff::new(
        epoch_state.clone(),
        NoChainHealth::new(),
        NoPipelineBackpressure::new(),
    );

    let wellformed_node = new_node(1, 10, signers[0].author(), vec![]);
    let equivocating_node = new_node(1, 20, signers[0].author(), vec![]);

    assert_ne!(wellformed_node.digest(), equivocating_node.digest());

    let rb_receiver = NodeBroadcastHandler::new(
        dag,
        order_rule,
        signers[3].clone(),
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockFetchRequester {}),
        DagPayloadConfig::default(),
        ValidatorTxnConfig::default_disabled(),
        OnChainRandomnessConfig::default_disabled(),
        OnChainJWKConsensusConfig::default_disabled(),
        health_backoff,
    );

    let expected_result = Vote::new(
        wellformed_node.metadata().clone(),
        wellformed_node.sign_vote(&signers[3]).unwrap(),
    );
    // expect an ack for a valid message
    assert_ok_eq!(rb_receiver.process(wellformed_node).await, expected_result);
    // expect the original ack for any future message from same author
    assert_ok_eq!(
        rb_receiver.process(equivocating_node).await,
        expected_result
    );
}

// TODO: Unit test node broad receiver with a pruned DAG store. Possibly need a validator verifier trait.

#[tokio::test]
async fn test_node_broadcast_receiver_failure() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let validator_verifier = Arc::new(validator_verifier);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.clone(),
    });
    let signers: Vec<_> = signers.into_iter().map(Arc::new).collect();

    let mut rb_receivers: Vec<_> = signers
        .iter()
        .map(|signer| {
            let storage = Arc::new(MockStorage::new());
            let dag = Arc::new(DagStore::new(
                epoch_state.clone(),
                storage.clone(),
                Arc::new(MockPayloadManager {}),
                0,
                TEST_DAG_WINDOW,
            ));
            let order_rule = Arc::new(MockOrderRule {});

            NodeBroadcastHandler::new(
                dag,
                order_rule,
                signer.clone(),
                epoch_state.clone(),
                storage,
                Arc::new(MockFetchRequester {}),
                DagPayloadConfig::default(),
                ValidatorTxnConfig::default_disabled(),
                OnChainRandomnessConfig::default_disabled(),
                OnChainJWKConsensusConfig::default_disabled(),
                HealthBackoff::new(
                    epoch_state.clone(),
                    NoChainHealth::new(),
                    NoPipelineBackpressure::new(),
                ),
            )
        })
        .collect();

    // Round 1
    let node = new_node(1, 10, signers[0].author(), vec![]);
    let vote = rb_receivers[1].process(node.clone()).await.unwrap();

    // Round 2 with invalid parent
    let partial_sigs = PartialSignatures::new(BTreeMap::from([(
        signers[1].author(),
        vote.signature().clone(),
    )]));
    let node_cert = NodeCertificate::new(
        node.metadata().clone(),
        validator_verifier
            .aggregate_signatures(partial_sigs.signatures_iter())
            .unwrap(),
    );
    let node = new_node(2, 20, signers[0].author(), vec![node_cert]);
    assert_eq!(
        rb_receivers[1].process(node).await.unwrap_err().to_string(),
        NodeBroadcastHandleError::InvalidParent.to_string(),
    );

    // Round 1 - add all nodes
    let node_certificates: Vec<_> = signers
        .iter()
        .map(|signer| {
            let node = new_node(1, 10, signer.author(), vec![]);
            let mut partial_sigs = PartialSignatures::empty();
            rb_receivers
                .iter_mut()
                .zip(&signers)
                .for_each(|(rb_receiver, signer)| {
                    let sig = block_on(rb_receiver.process(node.clone())).unwrap();
                    partial_sigs.add_signature(signer.author(), sig.signature().clone())
                });
            NodeCertificate::new(
                node.metadata().clone(),
                validator_verifier
                    .aggregate_signatures(partial_sigs.signatures_iter())
                    .unwrap(),
            )
        })
        .collect();

    // Add Round 2 node with proper certificates
    let node = new_node(2, 20, signers[0].author(), node_certificates);
    assert_eq!(
        rb_receivers[0].process(node).await.unwrap_err().to_string(),
        NodeBroadcastHandleError::MissingParents.to_string()
    );
}

#[tokio::test]
async fn test_node_broadcast_receiver_storage() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let signers: Vec<_> = signers.into_iter().map(Arc::new).collect();
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.into(),
    });

    let storage = Arc::new(MockStorage::new());
    let dag = Arc::new(DagStore::new(
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        0,
        TEST_DAG_WINDOW,
    ));
    let order_rule = Arc::new(MockOrderRule {});

    let node = new_node(1, 10, signers[0].author(), vec![]);

    let rb_receiver = NodeBroadcastHandler::new(
        dag.clone(),
        order_rule.clone(),
        signers[3].clone(),
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockFetchRequester {}),
        DagPayloadConfig::default(),
        ValidatorTxnConfig::default_disabled(),
        OnChainRandomnessConfig::default_disabled(),
        OnChainJWKConsensusConfig::default_disabled(),
        HealthBackoff::new(
            epoch_state.clone(),
            NoChainHealth::new(),
            NoPipelineBackpressure::new(),
        ),
    );
    let sig = rb_receiver.process(node).await.expect("must succeed");

    assert_ok_eq!(storage.get_votes(), vec![(
        NodeId::new(1, 1, signers[0].author()),
        sig
    )],);

    let rb_receiver = NodeBroadcastHandler::new(
        dag,
        order_rule.clone(),
        signers[3].clone(),
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockFetchRequester {}),
        DagPayloadConfig::default(),
        ValidatorTxnConfig::default_disabled(),
        OnChainRandomnessConfig::default_disabled(),
        OnChainJWKConsensusConfig::default_disabled(),
        HealthBackoff::new(
            epoch_state,
            NoChainHealth::new(),
            NoPipelineBackpressure::new(),
        ),
    );
    assert_ok!(rb_receiver.gc_before_round(2));
    assert_eq!(storage.get_votes().unwrap().len(), 0);
}
