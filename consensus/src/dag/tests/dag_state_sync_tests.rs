// Copyright © Aptos Foundation

use crate::{
    dag::{
        adapter::OrderedNotifier,
        dag_fetcher::{FetchRequestHandler, TDagFetcher},
        dag_state_sync::{DagStateSynchronizer, DAG_WINDOW},
        dag_store::Dag,
        storage::DAGStorage,
        tests::{dag_test::MockStorage, helpers::generate_dag_nodes},
        types::{CertifiedNodeMessage, RemoteFetchRequest},
        CertifiedNode, DAGMessage, RpcHandler, RpcWithFallback, TDAGNetworkSender,
    },
    test_utils::EmptyStateComputer,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_reliable_broadcast::RBNetworkSender;
use aptos_time_service::TimeService;
use aptos_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::random_validator_verifier,
};
use async_trait::async_trait;
use claims::assert_none;
use std::{sync::Arc, time::Duration};

struct MockDAGNetworkSender {}

#[async_trait]
impl RBNetworkSender<DAGMessage> for MockDAGNetworkSender {
    async fn send_rb_rpc(
        &self,
        _receiver: Author,
        _message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGMessage> {
        unimplemented!()
    }
}

#[async_trait]
impl TDAGNetworkSender for MockDAGNetworkSender {
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

struct MockDagFetcher {
    target_dag: Arc<RwLock<Dag>>,
    epoch_state: Arc<EpochState>,
}

#[async_trait]
impl TDagFetcher for MockDagFetcher {
    async fn fetch(
        &self,
        remote_request: RemoteFetchRequest,
        _responders: Vec<Author>,
        new_dag: Arc<RwLock<Dag>>,
    ) -> anyhow::Result<()> {
        let response = FetchRequestHandler::new(self.target_dag.clone(), self.epoch_state.clone())
            .process(remote_request)
            .await
            .unwrap();

        let mut new_dag_writer = new_dag.write();

        for node in response.certified_nodes().into_iter().rev() {
            new_dag_writer.add_node(node).unwrap()
        }

        Ok(())
    }
}

struct MockNotifier {}

#[async_trait]
impl OrderedNotifier for MockNotifier {
    fn send_ordered_nodes(
        &self,
        _ordered_nodes: Vec<Arc<CertifiedNode>>,
        _failed_author: Vec<(Round, Author)>,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

fn setup(epoch_state: Arc<EpochState>, storage: Arc<dyn DAGStorage>) -> DagStateSynchronizer {
    let time_service = TimeService::mock();
    let state_computer = Arc::new(EmptyStateComputer {});

    DagStateSynchronizer::new(epoch_state, time_service, state_computer, storage)
}

#[tokio::test]
async fn test_dag_state_sync() {
    const NUM_ROUNDS: usize = 90;
    const LI_ROUNDS: usize = NUM_ROUNDS * 2 / 3;
    const SLOW_DAG_ROUNDS: usize = NUM_ROUNDS / 3;

    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let storage = Arc::new(MockStorage::new());

    let virtual_dag = (0..NUM_ROUNDS)
        .map(|_| {
            signers
                .iter()
                .map(|_| Some(vec![true; signers.len() * 2 / 3 + 1]))
                .collect()
        })
        .collect::<Vec<_>>();
    let nodes = generate_dag_nodes(&virtual_dag, &validators);

    let mut fast_dag = Dag::new(epoch_state.clone(), Arc::new(MockStorage::new()), 1, 0);
    for round_nodes in &nodes {
        for node in round_nodes.iter().flatten() {
            fast_dag.add_node(node.clone()).unwrap();
        }
    }
    let fast_dag = Arc::new(RwLock::new(fast_dag));

    let mut slow_dag = Dag::new(epoch_state.clone(), Arc::new(MockStorage::new()), 1, 0);
    for round_nodes in nodes.iter().take(SLOW_DAG_ROUNDS) {
        for node in round_nodes.iter().flatten() {
            slow_dag.add_node(node.clone()).unwrap();
        }
    }
    let slow_dag = Arc::new(RwLock::new(slow_dag));

    let li_node = nodes[LI_ROUNDS - 1].first().unwrap().clone().unwrap();
    let sync_to_li = LedgerInfoWithSignatures::new(
        LedgerInfo::new(
            BlockInfo::new(
                epoch_state.epoch,
                li_node.round(),
                HashValue::zero(),
                HashValue::zero(),
                0,
                0,
                None,
            ),
            li_node.digest(),
        ),
        AggregateSignature::empty(),
    );
    let sync_to_node = nodes[NUM_ROUNDS - 1].first().unwrap().clone().unwrap();

    let sync_node_li = CertifiedNodeMessage::new(sync_to_node, sync_to_li);

    let state_sync = setup(epoch_state.clone(), storage.clone());
    let dag_fetcher = MockDagFetcher {
        target_dag: fast_dag.clone(),
        epoch_state: epoch_state.clone(),
    };

    let sync_result = state_sync
        .sync_dag_to(&sync_node_li, dag_fetcher, slow_dag.clone(), 0)
        .await;
    let new_dag = sync_result.unwrap().unwrap();

    assert_eq!(new_dag.lowest_round(), (LI_ROUNDS - DAG_WINDOW) as Round);
    assert_eq!(new_dag.highest_round(), (NUM_ROUNDS - 1) as Round);
    assert_none!(new_dag.highest_ordered_anchor_round(),);
}
