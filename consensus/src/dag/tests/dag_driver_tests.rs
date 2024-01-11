// Copyright © Aptos Foundation

use crate::{
    dag::{
        adapter::TLedgerInfoProvider,
        anchor_election::{RoundRobinAnchorElection, TChainHealthBackoff},
        dag_driver::DagDriver,
        dag_fetcher::TFetchRequester,
        dag_network::{RpcWithFallback, TDAGNetworkSender},
        dag_store::Dag,
        errors::DagDriverError,
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
use aptos_bounded_executor::BoundedExecutor;
use aptos_config::config::DagPayloadConfig;
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    validator_signer::ValidatorSigner,
    validator_verifier::{random_validator_verifier, ValidatorVerifier},
};
use async_trait::async_trait;
use claims::{assert_ok, assert_ok_eq};
use futures_channel::mpsc::unbounded;
use std::{sync::Arc, time::Duration};
use tokio::{runtime::Handle, sync::oneshot};
use tokio_retry::strategy::ExponentialBackoff;

struct MockNetworkSender {
    _drop_notifier: Option<oneshot::Sender<()>>,
}

#[async_trait]
impl RBNetworkSender<DAGMessage, DAGRpcResult> for MockNetworkSender {
    async fn send_rb_rpc(
        &self,
        _receiver: Author,
        _messagee: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
        Ok(DAGRpcResult(Ok(DAGMessage::TestAck(TestAck(Vec::new())))))
    }
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

struct MockChainHealthBackoff {}

impl TChainHealthBackoff for MockChainHealthBackoff {
    fn get_round_backoff(&self, _round: Round) -> (f64, Option<Duration>) {
        (1.0, None)
    }

    fn get_round_payload_limits(&self, _round: Round) -> (f64, Option<(u64, u64)>) {
        (1.0, None)
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
        verifier: validator_verifier,
    });

    let mock_ledger_info = LedgerInfo::mock_genesis(None);
    let mock_ledger_info = generate_ledger_info_with_sig(signers, mock_ledger_info);
    let storage = Arc::new(MockStorage::new_with_ledger_info(mock_ledger_info.clone()));
    let dag = Arc::new(RwLock::new(Dag::new(
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        0,
        TEST_DAG_WINDOW,
    )));

    let rb = Arc::new(ReliableBroadcast::new(
        signers.iter().map(|s| s.author()).collect(),
        network_sender.clone(),
        ExponentialBackoff::from_millis(10),
        aptos_time_service::TimeService::mock(),
        Duration::from_millis(500),
        BoundedExecutor::new(2, Handle::current()),
    ));
    let time_service = TimeService::mock();
    let validators = signers.iter().map(|vs| vs.author()).collect();
    let (tx, _) = unbounded();
    let order_rule = OrderRule::new(
        epoch_state.clone(),
        1,
        dag.clone(),
        Arc::new(RoundRobinAnchorElection::new(validators)),
        Arc::new(TestNotifier { tx }),
        storage.clone(),
        TEST_DAG_WINDOW as Round,
    );

    let fetch_requester = Arc::new(MockFetchRequester {});

    let ledger_info_provider = Arc::new(MockLedgerInfoProvider {
        latest_ledger_info: mock_ledger_info,
    });
    let (round_tx, _round_rx) = tokio::sync::mpsc::channel(10);
    let round_state = RoundState::new(
        round_tx.clone(),
        Box::new(OptimisticResponsive::new(round_tx)),
    );

    DagDriver::new(
        signers[0].author(),
        epoch_state,
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
        Arc::new(MockChainHealthBackoff {}),
        false,
    )
}

#[tokio::test]
async fn test_certified_node_handler() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let network_sender = Arc::new(MockNetworkSender {
        _drop_notifier: None,
    });
    let mut driver = setup(&signers, validator_verifier, network_sender);

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
    aptos_logger::Logger::init_for_testing();

    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let (tx, rx) = oneshot::channel();
    let network_sender = Arc::new(MockNetworkSender {
        _drop_notifier: Some(tx),
    });
    let mut driver = setup(&signers, validator_verifier, network_sender);

    driver.enter_new_round(1).await;

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_secs(5)).await;
        drop(driver);
    });

    let _ = rx.await;
}
