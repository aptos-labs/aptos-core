// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::rand::secret_sharing::{
    storage::{storage_key, SecretShareKey, SecretShareStorage},
    verifier::SecretShareVerifier,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
use std::{collections::HashMap, sync::Arc};

struct CachedSelfShare {
    share: SecretShare,
    verified: bool,
}

pub struct PersistedSelfShares {
    storage: Arc<dyn SecretShareStorage>,
    retention_rounds: Round,
    shares: HashMap<SecretShareKey, CachedSelfShare>,
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
            shares.insert(key, CachedSelfShare {
                share,
                verified: false,
            });
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
        self.shares
            .insert(storage_key(share.metadata()), CachedSelfShare {
                share,
                verified: false,
            });
        Ok(())
    }

    pub fn advance_retention(&mut self, latest_round: Round) {
        let oldest_retained_round = latest_round.saturating_sub(self.retention_rounds);
        let expired_keys = self
            .shares
            .iter()
            .filter_map(|(key, recovered)| {
                (recovered.share.round() < oldest_retained_round).then_some(*key)
            })
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

    pub fn get(
        &mut self,
        metadata: &SecretShareMetadata,
        verifier: &SecretShareVerifier,
        author: &Author,
    ) -> Option<SecretShare> {
        let recovered = self.shares.get_mut(&storage_key(metadata))?;
        if recovered.share.metadata() != metadata {
            return None;
        }
        if !recovered.verified {
            verifier
                .verify(&recovered.share, author)
                .expect("Invalid persisted secret share");
            recovered.verified = true;
        }
        Some(recovered.share.clone())
    }

    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.shares.len()
    }

    #[cfg(test)]
    pub fn contains_key(&self, key: &SecretShareKey) -> bool {
        self.shares.contains_key(key)
    }

    #[cfg(test)]
    pub fn is_verified(&self, key: &SecretShareKey) -> bool {
        self.shares
            .get(key)
            .is_some_and(|recovered| recovered.verified)
    }
}
