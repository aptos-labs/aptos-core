// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::rand::secret_sharing::storage::{storage_key, SecretShareKey, SecretShareStorage};
use aptos_consensus_types::common::{Author, Round};
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
use std::{collections::HashMap, sync::Arc};

pub struct PersistedSelfShares {
    storage: Arc<dyn SecretShareStorage>,
    retention_rounds: Round,
    shares: HashMap<SecretShareKey, SecretShare>,
}

impl PersistedSelfShares {
    pub fn new(
        epoch: u64,
        author: Author,
        storage: Arc<dyn SecretShareStorage>,
        highest_committed_round: Round,
        retention_rounds: Round,
    ) -> Self {
        let oldest_retained_round = highest_committed_round.saturating_sub(retention_rounds);
        let all_self_shares = storage
            .get_all_self_shares()
            .expect("Failed to load secret shares at epoch start");
        let mut shares = HashMap::new();
        let mut keys_to_prune = Vec::new();
        for share in all_self_shares {
            let key = storage_key(share.metadata());
            if share.epoch() < epoch {
                keys_to_prune.push(key);
                continue;
            }
            assert_eq!(
                share.epoch(),
                epoch,
                "Persisted secret share has wrong epoch"
            );
            assert_eq!(
                share.author(),
                &author,
                "Persisted secret share has wrong author"
            );
            if share.round() < oldest_retained_round {
                keys_to_prune.push(key);
                continue;
            }
            shares.insert(key, share);
        }
        storage
            .prune_self_shares(&keys_to_prune)
            .expect("Failed to prune stale secret shares at epoch start");

        Self {
            storage,
            retention_rounds,
            shares,
        }
    }

    pub fn persist(&mut self, share: SecretShare) -> anyhow::Result<()> {
        // Overwriting is safe: self-share derivation is deterministic, using the fixed epoch
        // master-secret share and the block's deterministic ciphertext/round digest, with no RNG.
        self.storage.save_self_share(&share)?;
        self.shares.insert(storage_key(share.metadata()), share);
        Ok(())
    }

    pub fn advance_retention(&mut self, latest_round: Round) {
        let oldest_retained_round = latest_round.saturating_sub(self.retention_rounds);
        let expired_keys = self
            .shares
            .iter()
            .filter_map(|(key, share)| (share.round() < oldest_retained_round).then_some(*key))
            .collect::<Vec<_>>();
        if expired_keys.is_empty() {
            return;
        }
        self.storage
            .prune_self_shares(&expired_keys)
            .expect("Failed to prune expired secret shares");
        for key in expired_keys {
            self.shares.remove(&key);
        }
    }

    pub fn get(&self, metadata: &SecretShareMetadata) -> Option<SecretShare> {
        let share = self.shares.get(&storage_key(metadata))?;
        if share.metadata() != metadata {
            return None;
        }
        Some(share.clone())
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.shares.len()
    }

    #[cfg(test)]
    pub fn contains_key(&self, key: &SecretShareKey) -> bool {
        self.shares.contains_key(key)
    }
}
