// Copyright © Aptos Foundation

use super::helpers::TEST_DAG_WINDOW;
use crate::{
    consensusdb::ConsensusDB,
    dag::{
        adapter::OrderedNotifier,
        dag_fetcher::{FetchRequestHandler, TDagFetcher},
        dag_state_sync::DagStateSynchronizer,
        dag_store::DagStore,
        errors::DagFetchError,
        storage::DAGStorage,
        tests::{
            dag_test::MockStorage,
            helpers::{generate_dag_nodes, MockPayloadManager},
        },
        types::{CertifiedNodeMessage, RemoteFetchRequest},
        CertifiedNode, DAGMessage, DAGRpcResult, RpcHandler, RpcWithFallback, StorageAdapter,
        TDAGNetworkSender,
    },
    pipeline::execution_client::DummyExecutionClient,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_crypto::HashValue;
use aptos_reliable_broadcast::RBNetworkSender;
use aptos_storage_interface::mock::MockDbReaderWriter;
use aptos_temppath::TempPath;
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
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::time::Instant;

struct MockDAGNetworkSender {}

#[async_trait]
impl RBNetworkSender<DAGMessage, DAGRpcResult> for MockDAGNetworkSender {
    async fn send_rb_rpc(
        &self,
        _receiver: Author,
        _message: DAGMessage,
        _timeout: Duration,
    ) -> anyhow::Result<DAGRpcResult> {
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

struct MockDagFetcher {
    target_dag: Arc<DagStore>,
    epoch_state: Arc<EpochState>,
}

#[async_trait]
impl TDagFetcher for MockDagFetcher {
    async fn fetch(
        &self,
        remote_request: RemoteFetchRequest,
        _responders: Vec<Author>,
        new_dag: Arc<DagStore>,
    ) -> Result<(), DagFetchError> {
        let response = FetchRequestHandler::new(self.target_dag.clone(), self.epoch_state.clone())
            .process(remote_request)
            .await
            .unwrap();

        for node in response.certified_nodes().into_iter().rev() {
            new_dag.write().add_node_for_test(node).unwrap()
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
    ) {
    }
}

fn setup(epoch_state: Arc<EpochState>, storage: Arc<dyn DAGStorage>) -> DagStateSynchronizer {
    let time_service = TimeService::mock();
    let execution_client = Arc::new(DummyExecutionClient {});
    let payload_manager = Arc::new(MockPayloadManager {});

    DagStateSynchronizer::new(
        epoch_state,
        time_service,
        execution_client,
        storage,
        payload_manager,
        TEST_DAG_WINDOW as Round,
    )
}

#[tokio::test]
async fn test_dag_state_sync() {
    const NUM_ROUNDS: u64 = 90;
    const LI_ROUNDS: u64 = NUM_ROUNDS * 2 / 3;
    const SLOW_DAG_ROUNDS: u64 = NUM_ROUNDS / 3;

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

    let fast_dag = Arc::new(DagStore::new(
        epoch_state.clone(),
        Arc::new(MockStorage::new()),
        Arc::new(MockPayloadManager {}),
        1,
        0,
    ));
    for round_nodes in &nodes {
        for node in round_nodes.iter().flatten() {
            fast_dag.write().add_node_for_test(node.clone()).unwrap();
        }
    }

    let slow_dag = Arc::new(DagStore::new(
        epoch_state.clone(),
        Arc::new(MockStorage::new()),
        Arc::new(MockPayloadManager {}),
        1,
        0,
    ));
    for round_nodes in nodes.iter().take(SLOW_DAG_ROUNDS as usize) {
        for node in round_nodes.iter().flatten() {
            slow_dag.write().add_node_for_test(node.clone()).unwrap();
        }
    }

    let li_node = nodes[LI_ROUNDS as usize - 1]
        .first()
        .unwrap()
        .clone()
        .unwrap();
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
    let sync_to_node = nodes[NUM_ROUNDS as usize - 1]
        .first()
        .unwrap()
        .clone()
        .unwrap();

    let sync_node_li = CertifiedNodeMessage::new(sync_to_node, sync_to_li);

    let state_sync = setup(epoch_state.clone(), storage.clone());
    let dag_fetcher = MockDagFetcher {
        target_dag: fast_dag.clone(),
        epoch_state: epoch_state.clone(),
    };

    let (request, responders, sync_dag_store) =
        state_sync.build_request(&sync_node_li, slow_dag.clone(), 0);

    let sync_result = state_sync
        .sync_dag_to(
            dag_fetcher,
            request,
            responders,
            sync_dag_store,
            sync_node_li.ledger_info().clone(),
        )
        .await;
    let new_dag = sync_result.unwrap();

    assert_eq!(
        new_dag.read().lowest_round(),
        (LI_ROUNDS - TEST_DAG_WINDOW) as Round
    );
    assert_eq!(new_dag.read().highest_round(), NUM_ROUNDS as Round);
    assert_none!(new_dag.read().highest_ordered_anchor_round(),);
}

#[tokio::test]
async fn test_dag_big_db() {
    const NUM_ROUNDS: u64 = 30;

    let (signers, validator_verifier) = random_validator_verifier(150, None, false);
    let validators = validator_verifier.get_ordered_account_addresses();
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let mut db_root_path = TempPath::new();
    db_root_path.persist();
    let consensus_db = Arc::new(ConsensusDB::new(db_root_path.as_ref()));
    let aptos_db = Arc::new(MockDbReaderWriter {});
    let storage = Arc::new(StorageAdapter::new(
        1,
        HashMap::from([(1, validators.clone())]),
        consensus_db,
        aptos_db,
    ));

    let virtual_dag = (0..NUM_ROUNDS)
        .map(|_| {
            signers
                .iter()
                .map(|_| Some(vec![true; signers.len() * 2 / 3 + 1]))
                .collect()
        })
        .collect::<Vec<_>>();
    let nodes = generate_dag_nodes(&virtual_dag, &validators);

    let start = Instant::now();

    let dag = Arc::new(DagStore::new(
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        1,
        30,
    ));
    for round_nodes in &nodes {
        for node in round_nodes.iter().flatten() {
            dag.add_node(node.clone()).unwrap();
        }
    }
    println!("add elapsed {}", start.elapsed().as_secs_f64());

    println!("db_path {:?}", db_root_path);

    for _ in 0..10 {
        let start = Instant::now();
        let dag = Arc::new(DagStore::new(
            epoch_state.clone(),
            storage.clone(),
            Arc::new(MockPayloadManager {}),
            1,
            30,
        ));
        println!("elapsed {}", start.elapsed().as_secs_f64());
    }

    storage
        .save_pending_node(&nodes[0][0].as_deref().unwrap())
        .unwrap();
    let start = Instant::now();
    for _ in 0..100 {
        storage.get_pending_node().unwrap();
    }
    println!("elapsed {}", start.elapsed().as_secs_f64());
    let _ = tokio::time::sleep(Duration::from_secs(5));
}
