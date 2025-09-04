// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_network::{RpcWithFallback, TDAGNetworkSender},
    types::{DAGMessage, TestAck, TestMessage},
    DAGRpcResult,
};
use anyhow::{anyhow, bail};
use velor_consensus_types::common::Author;
use velor_infallible::Mutex;
use velor_reliable_broadcast::RBNetworkSender;
use velor_time_service::{TimeService, TimeServiceTrait};
use velor_types::validator_verifier::random_validator_verifier;
use async_trait::async_trait;
use bytes::Bytes;
use claims::{assert_err, assert_ok};
use futures::StreamExt;
use std::{collections::HashMap, sync::Arc, time::Duration};

#[derive(Clone)]
enum TestPeerState {
    Fast,
    Slow(Duration),
    FailSlow(Duration),
}

#[derive(Clone)]
struct MockDAGNetworkSender {
    time_service: TimeService,
    test_peer_state: Arc<Mutex<HashMap<Author, TestPeerState>>>,
}

#[async_trait]
impl RBNetworkSender<DAGMessage, DAGRpcResult> for MockDAGNetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        _receiver: Author,
        _message: Bytes,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        unimplemented!()
    }

    async fn send_rb_rpc(
        &self,
        _receiver: Author,
        _message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        unimplemented!()
    }

    fn to_bytes_by_protocol(
        &self,
        _peers: Vec<Author>,
        _message: DAGMessage,
    ) -> anyhow::Result<HashMap<Author, Bytes>> {
        unimplemented!()
    }

    fn sort_peers_by_latency(&self, _: &mut [Author]) {}
}

#[async_trait]
impl TDAGNetworkSender for MockDAGNetworkSender {
    async fn send_rpc(
        &self,
        receiver: Author,
        message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        let message: TestMessage = message.try_into()?;
        let state = {
            self.test_peer_state
                .lock()
                .get(&receiver)
                .ok_or_else(|| anyhow!("lookup failed"))?
                .clone()
        };
        match state {
            TestPeerState::Fast => Ok(Ok(TestAck(message.0).into()).into()),
            TestPeerState::Slow(duration) => {
                self.time_service.sleep(duration).await;
                Ok(Ok(TestAck(message.0).into()).into())
            },
            TestPeerState::FailSlow(duration) => {
                self.time_service.sleep(duration).await;
                bail!("failed to respond");
            },
        }
    }

    async fn send_rpc_with_fallbacks(
        self: Arc<Self>,
        responders: Vec<Author>,
        message: DAGMessage,
        retry_interval: Duration,
        rpc_timeout: Duration,
        min_concurrent_responders: u32,
        max_concurrent_responders: u32,
    ) -> RpcWithFallback {
        RpcWithFallback::new(
            responders,
            message,
            retry_interval,
            rpc_timeout,
            self.clone(),
            self.time_service.clone(),
            min_concurrent_responders,
            max_concurrent_responders,
        )
    }
}

#[tokio::test]
async fn test_send_rpc_with_fallback() {
    let (_, validator_verifier) = random_validator_verifier(5, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let time_service = TimeService::real();

    let sender = MockDAGNetworkSender {
        time_service: time_service.clone(),
        test_peer_state: Arc::new(Mutex::new(HashMap::from([
            (validators[0], TestPeerState::Fast),
            (
                validators[1],
                TestPeerState::FailSlow(Duration::from_secs(1)),
            ),
            (validators[2], TestPeerState::Slow(Duration::from_secs(5))),
            (
                validators[3],
                TestPeerState::FailSlow(Duration::from_secs(3)),
            ),
            (validators[4], TestPeerState::Slow(Duration::from_secs(2))),
        ]))),
    };

    let message = TestMessage(vec![42; validators.len() - 1]);
    let mut rpc = Arc::new(sender)
        .send_rpc_with_fallbacks(
            validators,
            message.into(),
            Duration::from_millis(100),
            Duration::from_secs(5),
            1,
            4,
        )
        .await;

    assert_ok!(rpc.next().await.unwrap().result.unwrap().0);
    assert_err!(rpc.next().await.unwrap().result);
    assert_ok!(rpc.next().await.unwrap().result.unwrap().0);
    assert_err!(rpc.next().await.unwrap().result);
    assert_ok!(rpc.next().await.unwrap().result.unwrap().0);
}
