// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_network::DAGNetworkSender,
    reliable_broadcast::{BroadcastStatus, NodeBroadcastHandler, ReliableBroadcast},
    types::{DAGMessage, Node, NodeDigestSignature, TestAck, TestMessage},
    RpcHandler,
};
use anyhow::bail;
use aptos_consensus_types::common::{Author, Payload, Round};
use aptos_infallible::Mutex;
use aptos_types::{
    validator_signer::ValidatorSigner, validator_verifier::random_validator_verifier,
};
use async_trait::async_trait;
use claims::assert_ok_eq;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
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
        message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGMessage> {
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
        let message = TestMessage::try_from(message)?;
        self.received.lock().insert(receiver, message.clone());
        Ok(TestAck(message.0).into())
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
async fn test_node_broadcast_receiver() {
    let signer = ValidatorSigner::from_int(10);
    let validators = vec![Author::random(); 5];

    let message1 = create_test_node(1, 10, validators[1]);
    let message2 = create_test_node(1, 20, validators[1]);

    assert_ne!(message1.digest(), message2.digest());

    let mut rb_receiver = NodeBroadcastHandler::new(signer.clone());

    let expected_result =
        NodeDigestSignature::new(0, message1.digest(), message1.sign(&signer).unwrap());
    // expect an ack for a valid message
    assert_ok_eq!(rb_receiver.process(message1), expected_result);
    // expect the original ack for any future message from same author
    assert_ok_eq!(rb_receiver.process(message2), expected_result);
}

fn create_test_node(round: Round, timestamp: u64, author: Author) -> Node {
    Node::new(0, round, author, timestamp, Payload::empty(false), vec![])
}
