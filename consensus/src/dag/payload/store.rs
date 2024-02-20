use crate::dag::storage::DAGStorage;
use anyhow::ensure;
use aptos_consensus_types::{
    common::{Author, Round},
    dag_payload::{DecoupledPayload, PayloadDigest, PayloadId},
};
use aptos_crypto::HashValue;
use aptos_infallible::RwLock;
use aptos_logger::{error, info};
use aptos_types::epoch_state::EpochState;
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use thiserror::Error as ThisError;

#[derive(Clone, Debug, ThisError, Serialize, Deserialize)]
pub enum DagPayloadStoreError {
    #[error("payload is missing {0}")]
    Missing(PayloadId),
    #[error("garbage collected, request round {0}, lowest round {1}")]
    GarbageCollected(Round, Round),
}

pub struct DagPayloadStore {
    payload_by_digest: DashMap<HashValue, Arc<DecoupledPayload>>,
    storage: Arc<dyn DAGStorage>,
    /// Map between peer id to vector index
    author_to_index: HashMap<Author, usize>,
    start_round: RwLock<Round>,
    epoch_state: Arc<EpochState>,
    /// The window we maintain between highest committed round and initial round
    window_size: u64,
}

impl DagPayloadStore {
    pub fn new(
        epoch_state: Arc<EpochState>,
        storage: Arc<dyn DAGStorage>,
        start_round: Round,
        window_size: u64,
    ) -> Self {
        let mut all_payloads = storage.get_payloads().unwrap_or_default();
        all_payloads.sort_unstable_by_key(|(_, payload)| payload.info().round());

        let author_to_index = epoch_state.verifier.address_to_validator_index().clone();
        let mut to_prune = vec![];
        let store = Self {
            payload_by_digest: DashMap::new(),
            storage: storage.clone(),
            author_to_index,
            start_round: RwLock::new(start_round),
            epoch_state,
            window_size,
        };

        for (digest, payload) in all_payloads {
            if let Err(e) = store.insert(payload) {
                info!("pruning payload {} due to {}", digest, e);
                to_prune.push(digest);
            }
        }
        if let Err(e) = storage.delete_payloads(to_prune) {
            error!("Error deleting expired payloads: {:?}", e);
        }
        store
    }

    pub fn prune(&self, digests: Vec<PayloadDigest>) {
        for digest in &digests {
            self.payload_by_digest.remove(digest);
        }
        if let Err(e) = self.storage.delete_certified_nodes(digests) {
            error!("Error deleting expired payload: {:?}", e);
        }
    }

    pub fn commit_callback(&self, commit_round: Round, digests: Vec<PayloadDigest>) {
        let mut start_round = self.start_round.write();
        let new_start_round = commit_round.saturating_sub(3 * self.window_size);
        if new_start_round > *start_round {
            *start_round = new_start_round;
            self.prune(digests);
        }
    }

    pub fn is_missing(&self, id: &PayloadId, digest: &PayloadDigest) -> bool {
        self.get(id, digest)
            .is_err_and(|e| matches!(e, DagPayloadStoreError::Missing(_)))
    }

    pub fn insert(&self, payload: DecoupledPayload) -> anyhow::Result<()> {
        let digest = payload.digest();

        ensure!(
            payload.info().epoch() == self.epoch_state.epoch,
            "different epoch {}, current {}",
            payload.info().epoch(),
            self.epoch_state.epoch
        );
        let author = payload.author();
        self.author_to_index
            .get(author)
            .ok_or_else(|| anyhow::anyhow!("unknown author"))?;
        let round = payload.info().round();
        let lowest_round = self.start_round.read();
        ensure!(
            round >= *lowest_round,
            "round too low {}, lowest in dag {}",
            round,
            *lowest_round
        );

        ensure!(!self.payload_by_digest.contains_key(digest));

        self.storage.save_payload(&payload)?;

        self.payload_by_digest.insert(*digest, Arc::new(payload));

        Ok(())
    }

    pub fn get(
        &self,
        id: &PayloadId,
        digest: &PayloadDigest,
    ) -> Result<Arc<DecoupledPayload>, DagPayloadStoreError> {
        let lowest_round = self.start_round.read();
        if id.round() < *lowest_round {
            return Err(DagPayloadStoreError::GarbageCollected(
                id.round(),
                *lowest_round,
            ));
        }
        Ok(self
            .payload_by_digest
            .get(digest)
            .map(|node_payload| node_payload.clone())
            .ok_or_else(|| DagPayloadStoreError::Missing(id.clone()))?)
    }
}
