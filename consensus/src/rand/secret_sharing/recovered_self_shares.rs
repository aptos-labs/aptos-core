// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::rand::secret_sharing::{
    storage::{storage_key, SecretShareKey, SecretShareStorage},
    verifier::SecretShareVerifier,
};
use aptos_consensus_types::common::{Author, Round};
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
use std::{collections::HashMap, sync::Arc};

struct RecoveredSelfShare {
    share: SecretShare,
    verified: bool,
}

pub struct RecoveredSelfShares {
    storage: Arc<dyn SecretShareStorage>,
    retention_rounds: Round,
    shares: HashMap<SecretShareKey, RecoveredSelfShare>,
}

impl RecoveredSelfShares {
    pub fn new(
        epoch: u64,
        author: Author,
        storage: Arc<dyn SecretShareStorage>,
        highest_committed_round: Round,
        retention_rounds: Round,
    ) -> Self {
        storage
            .prune_before_epoch(epoch)
            .expect("Failed to prune old secret shares at epoch start");
        let oldest_retained_round = highest_committed_round.saturating_sub(retention_rounds);
        let loaded_self_shares = storage
            .load_self_shares(epoch)
            .expect("Failed to load secret shares at epoch start");
        let mut shares = HashMap::new();
        let mut expired_keys = Vec::new();
        for share in loaded_self_shares {
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
            let key = storage_key(share.metadata());
            if share.round() < oldest_retained_round {
                expired_keys.push(key);
                continue;
            }
            shares.insert(key, RecoveredSelfShare {
                share,
                verified: false,
            });
        }
        storage
            .prune_self_shares(&expired_keys)
            .expect("Failed to prune expired secret shares at epoch start");

        Self {
            storage,
            retention_rounds,
            shares,
        }
    }

    pub fn persist(&mut self, share: SecretShare) -> anyhow::Result<()> {
        self.storage.save_self_share(&share)?;
        self.shares
            .insert(storage_key(share.metadata()), RecoveredSelfShare {
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
