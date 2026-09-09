// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    block_storage::BlockReader,
    payload_manager::TPayloadManager,
    pipeline::decryption_pipeline_builder::{
        derive_secret_share, select_encrypted_txns_for_secret_share, SecretShareSelection,
    },
};
use anyhow::{anyhow, ensure, Context};
use aptos_consensus_types::pipelined_block::{DecryptionOutcome, PipelinedBlock};
use aptos_infallible::RwLock;
use aptos_types::secret_sharing::{Author, SecretShare, SecretShareConfig, SecretShareMetadata};
use async_trait::async_trait;
use futures::FutureExt;
use std::sync::Arc;

#[async_trait]
pub trait SecretShareRecovery: Send + Sync {
    async fn recover(&self, metadata: SecretShareMetadata) -> anyhow::Result<Option<SecretShare>>;
}

/// Startup creates the secret-share manager before the block store. This
/// forwarding implementation lets the concrete, read-only recovery service be
/// injected once the block store exists without coupling the manager to it.
#[derive(Default)]
pub struct LateBoundSecretShareRecovery {
    inner: RwLock<Option<Arc<dyn SecretShareRecovery>>>,
}

impl LateBoundSecretShareRecovery {
    pub fn clear(&self) {
        *self.inner.write() = None;
    }

    pub fn set(&self, recovery: Arc<dyn SecretShareRecovery>) {
        *self.inner.write() = Some(recovery);
    }
}

#[async_trait]
impl SecretShareRecovery for LateBoundSecretShareRecovery {
    async fn recover(&self, metadata: SecretShareMetadata) -> anyhow::Result<Option<SecretShare>> {
        let recovery = self
            .inner
            .read()
            .clone()
            .ok_or_else(|| anyhow!("secret-share recovery is not initialized"))?;
        recovery.recover(metadata).await
    }
}

pub struct OrderedBlockSecretShareRecovery {
    author: Author,
    secret_share_config: SecretShareConfig,
    payload_manager: Arc<dyn TPayloadManager>,
    block_reader: Arc<dyn BlockReader>,
}

impl OrderedBlockSecretShareRecovery {
    pub fn new(
        author: Author,
        secret_share_config: SecretShareConfig,
        payload_manager: Arc<dyn TPayloadManager>,
        block_reader: Arc<dyn BlockReader>,
    ) -> Self {
        Self {
            author,
            secret_share_config,
            payload_manager,
            block_reader,
        }
    }

    fn outcome_round(outcome: &DecryptionOutcome) -> anyhow::Result<Option<u64>> {
        match outcome {
            DecryptionOutcome::Disabled => Err(anyhow!("decryption is disabled")),
            DecryptionOutcome::Legacy(_) => Ok(None),
            DecryptionOutcome::RoundTracked {
                next_decryption_round,
                payload: _,
            } => Ok(Some(*next_decryption_round)),
        }
    }

    fn retained_parent_round(parent: &PipelinedBlock) -> anyhow::Result<Option<Option<u64>>> {
        let Some(futures) = parent.pipeline_futs() else {
            return Ok(None);
        };
        match futures.decryption_fut.now_or_never() {
            Some(Ok(result)) => Self::outcome_round(&result.outcome).map(Some),
            Some(Err(_)) => Ok(None),
            None => Ok(None),
        }
    }

    async fn materialize(
        &self,
        block: &PipelinedBlock,
    ) -> anyhow::Result<(
        Vec<aptos_types::transaction::SignedTransaction>,
        Option<u64>,
    )> {
        self.payload_manager
            .check_payload_availability(block.block())
            .map_err(|missing| anyhow!("payload is unavailable locally: {missing:?}"))?;
        let (txns, max_txns_from_block_to_execute, _block_gas_limit) = self
            .payload_manager
            .get_transactions(block.block(), None)
            .await
            .context("failed to materialize ordered block payload")?;
        Ok((txns, max_txns_from_block_to_execute))
    }

    async fn reconstruct_parent_round(
        &self,
        commit_root: &PipelinedBlock,
        ancestors: &[Arc<PipelinedBlock>],
    ) -> anyhow::Result<Option<u64>> {
        let seed = Self::retained_parent_round(commit_root)?
            .ok_or_else(|| anyhow!("committed-root decryption outcome is unavailable"))?;
        let Some(mut next_decryption_round) = seed else {
            return Ok(None);
        };

        for block in ancestors {
            let (txns, max_txns_from_block_to_execute) = self.materialize(block).await?;
            let encrypted_txns_len = txns.iter().filter(|txn| txn.is_encrypted_txn()).count();
            if matches!(
                select_encrypted_txns_for_secret_share(
                    block.block(),
                    &self.secret_share_config,
                    encrypted_txns_len,
                    max_txns_from_block_to_execute,
                    next_decryption_round,
                ),
                SecretShareSelection::Selected {
                    selected_len: 1..,
                    ..
                }
            ) {
                next_decryption_round = next_decryption_round
                    .checked_add(1)
                    .ok_or_else(|| anyhow!("decryption round overflow"))?;
            }
        }
        Ok(Some(next_decryption_round))
    }
}

#[async_trait]
impl SecretShareRecovery for OrderedBlockSecretShareRecovery {
    async fn recover(&self, requested: SecretShareMetadata) -> anyhow::Result<Option<SecretShare>> {
        let commit_root = self.block_reader.commit_root();
        let ordered_root = self.block_reader.ordered_root();
        let canonical_path = self
            .block_reader
            .path_from_commit_root(ordered_root.id())
            .ok_or_else(|| anyhow!("ordered root is not a descendant of the commit root"))?;

        // Reject a moving-tree snapshot instead of accidentally accepting a
        // block from a path that ceased to be canonical while it was read.
        ensure!(
            self.block_reader.commit_root().id() == commit_root.id()
                && self.block_reader.ordered_root().id() == ordered_root.id(),
            "block-store roots changed during recovery"
        );

        let target_index = canonical_path
            .iter()
            .position(|block| block.id() == requested.block_id)
            .ok_or_else(|| anyhow!("requested block is not on the canonical ordered path"))?;
        let target = &canonical_path[target_index];
        ensure!(
            target.epoch() == requested.epoch,
            "requested epoch mismatch"
        );
        ensure!(
            target.round() == requested.round,
            "requested round mismatch"
        );
        ensure!(
            target.timestamp_usecs() == requested.timestamp,
            "requested timestamp mismatch"
        );
        ensure!(
            target.id() == requested.block_id,
            "requested block ID mismatch"
        );

        let parent = if target_index == 0 {
            commit_root.as_ref()
        } else {
            canonical_path[target_index - 1].as_ref()
        };
        let chained_round = match Self::retained_parent_round(parent)? {
            Some(round) => round,
            None => {
                self.reconstruct_parent_round(&commit_root, &canonical_path[..target_index])
                    .await?
            },
        };
        let digest_round = chained_round.unwrap_or_else(|| target.round());

        let (txns, max_txns_from_block_to_execute) = self.materialize(target).await?;
        let encrypted_txns: Vec<_> = txns
            .into_iter()
            .filter(|txn| txn.is_encrypted_txn())
            .collect();
        let selected_len = match select_encrypted_txns_for_secret_share(
            target.block(),
            &self.secret_share_config,
            encrypted_txns.len(),
            max_txns_from_block_to_execute,
            digest_round,
        ) {
            SecretShareSelection::Selected { selected_len, .. } => {
                if selected_len == 0 {
                    return Ok(None);
                }
                selected_len
            },
            SecretShareSelection::NoEncryptedTransactions
            | SecretShareSelection::TrustedSetupExhausted
            | SecretShareSelection::EpochEnd => return Ok(None),
        };

        let (_, _, _, share) = derive_secret_share(
            &encrypted_txns[..selected_len],
            target.block(),
            self.author,
            &self.secret_share_config,
            digest_round,
        )
        .await?;
        ensure!(
            share.metadata() == &requested,
            "locally reconstructed metadata does not match the request"
        );
        Ok(Some(share))
    }
}
