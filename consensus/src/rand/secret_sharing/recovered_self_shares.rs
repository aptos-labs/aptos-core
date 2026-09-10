// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::rand::secret_sharing::storage::{storage_key, SecretShareKey, SecretShareStorage};
use aptos_consensus_types::common::{Author, Round};
use aptos_logger::error;
use aptos_types::secret_sharing::{SecretShare, SecretShareMetadata};
use std::{collections::HashMap, sync::Arc};

pub enum RecoveredShare {
    Verified(SecretShare),
    Unverified(SecretShare),
}

struct RecoveredSelfShare {
    share: SecretShare,
    verified: bool,
}

pub struct RecoveredSelfShares {
    epoch: u64,
    storage: Arc<dyn SecretShareStorage>,
    retention_rounds: Round,
    shares: HashMap<SecretShareKey, RecoveredSelfShare>,
    pending_epoch_prune: bool,
    pending_prune_before_round: Option<Round>,
}

impl RecoveredSelfShares {
    pub fn new(
        epoch: u64,
        author: Author,
        storage: Arc<dyn SecretShareStorage>,
        highest_committed_round: Round,
        retention_rounds: Round,
    ) -> Self {
        let pending_epoch_prune = if let Err(error) = storage.prune_before_epoch(epoch) {
            error!(
                epoch = epoch,
                "Failed to prune old secret shares at epoch start; will retry: {error}"
            );
            true
        } else {
            false
        };
        let oldest_retained_round = highest_committed_round.saturating_sub(retention_rounds);
        let pending_prune_before_round =
            if let Err(error) = storage.prune_before_round(epoch, oldest_retained_round) {
                error!(
                    epoch = epoch,
                    oldest_retained_round = oldest_retained_round,
                    "Failed to prune expired secret shares at epoch start; will retry: {error}"
                );
                Some(oldest_retained_round)
            } else {
                None
            };

        let loaded_self_shares = storage
            .load_self_shares(epoch)
            .unwrap_or_else(|error| panic!("Failed to load secret shares at epoch start: {error}"));
        let mut shares = HashMap::new();
        for (key, loaded_share) in loaded_self_shares {
            let share = match loaded_share {
                Ok(share) => share,
                Err(error) => {
                    error!(
                        epoch = key.0,
                        block_id = key.1,
                        "Deleting invalid persisted secret share: {error}"
                    );
                    Self::delete_or_panic(&storage, &key, "invalid");
                    continue;
                },
            };
            if share.epoch() != epoch || share.author() != &author {
                error!(
                    expected_epoch = epoch,
                    share_epoch = share.epoch(),
                    expected_author = author,
                    share_author = share.author(),
                    "Deleting persisted secret share with invalid identity"
                );
                Self::delete_or_panic(&storage, &key, "invalid identity");
                continue;
            }
            // Enforce the retention window independently of whether physical
            // database pruning succeeded.
            if share.round() < oldest_retained_round {
                continue;
            }
            shares.insert(key, RecoveredSelfShare {
                share,
                verified: false,
            });
        }

        Self {
            epoch,
            storage,
            retention_rounds,
            shares,
            pending_epoch_prune,
            pending_prune_before_round,
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
        let removed_shares = self.shares.len() != previous_len;

        if self.pending_epoch_prune {
            if let Err(error) = self.storage.prune_before_epoch(self.epoch) {
                error!(
                    epoch = self.epoch,
                    "Failed to retry pruning old secret shares: {error}"
                );
            } else {
                self.pending_epoch_prune = false;
            }
        }

        if removed_shares || self.pending_prune_before_round.is_some() {
            let prune_before_round = self
                .pending_prune_before_round
                .unwrap_or_default()
                .max(oldest_retained_round);
            if let Err(error) = self
                .storage
                .prune_before_round(self.epoch, prune_before_round)
            {
                self.pending_prune_before_round = Some(prune_before_round);
                error!(
                    epoch = self.epoch,
                    oldest_retained_round = prune_before_round,
                    "Failed to prune expired secret shares; will retry: {error}"
                );
            } else {
                self.pending_prune_before_round = None;
            }
        }
    }

    pub fn get(&self, metadata: &SecretShareMetadata) -> Option<RecoveredShare> {
        let recovered = self.shares.get(&storage_key(metadata))?;
        if recovered.share.metadata() != metadata {
            return None;
        }
        if recovered.verified {
            Some(RecoveredShare::Verified(recovered.share.clone()))
        } else {
            Some(RecoveredShare::Unverified(recovered.share.clone()))
        }
    }

    pub fn mark_verified(&mut self, key: &SecretShareKey) -> Option<SecretShare> {
        let recovered = self.shares.get_mut(key)?;
        recovered.verified = true;
        Some(recovered.share.clone())
    }

    pub fn delete_invalid(&mut self, key: &SecretShareKey) {
        self.shares.remove(key);
        Self::delete_or_panic(&self.storage, key, "cryptographically invalid");
    }

    fn delete_or_panic(storage: &Arc<dyn SecretShareStorage>, key: &SecretShareKey, reason: &str) {
        storage.delete_self_share(key).unwrap_or_else(|error| {
            panic!(
                "Failed to delete {reason} persisted secret share for epoch {}, block {}: {error}",
                key.0, key.1
            )
        });
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::secret_sharing::{
        storage::{InMemorySecretShareStorage, LoadedSecretShare},
        test_utils::{create_metadata, create_secret_share, TestContext},
    };
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct FlakyPruneStorage {
        inner: InMemorySecretShareStorage,
        fail_round_prune_attempt: usize,
        round_prune_attempts: AtomicUsize,
    }

    impl FlakyPruneStorage {
        fn new(fail_round_prune_attempt: usize) -> Self {
            Self {
                inner: InMemorySecretShareStorage::new(),
                fail_round_prune_attempt,
                round_prune_attempts: AtomicUsize::new(0),
            }
        }
    }

    impl SecretShareStorage for FlakyPruneStorage {
        fn save_self_share(&self, share: &SecretShare) -> anyhow::Result<()> {
            self.inner.save_self_share(share)
        }

        fn delete_self_share(&self, key: &SecretShareKey) -> anyhow::Result<()> {
            self.inner.delete_self_share(key)
        }

        fn load_self_shares(&self, epoch: u64) -> anyhow::Result<Vec<LoadedSecretShare>> {
            self.inner.load_self_shares(epoch)
        }

        fn prune_before_epoch(&self, epoch: u64) -> anyhow::Result<()> {
            self.inner.prune_before_epoch(epoch)
        }

        fn prune_before_round(&self, epoch: u64, round: Round) -> anyhow::Result<()> {
            let attempt = self.round_prune_attempts.fetch_add(1, Ordering::Relaxed);
            if attempt == self.fail_round_prune_attempt {
                anyhow::bail!("injected round prune failure")
            }
            self.inner.prune_before_round(epoch, round)
        }
    }

    #[test]
    fn startup_filters_expired_shares_when_pruning_fails_and_retries() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(FlakyPruneStorage::new(0));
        let old_metadata = create_metadata(ctx.epoch, 10);
        let boundary_metadata = create_metadata(ctx.epoch, 20);
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &old_metadata))
            .unwrap();
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &boundary_metadata))
            .unwrap();

        let mut recovered =
            RecoveredSelfShares::new(ctx.epoch, ctx.authors[0], storage.clone(), 30, 10);

        assert!(!recovered.contains_key(&storage_key(&old_metadata)));
        assert!(recovered.contains_key(&storage_key(&boundary_metadata)));
        assert_eq!(storage.load_self_shares(ctx.epoch).unwrap().len(), 2);

        recovered.advance_retention(31);

        assert_eq!(storage.round_prune_attempts.load(Ordering::Relaxed), 2);
        assert!(storage.load_self_shares(ctx.epoch).unwrap().is_empty());
    }

    #[test]
    fn runtime_pruning_failure_is_retried_after_cache_eviction() {
        let ctx = TestContext::new(vec![1, 1, 1, 1]);
        let storage = Arc::new(FlakyPruneStorage::new(1));
        let metadata = create_metadata(ctx.epoch, 10);
        storage
            .save_self_share(&create_secret_share(&ctx, 0, &metadata))
            .unwrap();
        let mut recovered =
            RecoveredSelfShares::new(ctx.epoch, ctx.authors[0], storage.clone(), 10, 10);

        recovered.advance_retention(21);
        assert!(!recovered.contains_key(&storage_key(&metadata)));
        assert_eq!(storage.load_self_shares(ctx.epoch).unwrap().len(), 1);

        recovered.advance_retention(22);

        assert_eq!(storage.round_prune_attempts.load(Ordering::Relaxed), 3);
        assert!(storage.load_self_shares(ctx.epoch).unwrap().is_empty());
    }
}
