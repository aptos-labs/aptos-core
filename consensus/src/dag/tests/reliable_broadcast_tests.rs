// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        reliable_broadcast::{BroadcastStatus, DAGNetworkSender, ReliableBroadcast},
        types::DAGMessage,
    },
    network_interface::ConsensusMsg,
};
use anyhow::bail;
use aptos_consensus_types::common::Author;
use aptos_infallible::Mutex;
use aptos_types::validator_verifier::random_validator_verifier;
use async_trait::async_trait;
use futures::{
    future::{AbortHandle, Abortable},
    FutureExt,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    sync::Arc,
    time::Duration,
};
use tokio::sync::oneshot;

#[derive(Serialize, Deserialize, Clone)]
struct TestMessage(Vec<u8>);

impl DAGMessage for TestMessage {
    fn epoch(&self) -> u64 {
        1
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct TestAck;

impl DAGMessage for TestAck {
    fn epoch(&self) -> u64 {
        1
    }
}

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
        self.received
            .lock()
            .insert(receiver, TestMessage::from_network_message(message)?);
        Ok(TestAck.into_network_message())
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
