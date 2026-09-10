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
    epoch: u64,
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
        storage.prune_before_epoch(epoch).unwrap_or_else(|error| {
            panic!("Failed to prune old secret shares at epoch start: {error}")
        });
        let oldest_retained_round = highest_committed_round.saturating_sub(retention_rounds);
        storage
            .prune_before_round(epoch, oldest_retained_round)
            .unwrap_or_else(|error| {
                panic!("Failed to prune expired secret shares at epoch start: {error}")
            });

        let loaded_self_shares = storage
            .load_self_shares(epoch)
            .unwrap_or_else(|error| panic!("Failed to load secret shares at epoch start: {error}"));
        let mut shares = HashMap::new();
        for loaded_share in loaded_self_shares {
            let share = loaded_share
                .unwrap_or_else(|error| panic!("Invalid persisted secret share: {error}"));
            assert!(
                share.epoch() == epoch && share.author() == &author,
                "Persisted secret share has invalid identity: expected epoch {epoch} and author \
                 {author}, got epoch {} and author {}",
                share.epoch(),
                share.author(),
            );
            shares.insert(storage_key(share.metadata()), RecoveredSelfShare {
                share,
                verified: false,
            });
        }

        Self {
            epoch,
            storage,
            retention_rounds,
            shares,
        }
    }

    pub fn persist(&mut self, share: SecretShare) -> anyhow::Result<()> {
        self.storage.save_self_share(&share)?;
        let round = share.round();
        self.shares
            .insert(storage_key(share.metadata()), RecoveredSelfShare {
                share,
                verified: false,
            });
        self.advance_retention(round);
        Ok(())
    }

    pub fn advance_retention(&mut self, latest_round: Round) {
        let oldest_retained_round = latest_round.saturating_sub(self.retention_rounds);
        let previous_len = self.shares.len();
        self.shares
            .retain(|_, recovered| recovered.share.round() >= oldest_retained_round);
        if self.shares.len() != previous_len {
            self.storage
                .prune_before_round(self.epoch, oldest_retained_round)
                .unwrap_or_else(|error| {
                    panic!(
                        "Failed to prune secret shares before round {oldest_retained_round}: \
                         {error}"
                    )
                });
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
                .unwrap_or_else(|error| panic!("Invalid persisted secret share: {error}"));
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
