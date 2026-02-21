// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pipeline::pipeline_builder::{PipelineBuilder, Tracker};
use anyhow::{anyhow, Context};
use aptos_batch_encryption::{
    errors::MissingEvalProofError,
    schemes::fptx_weighted::FPTXWeighted,
    shared::{ciphertext::PreparedCiphertext, ids::Id},
    traits::BatchThresholdEncryption,
};
use aptos_consensus_types::{
    block::Block,
    common::Author,
    pipelined_block::{DecryptionResult, MaterializeResult, TaskFuture, TaskResult},
};
use aptos_logger::{error, info};
use aptos_types::{
    decryption::{BlockTxnDecryptionKey, DecKeyMetadata},
    secret_sharing::{
        Ciphertext, DecryptionKey, SecretShare, SecretShareConfig, SecretShareMetadata,
        SecretSharedKey,
    },
    transaction::encrypted_payload::DecryptedPayload,
};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::sync::Arc;
use tokio::sync::oneshot;

impl PipelineBuilder {
    /// Precondition: Block is materialized and the transactions are available locally
    /// What it does: Decrypt encrypted transactions in the block
    pub(crate) async fn decrypt_encrypted_txns(
        materialize_fut: TaskFuture<MaterializeResult>,
        block: Arc<Block>,
        author: Author,
        is_decryption_enabled: bool,
        maybe_secret_share_config: Option<SecretShareConfig>,
        derived_self_key_share_tx: oneshot::Sender<Option<SecretShare>>,
        secret_shared_key_rx: oneshot::Receiver<Option<SecretSharedKey>>,
    ) -> TaskResult<DecryptionResult> {
        let mut tracker = Tracker::start_waiting("decrypt_encrypted_txns", &block);
        let (input_txns, max_txns_from_block_to_execute, block_gas_limit) = materialize_fut.await?;

        tracker.start_working();

        // If decryption is disabled (by config or missing secret share config), pass through.
        if !is_decryption_enabled {
            return Ok((
                input_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
                None,
            ));
        }
        // If the config is None, then we are on the observer path:
        // no local secret share config available, so receive the pre-computed
        // decryption key via channel instead of deriving locally.
        // Assumption: `input_txns` is free of Encrypted Transactions
        // due to VM validation checks
        let Some(secret_share_config) = maybe_secret_share_config else {
            let _ = derived_self_key_share_tx.send(None);
            let maybe_key = secret_shared_key_rx
                .await
                .map_err(|_| anyhow!("secret_shared_key_rx dropped in observer path"))?;
            let dec_key = maybe_key.map(|key| {
                BlockTxnDecryptionKey::new(
                    DecKeyMetadata {
                        epoch: key.metadata.epoch,
                        round: key.metadata.round,
                    },
                    bcs::to_bytes(&key.key).expect("SecretSharedKey serialization"),
                )
            });
            return Ok((
                input_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
                Some(dec_key),
            ));
        };

        let (encrypted_txns, unencrypted_txns): (Vec<_>, Vec<_>) = input_txns
            .into_iter()
            .partition(|txn| txn.is_encrypted_txn());

        // TODO(ibalajiarun): figure out handling of empty encrypted txn vec

        // TODO(ibalajiarun): FIXME
        let len = 32;
        let encrypted_txns = if encrypted_txns.len() > len {
            let mut to_truncate = encrypted_txns;
            to_truncate.truncate(len);
            to_truncate
        } else {
            encrypted_txns
        };

        let txn_ciphertexts: Vec<Ciphertext> = encrypted_txns
            .iter()
            .map(|txn| {
                // TODO(ibalajiarun): Avoid clone and use reference instead
                txn.payload()
                    .as_encrypted_payload()
                    .expect("must be a encrypted txn")
                    .ciphertext()
                    .clone()
            })
            .collect();

        // TODO(ibalajiarun): Consider using commit block height to reduce trusted setup size
        // TODO(ibalajiarun): Fix this wrapping
        let encryption_round = block.round() % 200;
        let (digest, proofs_promise) = FPTXWeighted::digest(
            secret_share_config.digest_key(),
            &txn_ciphertexts,
            encryption_round,
        )?;

        let metadata = SecretShareMetadata::new(
            block.epoch(),
            block.round(),
            block.timestamp_usecs(),
            block.id(),
            digest.clone(),
        );

        let derived_key_share =
            FPTXWeighted::derive_decryption_key_share(secret_share_config.msk_share(), &digest)?;
        if derived_self_key_share_tx
            .send(Some(SecretShare::new(
                author,
                metadata.clone(),
                derived_key_share,
            )))
            .is_err()
        {
            return Err(anyhow!(
                "derived_self_key_share_tx receiver dropped, pipeline likely aborted"
            )
            .into());
        }

        // TODO(ibalajiarun): improve perf
        let proofs = FPTXWeighted::eval_proofs_compute_all(
            &proofs_promise,
            secret_share_config.digest_key(),
        );

        let prepared_txn_ciphertexts: Vec<Result<PreparedCiphertext, MissingEvalProofError>> =
            txn_ciphertexts
                .into_par_iter()
                .map(|ciphertext| FPTXWeighted::prepare_ct(&ciphertext, &digest, &proofs))
                .collect();

        let maybe_decryption_key = secret_shared_key_rx
            .await
            .map_err(|_| anyhow!("secret_shared_key_rx dropped"))?;
        // TODO(ibalajiarun): account for the case where decryption key is not available
        let decryption_key = maybe_decryption_key.expect("decryption key should be available");

        info!(
            "Successfully received decryption key for block {}: metadata={:?}",
            block.round(),
            decryption_key.metadata
        );

        let decrypted_txns: Vec<_> = encrypted_txns
            .into_par_iter()
            .zip(prepared_txn_ciphertexts)
            .map(|(mut txn, prepared_ciphertext_or_error)| {
                let id: Id = prepared_ciphertext_or_error
                    .as_ref()
                    .map_or_else(|MissingEvalProofError(id)| *id, |ct| ct.id());
                let eval_proof = proofs.get(&id).expect("must exist");

                match do_final_decryption(&decryption_key.key, prepared_ciphertext_or_error) {
                    Ok(payload) => {
                        let (executable, nonce) = payload.unwrap();
                        txn.payload_mut()
                            .as_encrypted_payload_mut()
                            .map(|p| {
                                p.into_decrypted(eval_proof, executable, nonce)
                                    .expect("must happen")
                            })
                            .expect("must exist");
                    },
                    Err(e) => {
                        error!(
                            "Failed to decrypt transaction with ciphertext id {:?}: {:?}",
                            id, e
                        );
                        txn.payload_mut()
                            .as_encrypted_payload_mut()
                            .map(|p| p.into_failed_decryption(eval_proof).expect("must happen"))
                            .expect("must exist");
                    },
                }
                txn
            })
            .collect();
        info!(
            "Decryption complete for block {}: {} encrypted, {} unencrypted",
            block.round(),
            decrypted_txns.len(),
            unencrypted_txns.len()
        );
        let output_txns = [decrypted_txns, unencrypted_txns].concat();

        let block_txn_dec_key = BlockTxnDecryptionKey::new(
            DecKeyMetadata {
                epoch: decryption_key.metadata.epoch,
                round: decryption_key.metadata.round,
            },
            bcs::to_bytes(&decryption_key.key).context("Decryption key serialization failed")?,
        );

        Ok((
            output_txns,
            max_txns_from_block_to_execute,
            block_gas_limit,
            Some(Some(block_txn_dec_key)),
        ))
    }
}

fn do_final_decryption(
    decryption_key: &DecryptionKey,
    prepared_ciphertext_or_error: Result<PreparedCiphertext, MissingEvalProofError>,
) -> anyhow::Result<DecryptedPayload> {
    FPTXWeighted::decrypt(decryption_key, &prepared_ciphertext_or_error?)
}
