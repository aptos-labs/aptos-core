// Copyright © Aptos Foundation

use super::{dag_test::MockStorage, helpers::new_node};
use crate::dag::{
    dag_fetcher::FetchHandler,
    dag_store::Dag,
    tests::helpers::new_certified_node,
    types::{FetchResponse, RemoteFetchRequest},
    RpcHandler,
};
use aptos_infallible::RwLock;
use aptos_types::{epoch_state::EpochState, validator_verifier::random_validator_verifier};
use claims::assert_ok_eq;
use std::sync::Arc;

#[test]
fn test_dag_fetcher_receiver() {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let storage = Arc::new(MockStorage::new());
    let dag = Arc::new(RwLock::new(Dag::new(epoch_state, storage)));

    let mut fetcher = FetchHandler::new(dag.clone());

    let mut first_round_nodes = vec![];

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_certified_node(1, signer.author(), vec![]);
        assert!(dag.write().add_node(node.clone()).is_ok());
        first_round_nodes.push(node);
    }

    let target_node = new_node(2, 100, signers[0].author(), vec![]);

    let request =
        RemoteFetchRequest::new(target_node.metadata().clone(), 1, vec![vec![false; 4]], 4);
    assert_eq!(
        fetcher.process(request).unwrap_err().to_string(),
        "not enough nodes to satisfy request"
    );

    // Round 1 - node 3
    {
        let node = new_certified_node(1, signers[3].author(), vec![]);
        assert!(dag.write().add_node(node.clone()).is_ok());
        first_round_nodes.push(node);
    }

    let request =
        RemoteFetchRequest::new(target_node.metadata().clone(), 1, vec![vec![false; 4]], 4);
    assert_ok_eq!(
        fetcher.process(request),
        FetchResponse::new(0, first_round_nodes)
    );
}
