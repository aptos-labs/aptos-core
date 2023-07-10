// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        dag_network::DAGNetworkSender,
        dag_store::Dag,
        reliable_broadcast::{
            BroadcastStatus, NodeBroadcastHandleError, NodeBroadcastHandler, ReliableBroadcast,
        },
        types::{DAGMessage, Node, NodeCertificate, NodeDigestSignature, TestAck, TestMessage},
        RpcHandler,
    },
    network::TConsensusMsg,
    network_interface::ConsensusMsg,
};
use anyhow::bail;
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_infallible::{Mutex, RwLock};
use aptos_types::{
    aggregate_signature::PartialSignatures, validator_verifier::random_validator_verifier,
};
use async_trait::async_trait;
use claims::assert_ok_eq;
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
    let author_to_index = validator_verifier.address_to_validator_index().clone();
    let dag = Arc::new(RwLock::new(Dag::new(author_to_index, 0)));

    let wellformed_node = new_node(0, 10, signers[0].author(), vec![]);
    let equivocating_node = new_node(0, 20, signers[0].author(), vec![]);

    assert_ne!(wellformed_node.digest(), equivocating_node.digest());

    let mut rb_receiver = NodeBroadcastHandler::new(dag, signers[3].clone(), validator_verifier);

    let expected_result = NodeDigestSignature::new(
        0,
        wellformed_node.digest(),
        wellformed_node.sign(&signers[3]).unwrap(),
    );
    // expect an ack for a valid message
    assert_ok_eq!(rb_receiver.process(wellformed_node), expected_result);
    // expect the original ack for any future message from same author
    assert_ok_eq!(rb_receiver.process(equivocating_node), expected_result);
}

#[tokio::test]
async fn test_node_broadcast_receiver_failure() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let author_to_index = validator_verifier.address_to_validator_index().clone();

    let mut rb_receivers: Vec<NodeBroadcastHandler> = signers
        .iter()
        .map(|signer| {
            let dag = Arc::new(RwLock::new(Dag::new(author_to_index.clone(), 0)));

            NodeBroadcastHandler::new(dag, signer.clone(), validator_verifier.clone())
        })
        .collect();

    // Round 0
    let node = new_node(0, 10, signers[0].author(), vec![]);
    let node_sig = rb_receivers[1].process(node.clone()).unwrap();

    // Round 1 without enough parents
    let partial_sigs = PartialSignatures::new(BTreeMap::from([(
        signers[1].author(),
        node_sig.signature().clone(),
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
        "not enough voting power"
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

fn new_node(round: Round, timestamp: u64, author: Author, parents: Vec<NodeCertificate>) -> Node {
    Node::new(0, round, author, timestamp, Payload::empty(false), parents)
}
