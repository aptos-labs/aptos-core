// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use super::helpers::MockPayloadManager;
use crate::dag::{
    dag_store::DagStore,
    storage::{CommitEvent, DAGStorage},
    tests::helpers::{new_certified_node, TEST_DAG_WINDOW},
    types::{CertifiedNode, DagSnapshotBitmask, Node},
    NodeId, Vote,
};
use velor_consensus_types::common::Author;
use velor_crypto::HashValue;
use velor_infallible::Mutex;
use velor_types::{
    epoch_state::EpochState, ledger_info::LedgerInfoWithSignatures,
    validator_signer::ValidatorSigner, validator_verifier::random_validator_verifier,
};
use std::{collections::HashMap, sync::Arc};

pub struct MockStorage {
    node_data: Mutex<Option<Node>>,
    vote_data: Mutex<HashMap<NodeId, Vote>>,
    certified_node_data: Mutex<HashMap<HashValue, CertifiedNode>>,
    latest_ledger_info: Option<LedgerInfoWithSignatures>,
    epoch_state: Option<Arc<EpochState>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            node_data: Mutex::new(None),
            vote_data: Mutex::new(HashMap::new()),
            certified_node_data: Mutex::new(HashMap::new()),
            latest_ledger_info: None,
            epoch_state: None,
        }
    }

    pub fn new_with_ledger_info(
        ledger_info: LedgerInfoWithSignatures,
        epoch_state: Arc<EpochState>,
    ) -> Self {
        Self {
            node_data: Mutex::new(None),
            vote_data: Mutex::new(HashMap::new()),
            certified_node_data: Mutex::new(HashMap::new()),
            latest_ledger_info: Some(ledger_info),
            epoch_state: Some(epoch_state),
        }
    }
}

impl DAGStorage for MockStorage {
    fn save_pending_node(&self, node: &Node) -> anyhow::Result<()> {
        self.node_data.lock().replace(node.clone());
        Ok(())
    }

    fn get_pending_node(&self) -> anyhow::Result<Option<Node>> {
        Ok(self.node_data.lock().clone())
    }

    fn delete_pending_node(&self) -> anyhow::Result<()> {
        self.node_data.lock().take();
        Ok(())
    }

    fn save_vote(&self, node_id: &NodeId, vote: &Vote) -> anyhow::Result<()> {
        self.vote_data.lock().insert(node_id.clone(), vote.clone());
        Ok(())
    }

    fn get_votes(&self) -> anyhow::Result<Vec<(NodeId, Vote)>> {
        Ok(self.vote_data.lock().clone().into_iter().collect())
    }

    fn delete_votes(&self, node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        for node_id in node_ids {
            self.vote_data.lock().remove(&node_id);
        }
        Ok(())
    }

    fn save_certified_node(&self, node: &CertifiedNode) -> anyhow::Result<()> {
        self.certified_node_data
            .lock()
            .insert(node.digest(), node.clone());
        Ok(())
    }

    fn get_certified_nodes(&self) -> anyhow::Result<Vec<(HashValue, CertifiedNode)>> {
        Ok(self
            .certified_node_data
            .lock()
            .clone()
            .into_iter()
            .collect())
    }

    fn delete_certified_nodes(&self, digests: Vec<HashValue>) -> anyhow::Result<()> {
        for digest in digests {
            self.certified_node_data.lock().remove(&digest);
        }
        Ok(())
    }

    fn get_latest_k_committed_events(&self, _k: u64) -> anyhow::Result<Vec<CommitEvent>> {
        Ok(vec![])
    }

    fn get_latest_ledger_info(&self) -> anyhow::Result<LedgerInfoWithSignatures> {
        self.latest_ledger_info
            .clone()
            .ok_or_else(|| anyhow::anyhow!("ledger info not set"))
    }

    fn get_epoch_to_proposers(&self) -> HashMap<u64, Vec<Author>> {
        self.epoch_state
            .as_ref()
            .map(|epoch_state| {
                [(
                    epoch_state.epoch,
                    epoch_state.verifier.get_ordered_account_addresses(),
                )]
                .into()
            })
            .unwrap_or_default()
    }
}

fn setup() -> (
    Vec<ValidatorSigner>,
    Arc<EpochState>,
    DagStore,
    Arc<MockStorage>,
) {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier.into(),
    });
    let storage = Arc::new(MockStorage::new());
    let payload_manager = Arc::new(MockPayloadManager {});
    let dag = DagStore::new(
        epoch_state.clone(),
        storage.clone(),
        payload_manager,
        1,
        TEST_DAG_WINDOW,
    );
    (signers, epoch_state, dag, storage)
}

#[test]
fn test_dag_insertion_succeed() {
    let (signers, epoch_state, dag, _) = setup();

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_certified_node(1, signer.author(), vec![]);
        assert!(dag.write().add_node_for_test(node).is_ok());
    }
    let parents = dag
        .read()
        .get_strong_links_for_round(1, &epoch_state.verifier)
        .unwrap();

    // Round 2 nodes 0, 1, 2 links to 0, 1, 2
    for signer in &signers[0..3] {
        let node = new_certified_node(2, signer.author(), parents.clone());
        assert!(dag.write().add_node_for_test(node).is_ok());
    }

    // Round 3 nodes 1, 2 links to 0, 1, 2
    let parents = dag
        .read()
        .get_strong_links_for_round(2, &epoch_state.verifier)
        .unwrap();

    for signer in &signers[1..3] {
        let node = new_certified_node(3, signer.author(), parents.clone());
        assert!(dag.write().add_node_for_test(node).is_ok());
    }

    // not enough strong links
    assert!(dag
        .read()
        .get_strong_links_for_round(3, &epoch_state.verifier)
        .is_none());
}

#[test]
fn test_dag_insertion_failure() {
    let (signers, epoch_state, dag, _) = setup();

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_certified_node(1, signer.author(), vec![]);
        assert!(dag.write().add_node_for_test(node.clone()).is_ok());
        // duplicate node
        assert!(dag.write().add_node_for_test(node).is_err());
    }

    let missing_node = new_certified_node(1, signers[3].author(), vec![]);
    let mut parents = dag
        .read()
        .get_strong_links_for_round(1, &epoch_state.verifier)
        .unwrap();
    parents.push(missing_node.certificate());

    let node = new_certified_node(2, signers[0].author(), parents.clone());
    // parents not exist
    assert!(dag.write().add_node_for_test(node).is_err());

    let node = new_certified_node(3, signers[0].author(), vec![]);
    // round too high
    assert!(dag.write().add_node_for_test(node).is_err());

    let node = new_certified_node(2, signers[0].author(), parents[0..3].to_vec());
    assert!(dag.write().add_node_for_test(node).is_ok());
    let node = new_certified_node(2, signers[0].author(), vec![]);
    // equivocation node
    assert!(dag.write().add_node_for_test(node).is_err());
}

#[test]
fn test_dag_recover_from_storage() {
    let (signers, epoch_state, dag, storage) = setup();

    let mut metadatas = vec![];

    for round in 1..10 {
        let parents = dag
            .read()
            .get_strong_links_for_round(round, &epoch_state.verifier)
            .unwrap_or_default();
        for signer in &signers[0..3] {
            let node = new_certified_node(round, signer.author(), parents.clone());
            metadatas.push(node.metadata().clone());
            assert!(dag.add_node(node).is_ok());
        }
    }
    let new_dag = DagStore::new(
        epoch_state.clone(),
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        0,
        TEST_DAG_WINDOW,
    );

    for metadata in &metadatas {
        assert!(new_dag.read().exists(metadata));
    }

    let new_epoch_state = Arc::new(EpochState {
        epoch: 2,
        verifier: epoch_state.verifier.clone(),
    });

    let _new_epoch_dag = DagStore::new(
        new_epoch_state,
        storage.clone(),
        Arc::new(MockPayloadManager {}),
        0,
        TEST_DAG_WINDOW,
    );
    assert!(storage.certified_node_data.lock().is_empty());
}

#[test]
fn test_dag_bitmask() {
    let (signers, epoch_state, dag, _) = setup();

    assert_eq!(
        dag.read().bitmask(TEST_DAG_WINDOW),
        DagSnapshotBitmask::new(1, vec![vec![false; 4]; TEST_DAG_WINDOW as usize])
    );

    for round in 1..5 {
        let parents = dag
            .read()
            .get_strong_links_for_round(round - 1, &epoch_state.verifier)
            .unwrap_or_default();
        if round > 1 {
            assert!(!parents.is_empty());
        }
        for signer in &signers[0..3] {
            let node = new_certified_node(round, signer.author(), parents.clone());
            assert!(dag.write().add_node_for_test(node).is_ok());
        }
    }
    let mut bitmask = vec![vec![true, true, true, false]; 2];
    bitmask.resize(TEST_DAG_WINDOW as usize + 1, vec![false; 4]);
    assert_eq!(dag.read().bitmask(8), DagSnapshotBitmask::new(3, bitmask));

    // Populate the fourth author for all rounds
    for round in 1..5 {
        let parents = dag
            .read()
            .get_strong_links_for_round(round - 1, &epoch_state.verifier)
            .unwrap_or_default();
        if round > 1 {
            assert!(!parents.is_empty());
        }
        let node = new_certified_node(round, signers[3].author(), parents.clone());
        assert!(dag.write().add_node_for_test(node).is_ok());
    }
    assert_eq!(
        dag.read().bitmask(10),
        DagSnapshotBitmask::new(5, vec![vec![false; 4]; 6])
    );
    assert_eq!(
        dag.read().bitmask(6),
        DagSnapshotBitmask::new(5, vec![vec![false; 4]; 2])
    );
}
