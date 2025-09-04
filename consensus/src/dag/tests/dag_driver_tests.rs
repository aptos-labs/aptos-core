// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dag::{
        adapter::TLedgerInfoProvider,
        anchor_election::RoundRobinAnchorElection,
        dag_driver::DagDriver,
        dag_fetcher::TFetchRequester,
        dag_network::{RpcWithFallback, TDAGNetworkSender},
        dag_store::DagStore,
        errors::DagDriverError,
        health::{HealthBackoff, NoChainHealth, NoPipelineBackpressure},
        order_rule::OrderRule,
        round_state::{OptimisticResponsive, RoundState},
        tests::{
            dag_test::MockStorage,
            helpers::{new_certified_node, MockPayloadManager, TEST_DAG_WINDOW},
            order_rule_tests::TestNotifier,
        },
        types::{CertifiedAck, DAGMessage, TestAck},
        DAGRpcResult, RpcHandler,
    },
    test_utils::MockPayloadManager as MockPayloadClient,
};
use velor_bounded_executor::BoundedExecutor;
use velor_config::config::DagPayloadConfig;
use velor_consensus_types::common::{Author, Round};
use velor_infallible::Mutex;
use velor_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use velor_time_service::TimeService;
use velor_types::{
    epoch_state::EpochState,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
};
use async_trait::async_trait;
use bytes::Bytes;
use claims::{assert_ok, assert_ok_eq};
use futures_channel::mpsc::unbounded;
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::{runtime::Handle, sync::oneshot};
use tokio_retry::strategy::ExponentialBackoff;

struct MockNetworkSender {
    _drop_notifier: Option<oneshot::Sender<()>>,
}

#[async_trait]
impl RBNetworkSender<DAGMessage, DAGRpcResult> for MockNetworkSender {
    async fn send_rb_rpc_raw(
        &self,
        _receiver: Author,
        _messages: Bytes,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        Ok(DAGRpcResult(Ok(DAGMessage::TestAck(TestAck(Vec::new())))))
    }

    async fn send_rb_rpc(
        &self,
        _receiver: Author,
        _message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        Ok(DAGRpcResult(Ok(DAGMessage::TestAck(TestAck(Vec::new())))))
    }

    fn to_bytes_by_protocol(
        &self,
        peers: Vec<Author>,
        _message: DAGMessage,
    ) -> anyhow::Result<HashMap<Author, Bytes>> {
        Ok(peers.into_iter().map(|peer| (peer, Bytes::new())).collect())
    }

    fn sort_peers_by_latency(&self, _: &mut [Author]) {}
}

#[async_trait]
impl TDAGNetworkSender for MockNetworkSender {
    async fn send_rpc(
        &self,
        _receiver: Author,
        _message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        unimplemented!()
    }

    /// Given a list of potential responders, sending rpc to get response from any of them and could
    /// fallback to more in case of failures.
    async fn send_rpc_with_fallbacks(
        self: Arc<Self>,
        _responders: Vec<Author>,
        _message: DAGMessage,
        _retry_interval: Duration,
        _rpc_timeout: Duration,
        _min_concurrent_responders: u32,
        _max_concurrent_responders: u32,
    ) -> RpcWithFallback {
        unimplemented!()
    }
}

struct MockLedgerInfoProvider {
    latest_ledger_info: LedgerInfoWithSignatures,
}

impl TLedgerInfoProvider for MockLedgerInfoProvider {
    fn get_latest_ledger_info(&self) -> LedgerInfoWithSignatures {
        self.latest_ledger_info.clone()
    }

    fn get_highest_committed_anchor_round(&self) -> Round {
        self.latest_ledger_info.ledger_info().round()
    }
}

struct MockFetchRequester {}

impl TFetchRequester for MockFetchRequester {
    fn request_for_node(&self, _node: crate::dag::Node) -> anyhow::Result<()> {
        unimplemented!()
    }

    fn request_for_certified_node(&self, _node: crate::dag::CertifiedNode) -> anyhow::Result<()> {
        Ok(())
    }
}

fn setup(
    signers: &[ValidatorSigner],
    validator_verifier: ValidatorVerifier,
    network_sender: Arc<MockNetworkSender>,
) -> DagDriver {
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.into(),
    });

    let mock_ledger_info = LedgerInfo::mock_genesis(None);
    let mock_ledger_info = generate_ledger_info_with_sig(signers, mock_ledger_info);
    let storage = Arc::new(MockStorage::new_with_ledger_info(
        mock_ledger_info.clone(),
        epoch_state.clone(),
    ));
    let dag = Arc::new(DagStore::new(
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        0,
        TEST_DAG_WINDOW,
    ));

    let validators: Vec<_> = signers.iter().map(|s| s.author()).collect();
    let rb = Arc::new(ReliableBroadcast::new(
        validators[0],
        validators,
        network_sender.clone(),
        ExponentialBackoff::from_millis(10),
        velor_time_service::TimeService::mock(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    ));
    let time_service = TimeService::mock();
    let validators = signers.iter().map(|vs| vs.author()).collect();
    let (tx, _) = unbounded();
    let order_rule = Arc::new(Mutex::new(OrderRule::new(
        epoch_state.clone(),
        1,
        dag.clone(),
        Arc::new(RoundRobinAnchorElection::new(validators)),
        Arc::new(TestNotifier { tx }),
        TEST_DAG_WINDOW as Round,
        None,
    )));

    let fetch_requester = Arc::new(MockFetchRequester {});

    let ledger_info_provider = Arc::new(MockLedgerInfoProvider {
        latest_ledger_info: mock_ledger_info,
    });
    let (round_tx, _round_rx) = tokio::sync::mpsc::unbounded_channel();
    let round_state = RoundState::new(
        round_tx.clone(),
        Box::new(OptimisticResponsive::new(round_tx)),
    );

    DagDriver::new(
        signers[0].author(),
        epoch_state.clone(),
        dag,
        Arc::new(MockPayloadClient::new(None)),
        rb,
        time_service,
        storage,
        order_rule,
        fetch_requester,
        ledger_info_provider,
        round_state,
        TEST_DAG_WINDOW as Round,
        DagPayloadConfig::default(),
        HealthBackoff::new(
            epoch_state,
            NoChainHealth::new(),
            NoPipelineBackpressure::new(),
        ),
        false,
        true,
    )
}

#[tokio::test]
async fn test_certified_node_handler() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let network_sender = Arc::new(MockNetworkSender {
        _drop_notifier: None,
    });
    let driver = setup(&signers, validator_verifier, network_sender);

    let first_round_node = new_certified_node(1, signers[0].author(), vec![]);
    // expect an ack for a valid message
    assert_ok!(driver.process(first_round_node.clone()).await);
    // expect an ack if the same message is sent again
    assert_ok_eq!(driver.process(first_round_node).await, CertifiedAck::new(1));

    let parent_node = new_certified_node(1, signers[1].author(), vec![]);
    let invalid_node = new_certified_node(2, signers[0].author(), vec![parent_node.certificate()]);
    assert_eq!(
        driver.process(invalid_node).await.unwrap_err().to_string(),
        DagDriverError::MissingParents.to_string()
    );
}

#[tokio::test]
async fn test_dag_driver_drop() {
    velor_logger::Logger::init_for_testing();

    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let (tx, rx) = oneshot::channel();
    let network_sender = Arc::new(MockNetworkSender {
        _drop_notifier: Some(tx),
    });
    let driver = setup(&signers, validator_verifier, network_sender);

    driver.enter_new_round(1).await;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        drop(driver);
    });

    let _ = rx.await;
}
