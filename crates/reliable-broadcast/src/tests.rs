// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{BroadcastStatus, RBMessage, RBNetworkSender, ReliableBroadcast};
use anyhow::bail;
use aptos_bounded_executor::BoundedExecutor;
use aptos_consensus_types::common::Author;
use aptos_enum_conversion_derive::EnumConversion;
use aptos_infallible::Mutex;
use aptos_time_service::TimeService;
use aptos_types::validator_verifier::random_validator_verifier;
use async_trait::async_trait;
use futures::{
    stream::{AbortHandle, Abortable},
    FutureExt,
};
use futures_channel::oneshot;
use std::{
    collections::{hash_map::Entry, HashMap, HashSet},
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};
use tokio::runtime::Handle;
use tokio_retry::strategy::FixedInterval;

#[derive(Clone)]
struct TestMessage(Vec<u8>);

#[derive(Clone)]
struct TestAck(Vec<u8>);

#[derive(Clone, EnumConversion)]
enum TestRBMessage {
    TestMessage(TestMessage),
    TestAck(TestAck),
}

impl RBMessage for TestRBMessage {}

struct TestBroadcastStatus {
    threshold: usize,
    received: Arc<Mutex<HashSet<Author>>>,
}

impl<M> BroadcastStatus<M> for Arc<TestBroadcastStatus>
where
    M: RBMessage,
    TestAck: TryFrom<M> + Into<M>,
    TestMessage: TryFrom<M> + Into<M>,
{
    type Aggregated = HashSet<Author>;
    type Message = TestMessage;
    type Response = TestAck;

    fn add(&self, peer: Author, _ack: Self::Response) -> anyhow::Result<Option<Self::Aggregated>> {
        self.received.lock().insert(peer);
        if self.received.lock().len() == self.threshold {
            Ok(Some(self.received.lock().clone()))
        } else {
            Ok(None)
        }
    }
}

struct TestRBSender<M> {
    failures: Mutex<HashMap<Author, u8>>,
    received: Mutex<HashMap<Author, TestMessage>>,
    _marker: PhantomData<M>,
}

impl<M> TestRBSender<M>
where
    M: Send + Sync,
{
    fn new(failures: HashMap<Author, u8>) -> Self {
        Self {
            failures: Mutex::new(failures),
            received: Mutex::new(HashMap::new()),
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<M> RBNetworkSender<M> for TestRBSender<M>
where
    M: RBMessage,
    TestAck: TryFrom<M> + Into<M>,
    TestMessage: TryFrom<M, Error = anyhow::Error> + Into<M>,
{
    async fn send_rb_rpc(
        &self,
        receiver: Author,
        message: M,
        _timeout: Duration,
    ) -> anyhow::Result<M> {
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
        let message: TestMessage = message.try_into()?;
        self.received.lock().insert(receiver, message.clone());
        Ok(TestAck(message.0).into())
    }
}

#[tokio::test]
async fn test_reliable_broadcast() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let failures = HashMap::from([(validators[0], 1), (validators[2], 3)]);
    let sender = Arc::new(TestRBSender::<TestRBMessage>::new(failures));
    let rb = ReliableBroadcast::new(
        validators.clone(),
        sender,
        FixedInterval::from_millis(10),
        TimeService::real(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    );
    let message = TestMessage(vec![42; validators.len() - 1]);
    let aggregating = Arc::new(TestBroadcastStatus {
        threshold: validators.len(),
        received: Arc::new(Mutex::new(HashSet::new())),
    });
    let fut = rb.broadcast(message, aggregating);
    assert_eq!(fut.await, validators.into_iter().collect());
}

#[tokio::test]
async fn test_chaining_reliable_broadcast() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let failures = HashMap::from([(validators[0], 1), (validators[2], 3)]);
    let sender = Arc::new(TestRBSender::<TestRBMessage>::new(failures));
    let rb = Arc::new(ReliableBroadcast::new(
        validators.clone(),
        sender,
        FixedInterval::from_millis(10),
        TimeService::real(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    ));
    let message = TestMessage(vec![42; validators.len()]);
    let expected = validators.iter().cloned().collect();
    let aggregating = Arc::new(TestBroadcastStatus {
        threshold: validators.len(),
        received: Arc::new(Mutex::new(HashSet::new())),
    });
    let rb1 = rb.clone();
    let fut = rb1
        .broadcast(message.clone(), aggregating)
        .then(|aggregated| async move {
            assert_eq!(aggregated, expected);
            let aggregating = Arc::new(TestBroadcastStatus {
                threshold: validator_verifier.len(),
                received: Arc::new(Mutex::new(HashSet::new())),
            });
            rb.broadcast(message, aggregating).await
        });
    assert_eq!(fut.await, validators.into_iter().collect());
}

#[tokio::test]
async fn test_abort_reliable_broadcast() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let failures = HashMap::from([(validators[0], 1), (validators[2], 3)]);
    let sender = Arc::new(TestRBSender::<TestRBMessage>::new(failures));
    let rb = Arc::new(ReliableBroadcast::new(
        validators.clone(),
        sender,
        FixedInterval::from_millis(10),
        TimeService::real(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    ));
    let message = TestMessage(vec![42; validators.len()]);
    let (tx, rx) = oneshot::channel();
    let (abort_handle, abort_registration) = AbortHandle::new_pair();
    let aggregating = Arc::new(TestBroadcastStatus {
        threshold: validators.len(),
        received: Arc::new(Mutex::new(HashSet::new())),
    });
    let fut = Abortable::new(
        rb.broadcast(message.clone(), aggregating)
            .then(|_| async move {
                let aggregating = Arc::new(TestBroadcastStatus {
                    threshold: validators.len(),
                    received: Arc::new(Mutex::new(HashSet::new())),
                });
                let ret = rb.broadcast(message, aggregating).await;
                tx.send(ret)
            }),
        abort_registration,
    );
    tokio::spawn(fut);
    abort_handle.abort();
    assert!(rx.await.is_err());
}
