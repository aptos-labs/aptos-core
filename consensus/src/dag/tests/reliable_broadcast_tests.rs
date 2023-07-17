// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        dag_network::DAGNetworkSender,
        dag_store::Dag,
        reliable_broadcast::{
            BroadcastStatus, CertifiedNodeHandleError, CertifiedNodeHandler,
            NodeBroadcastHandleError, NodeBroadcastHandler, ReliableBroadcast,
        },
        storage::DAGStorage,
        tests::{
            dag_test::MockStorage,
            helpers::{new_certified_node, new_node},
        },
        types::{CertifiedAck, DAGMessage, NodeCertificate, TestAck, TestMessage},
        NodeId, RpcHandler, Vote,
    },
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
};
use anyhow::bail;
use aptos_consensus_types::common::Author;
use aptos_infallible::{Mutex, RwLock};
use aptos_types::{
    aggregate_signature::PartialSignatures, epoch_state::EpochState,
    validator_verifier::random_validator_verifier,
};
use async_trait::async_trait;
use claims::{assert_ok, assert_ok_eq};
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::oneshot;

struct TestBroadcastStatus {
    threshold: usize,
    received: HashSet<Author>,
}

impl BroadcastStatus for TestBroadcastStatus {
    type Ack = TestAck;
    type Aggregated = HashSet<Author>;
    type Message = TestMessage;

    fn add(&mut self, peer: Author, _ack: Self::Ack) -> anyhow::Result<Option<Self::Aggregated>> {
        self.received.insert(peer);
        if self.received.len() == self.threshold {
            Ok(Some(self.received.clone()))
        } else {
            Ok(None)
        }
    }
}

struct TestDAGSender {
    failures: Mutex<HashMap<Author, u8>>,
    received: Mutex<HashMap<Author, TestMessage>>,
}

impl TestDAGSender {
    fn new(failures: HashMap<Author, u8>) -> Self {
        Self {
            failures: Mutex::new(failures),
            received: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl DAGNetworkSender for TestDAGSender {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: ConsensusMsg,
        _timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg> {
        match self.failures.lock().entry(receiver) {
            Entry::Occupied(mut entry) => {
                let count = entry.get_mut();
                *count -= 1;
                if *count == 0 {
                    entry.remove();
                }
                bail!("simulated failure");
            },
            Entry::Vacant(_) => (),
        };
        let message: TestMessage = (TConsensusMsg::from_network_message(message)
            as anyhow::Result<DAGMessage>)?
            .try_into()?;
        self.received.lock().insert(receiver, message.clone());
        Ok(DAGMessage::from(TestAck(message.0)).into_network_message())
    }

    async fn send_rpc_with_fallbacks(
        &self,
        _responders: Vec<Author>,
        _message: ConsensusMsg,
        _timeout: Duration,
    ) -> anyhow::Result<ConsensusMsg> {
        unimplemented!();
    }
}

#[tokio::test]
async fn test_reliable_broadcast() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let failures = HashMap::from([(validators[0], 1), (validators[2], 3)]);
    let sender = Arc::new(TestDAGSender::new(failures));
    let rb = ReliableBroadcast::new(validators.clone(), sender);
    let message = TestMessage(vec![42; validators.len() - 1]);
    let aggregating = TestBroadcastStatus {
        threshold: validators.len(),
        received: HashSet::new(),
    };
    let fut = rb.broadcast::<TestBroadcastStatus>(message, aggregating);
    assert_eq!(fut.await, validators.into_iter().collect());
}

#[tokio::test]
async fn test_chaining_reliable_broadcast() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let failures = HashMap::from([(validators[0], 1), (validators[2], 3)]);
    let sender = Arc::new(TestDAGSender::new(failures));
    let rb = ReliableBroadcast::new(validators.clone(), sender);
    let message = TestMessage(vec![42; validators.len()]);
    let expected = validators.iter().cloned().collect();
    let aggregating = TestBroadcastStatus {
        threshold: validators.len(),
        received: HashSet::new(),
    };
    let fut = rb
        .broadcast::<TestBroadcastStatus>(message.clone(), aggregating)
        .then(|aggregated| async move {
            assert_eq!(aggregated, expected);
            let aggregating = TestBroadcastStatus {
                threshold: validator_verifier.len(),
                received: HashSet::new(),
            };
            rb.broadcast::<TestBroadcastStatus>(message, aggregating)
                .await
        });
    assert_eq!(fut.await, validators.into_iter().collect());
}

#[tokio::test]
async fn test_abort_reliable_broadcast() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let failures = HashMap::from([(validators[0], 1), (validators[2], 3)]);
    let sender = Arc::new(TestDAGSender::new(failures));
    let rb = ReliableBroadcast::new(validators.clone(), sender);
    let message = TestMessage(vec![42; validators.len()]);
    let (tx, rx) = oneshot::channel();
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let aggregating = TestBroadcastStatus {
        threshold: validators.len(),
        received: HashSet::new(),
    };
    let fut = Abortable::new(
        rb.broadcast::<TestBroadcastStatus>(message.clone(), aggregating)
            .then(|_| async move {
                let aggregating = TestBroadcastStatus {
                    threshold: validators.len(),
                    received: HashSet::new(),
                };
                let ret = rb
                    .broadcast::<TestBroadcastStatus>(message, aggregating)
                    .await;
                tx.send(ret)
            }),
        abort_registration,
    );
    tokio::spawn(fut);
    abort_handle.abort();
    assert!(rx.await.is_err());
}

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

    assert_ok_eq!(
        storage.get_votes(),
        HashMap::from([(NodeId::new(0, 1, signers[0].author()), sig)])
    );

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
