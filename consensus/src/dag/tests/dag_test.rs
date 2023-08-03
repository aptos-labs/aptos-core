// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::dag::{
    dag_store::Dag,
    storage::DAGStorage,
    tests::helpers::new_certified_node,
    types::{CertifiedNode, DagSnapshotBitmask, Node},
    NodeId, Vote,
};
use aptos_crypto::HashValue;
use aptos_infallible::Mutex;
use aptos_types::{
    epoch_state::EpochState, validator_signer::ValidatorSigner,
    validator_verifier::random_validator_verifier,
};
use std::{collections::HashMap, sync::Arc};

pub struct MockStorage {
    node_data: Mutex<HashMap<HashValue, Node>>,
    vote_data: Mutex<HashMap<NodeId, Vote>>,
    certified_node_data: Mutex<HashMap<HashValue, CertifiedNode>>,
}

impl MockStorage {
    pub fn new() -> Self {
        Self {
            node_data: Mutex::new(HashMap::new()),
            vote_data: Mutex::new(HashMap::new()),
            certified_node_data: Mutex::new(HashMap::new()),
        }
    }
}

impl DAGStorage for MockStorage {
    fn save_node(&self, node: &Node) -> anyhow::Result<()> {
        self.node_data.lock().insert(node.digest(), node.clone());
        Ok(())
    }

    fn delete_node(&self, digest: HashValue) -> anyhow::Result<()> {
        self.node_data.lock().remove(&digest);
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

    fn save_ordered_anchor_id(&self, _node_id: &NodeId) -> anyhow::Result<()> {
        todo!()
    }

    fn get_ordered_anchor_ids(&self) -> anyhow::Result<Vec<(NodeId, ())>> {
        todo!()
    }

    fn delete_ordered_anchor_ids(&self, _node_ids: Vec<NodeId>) -> anyhow::Result<()> {
        todo!()
    }
}

fn setup() -> (Vec<ValidatorSigner>, Arc<EpochState>, Dag, Arc<MockStorage>) {
    let (signers, validator_verifier) = random_validator_verifier(4, None, false);
    let epoch_state = Arc::new(EpochState {
        epoch: 1,
        verifier: validator_verifier,
    });
    let storage = Arc::new(MockStorage::new());
    let dag = Dag::new(epoch_state.clone(), storage.clone());
    (signers, epoch_state, dag, storage)
}

#[test]
fn test_dag_insertion_succeed() {
    let (signers, epoch_state, mut dag, _) = setup();

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_certified_node(1, signer.author(), vec![]);
        assert!(dag.add_node(node).is_ok());
    }
    let parents = dag
        .get_strong_links_for_round(1, &epoch_state.verifier)
        .unwrap();

    // Round 2 nodes 0, 1, 2 links to 0, 1, 2
    for signer in &signers[0..3] {
        let node = new_certified_node(2, signer.author(), parents.clone());
        assert!(dag.add_node(node).is_ok());
    }

    // Round 3 nodes 1, 2 links to 0, 1, 2
    let parents = dag
        .get_strong_links_for_round(2, &epoch_state.verifier)
        .unwrap();

    for signer in &signers[1..3] {
        let node = new_certified_node(3, signer.author(), parents.clone());
        assert!(dag.add_node(node).is_ok());
    }

    // not enough strong links
    assert!(dag
        .get_strong_links_for_round(3, &epoch_state.verifier)
        .is_none());
}

#[test]
fn test_dag_insertion_failure() {
    let (signers, epoch_state, mut dag, _) = setup();

    // Round 1 - nodes 0, 1, 2 links to vec![]
    for signer in &signers[0..3] {
        let node = new_certified_node(1, signer.author(), vec![]);
        assert!(dag.add_node(node.clone()).is_ok());
        // duplicate node
        assert!(dag.add_node(node).is_err());
    }

    let missing_node = new_certified_node(1, signers[3].author(), vec![]);
    let mut parents = dag
        .get_strong_links_for_round(1, &epoch_state.verifier)
        .unwrap();
    parents.push(missing_node.certificate());

    let node = new_certified_node(2, signers[0].author(), parents.clone());
    // parents not exist
    assert!(dag.add_node(node).is_err());

    let node = new_certified_node(3, signers[0].author(), vec![]);
    // round too high
    assert!(dag.add_node(node).is_err());

    let node = new_certified_node(2, signers[0].author(), parents[0..3].to_vec());
    assert!(dag.add_node(node).is_ok());
    let node = new_certified_node(2, signers[0].author(), vec![]);
    // equivocation node
    assert!(dag.add_node(node).is_err());
}

#[test]
fn test_dag_recover_from_storage() {
    let (signers, epoch_state, mut dag, storage) = setup();

    let mut metadatas = vec![];

    for round in 1..10 {
        let parents = dag
            .get_strong_links_for_round(round, &epoch_state.verifier)
            .unwrap_or_default();
        for signer in &signers[0..3] {
            let node = new_certified_node(round, signer.author(), parents.clone());
            metadatas.push(node.metadata().clone());
            assert!(dag.add_node(node).is_ok());
        }
    }
    let new_dag = Dag::new(epoch_state.clone(), storage.clone());

    for metadata in &metadatas {
        assert!(new_dag.exists(metadata));
    }

    let new_epoch_state = Arc::new(EpochState {
        epoch: 2,
        verifier: epoch_state.verifier.clone(),
    });

    let _new_epoch_dag = Dag::new(new_epoch_state, storage.clone());
    assert!(storage.certified_node_data.lock().is_empty());
}

#[test]
fn test_dag_bitmask() {
    let (signers, epoch_state, mut dag, _) = setup();

    let mut metadatas = vec![];

    for round in 1..5 {
        let parents = dag
            .get_strong_links_for_round(round, &epoch_state.verifier)
            .unwrap_or_default();
        for signer in &signers[0..3] {
            let node = new_certified_node(round, signer.author(), parents.clone());
            metadatas.push(node.metadata().clone());
            assert!(dag.add_node(node).is_ok());
        }
    }
    assert_eq!(
        dag.bitmask(15),
        DagSnapshotBitmask::new(1, vec![vec![true, true, true, false]; 4])
    );

    for round in 1..5 {
        let parents = dag
            .get_strong_links_for_round(round, &epoch_state.verifier)
            .unwrap_or_default();
        let node = new_certified_node(round, signers[3].author(), parents.clone());
        metadatas.push(node.metadata().clone());
        assert!(dag.add_node(node).is_ok());
    }
    assert_eq!(dag.bitmask(15), DagSnapshotBitmask::new(5, vec![]));
    assert_eq!(dag.bitmask(6), DagSnapshotBitmask::new(5, vec![]));
}
