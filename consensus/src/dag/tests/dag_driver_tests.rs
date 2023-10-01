// Copyright © Aptos Foundation

use crate::{
    dag::{
        adapter::TLedgerInfoProvider,
        anchor_election::RoundRobinAnchorElection,
        dag_driver::{DagDriver, DagDriverError},
        dag_fetcher::DagFetcherService,
        dag_network::{RpcWithFallback, TDAGNetworkSender},
        dag_state_sync::DAG_WINDOW,
        dag_store::Dag,
        order_rule::OrderRule,
        tests::{
            dag_test::MockStorage, helpers::new_certified_node, order_rule_tests::TestNotifier,
        },
        types::{CertifiedAck, DAGMessage},
        RpcHandler,
    },
    payload_manager::PayloadManager,
    test_utils::MockPayloadManager,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_infallible::RwLock;
use aptos_reliable_broadcast::{RBNetworkSender, ReliableBroadcast};
use aptos_time_service::TimeService;
use aptos_types::{
    epoch_state::EpochState,
    ledger_info::{generate_ledger_info_with_sig, LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::random_validator_verifier,
};
use async_trait::async_trait;
use claims::{assert_ok, assert_ok_eq};
use futures_channel::mpsc::unbounded;
use std::{sync::Arc, time::Duration};
use tokio_retry::strategy::ExponentialBackoff;

struct MockNetworkSender {}

#[async_trait]
impl RBNetworkSender<DAGMessage> for MockNetworkSender {
    async fn send_rb_rpc(
        &self,
        _receiver: Author,
        _messagee: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGMessage> {
        unimplemented!()
    }
}

#[async_trait]
impl TDAGNetworkSender for MockNetworkSender {
    async fn send_rpc(
        &self,
        _receiver: Author,
        _message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGMessage> {
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

#[tokio::test]
async fn test_certified_node_handler() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });

    let mock_ledger_info = LedgerInfo::mock_genesis(None);
    let mock_ledger_info = generate_ledger_info_with_sig(&signers, mock_ledger_info);
    let storage = Arc::new(MockStorage::new_with_ledger_info(mock_ledger_info.clone()));
    let dag = Arc::new(RwLock::new(Dag::new(
        epoch_state.clone(),
        storage.clone(),
        0,
        DAG_WINDOW,
    )));

    let network_sender = Arc::new(MockNetworkSender {});
    let rb = Arc::new(ReliableBroadcast::new(
        signers.iter().map(|s| s.author()).collect(),
        network_sender.clone(),
        ExponentialBackoff::from_millis(10),
        aptos_time_service::TimeService::mock(),
        Duration::from_millis(500),
    ));
    let time_service = TimeService::mock();
    let validators = signers.iter().map(|vs| vs.author()).collect();
    let (tx, _) = unbounded();
    let order_rule = OrderRule::new(
        epoch_state.clone(),
        LedgerInfo::mock_genesis(None),
        dag.clone(),
        Box::new(RoundRobinAnchorElection::new(validators)),
        Arc::new(TestNotifier { tx }),
        storage.clone(),
    );

    let (_, fetch_requester, _, _) = DagFetcherService::new(
        epoch_state.clone(),
        network_sender,
        dag.clone(),
        aptos_time_service::TimeService::mock(),
    );
    let fetch_requester = Arc::new(fetch_requester);

    let ledger_info_provider = Arc::new(MockLedgerInfoProvider {
        latest_ledger_info: mock_ledger_info,
    });

    let mut driver = DagDriver::new(
        signers[0].author(),
        epoch_state,
        dag,
        Arc::new(PayloadManager::DirectMempool),
        Arc::new(MockPayloadManager::new(None)),
        rb,
        time_service,
        storage,
        order_rule,
        fetch_requester,
        ledger_info_provider,
    );

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
