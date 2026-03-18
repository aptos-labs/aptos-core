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
    pipelined_block::{DecryptionResult, MaterializeResult, TaskError, TaskFuture, TaskResult},
};
use aptos_logger::{error, info};
use aptos_types::{
    decryption::BlockTxnDecryptionKey,
    secret_sharing::{
        Ciphertext, DecryptionKey, SecretShare, SecretShareConfig, SecretShareMetadata,
        SecretSharedKey,
    },
    transaction::{encrypted_payload::DecryptedPayload, SignedTransaction},
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
        observer_enabled: bool,
        observer_decrypted_txns: Option<Vec<SignedTransaction>>,
    ) -> TaskResult<DecryptionResult> {
        let mut tracker = Tracker::start_waiting("decrypt_encrypted_txns", &block);
        let (input_txns, max_txns_from_block_to_execute, block_gas_limit) = materialize_fut.await?;
        tracker.start_working();

        // TODO(ibalajiarun): if decryption is disabled, convert encrypted txns to failed decryption.
        if !is_decryption_enabled {
            return Ok(DecryptionResult::passthrough(
                input_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
                None,
            ));
        }

        // Assumption: `input_txns` is free of Encrypted Transactions
        // due to VM validation checks
        let Some(secret_share_config) = maybe_secret_share_config else {
            let _ = derived_self_key_share_tx.send(None);

            // Consensus node without secret share config (e.g. bootstrapping
            // epoch where chunky DKG is newly enabled but hasn't completed yet).
            // Return immediately with no decryption key to avoid a circular
            // dependency: has_rand_txns_fut -> prepare -> decrypt (waiting for
            // secret_shared_key_rx) -> ordering (blocked on has_rand_txns_fut).
            if !observer_enabled {
                return Ok(DecryptionResult::passthrough(
                    input_txns,
                    max_txns_from_block_to_execute,
                    block_gas_limit,
                    Some(None),
                ));
            }

            return decrypt_observer_path(
                input_txns,
                secret_shared_key_rx,
                observer_decrypted_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
            )
            .await;
        };

        decrypt_validator_path(
            input_txns,
            &block,
            author,
            &secret_share_config,
            derived_self_key_share_tx,
            secret_shared_key_rx,
            max_txns_from_block_to_execute,
            block_gas_limit,
        )
        .await
    }
}

/// Observer path: wait for the decryption key from the ordering path, then use
/// pre-decrypted transactions provided by the validator.
async fn decrypt_observer_path(
    input_txns: Vec<SignedTransaction>,
    secret_shared_key_rx: oneshot::Receiver<Option<SecretSharedKey>>,
    observer_decrypted_txns: Option<Vec<SignedTransaction>>,
    max_txns_from_block_to_execute: Option<u64>,
    block_gas_limit: Option<u64>,
) -> TaskResult<DecryptionResult> {
    let maybe_key = secret_shared_key_rx
        .await
        .map_err(|_| anyhow!("secret_shared_key_rx dropped in observer path"))?;

    if maybe_key.is_none() {
        if observer_decrypted_txns.is_some() {
            return Err(TaskError::InternalError(Arc::new(anyhow!(
                "observer decrypted txns should not be available if decryption key is not available"
            ))));
        }
        return Ok(DecryptionResult::passthrough(
            input_txns,
            max_txns_from_block_to_execute,
            block_gas_limit,
            Some(None),
        ));
    }

    if observer_decrypted_txns.is_none() {
        return Err(TaskError::InternalError(Arc::new(anyhow!(
            "observer decrypted txns should be available"
        ))));
    }

    // Partition out encrypted txns, discard them, use pre-decrypted txns from validator.
    let (_, regular_txns): (Vec<_>, Vec<_>) = input_txns
        .into_iter()
        .partition(|txn| txn.is_encrypted_txn());

    let dec_key = maybe_key
        .map(|key| BlockTxnDecryptionKey::from_secret_shared_key(&key))
        .transpose()?;

    Ok(DecryptionResult {
        decrypted_txns: observer_decrypted_txns
            .expect("observer decrypted txns should be available"),
        regular_txns,
        max_txns_from_block_to_execute,
        block_gas_limit,
        decryption_key: Some(dec_key),
    })
}

/// Validator path: derive key share, prepare ciphertexts, await the shared
/// decryption key, and decrypt all encrypted transactions.
async fn decrypt_validator_path(
    input_txns: Vec<SignedTransaction>,
    block: &Block,
    author: Author,
    secret_share_config: &SecretShareConfig,
    derived_self_key_share_tx: oneshot::Sender<Option<SecretShare>>,
    secret_shared_key_rx: oneshot::Receiver<Option<SecretSharedKey>>,
    max_txns_from_block_to_execute: Option<u64>,
    block_gas_limit: Option<u64>,
) -> TaskResult<DecryptionResult> {
    let (encrypted_txns, regular_txns): (Vec<_>, Vec<_>) = input_txns
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
        return Err(
            anyhow!("derived_self_key_share_tx receiver dropped, pipeline likely aborted").into(),
        );
    }

    // TODO(ibalajiarun): improve perf
    let proofs =
        FPTXWeighted::eval_proofs_compute_all(&proofs_promise, secret_share_config.digest_key());

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
        regular_txns.len()
    );

    let block_txn_dec_key = BlockTxnDecryptionKey::from_secret_shared_key(&decryption_key)
        .context("Decryption key serialization failed")?;

    Ok(DecryptionResult {
        decrypted_txns,
        regular_txns,
        max_txns_from_block_to_execute,
        block_gas_limit,
        decryption_key: Some(Some(block_txn_dec_key)),
    })
}

fn do_final_decryption(
    decryption_key: &DecryptionKey,
    prepared_ciphertext_or_error: Result<PreparedCiphertext, MissingEvalProofError>,
) -> anyhow::Result<DecryptedPayload> {
    FPTXWeighted::decrypt(decryption_key, &prepared_ciphertext_or_error?)
}
