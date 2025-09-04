// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::helpers::TEST_DAG_WINDOW;
use crate::{
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
        CertifiedNode, DAGMessage, DAGRpcResult, RpcHandler, RpcWithFallback, TDAGNetworkSender,
    },
    pipeline::execution_client::DummyExecutionClient,
};
use velor_consensus_types::common::{Author, Round};
use velor_crypto::HashValue;
use velor_reliable_broadcast::RBNetworkSender;
use velor_time_service::TimeService;
use velor_types::{
    aggregate_signature::AggregateSignature,
    block_info::BlockInfo,
    epoch_state::EpochState,
    ledger_info::{LedgerInfo, LedgerInfoWithSignatures},
    validator_verifier::random_validator_verifier,
};
use async_trait::async_trait;
use bytes::Bytes;
use claims::assert_none;
use std::{collections::HashMap, sync::Arc, time::Duration};

struct MockDAGNetworkSender {}

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
        verifier: validator_verifier.into(),
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
