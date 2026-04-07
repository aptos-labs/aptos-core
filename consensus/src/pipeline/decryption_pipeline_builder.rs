// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    counters, monitor,
    pipeline::pipeline_builder::{PipelineBuilder, Tracker},
};
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
use aptos_logger::{error, info, warn};
use aptos_types::{
    decryption::BlockTxnDecryptionKey,
    secret_sharing::{
        Ciphertext, DecryptionKey, EvalProof, SecretShare, SecretShareConfig, SecretShareMetadata,
        SecretSharedKey,
    },
    transaction::{
        encrypted_payload::{DecryptedPayload, DecryptionFailureReason, EncryptedPayload},
        SignedTransaction,
    },
};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
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
        let result = Self::decrypt_encrypted_txns_inner(
            materialize_fut,
            block,
            author,
            is_decryption_enabled,
            maybe_secret_share_config,
            derived_self_key_share_tx,
            secret_shared_key_rx,
            observer_enabled,
            observer_decrypted_txns,
        )
        .await;
        match &result {
            Ok(res) => record_decryption_metrics(res),
            Err(e) => {
                warn!("decrypt_encrypted_txns failed: {:?}", e);
                counters::DECRYPTION_PIPELINE_TXNS_COUNT
                    .with_label_values(&["error"])
                    .inc();
            },
        }
        result
    }

    async fn decrypt_encrypted_txns_inner(
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

        // Single partition point: split encrypted from regular transactions once
        // so all downstream paths receive pre-partitioned vecs.
        let (encrypted_txns, regular_txns): (Vec<_>, Vec<_>) = input_txns
            .into_iter()
            .partition(|txn| txn.is_encrypted_txn());

        if !is_decryption_enabled {
            let _ = derived_self_key_share_tx.send(None);
            let failed_txns = mark_txns_failed_decryption(
                encrypted_txns,
                DecryptionFailureReason::ConfigUnavailable,
            );
            return Ok(DecryptionResult {
                decrypted_txns: failed_txns,
                regular_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
                decryption_key: None,
            });
        }

        let Some(secret_share_config) = maybe_secret_share_config else {
            let _ = derived_self_key_share_tx.send(None);

            // Consensus node without secret share config (e.g. bootstrapping
            // epoch where chunky DKG is newly enabled but hasn't completed yet).
            // Return immediately with no decryption key to avoid a circular
            // dependency: has_rand_txns_fut -> prepare -> decrypt (waiting for
            // secret_shared_key_rx) -> ordering (blocked on has_rand_txns_fut).
            if !observer_enabled {
                let failed_txns = mark_txns_failed_decryption(
                    encrypted_txns,
                    DecryptionFailureReason::ConfigUnavailable,
                );
                return Ok(DecryptionResult {
                    decrypted_txns: failed_txns,
                    regular_txns,
                    max_txns_from_block_to_execute,
                    block_gas_limit,
                    decryption_key: Some(None),
                });
            }

            return decrypt_observer_path(
                encrypted_txns,
                regular_txns,
                secret_shared_key_rx,
                observer_decrypted_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
            )
            .await;
        };

        decrypt_validator_path(
            encrypted_txns,
            regular_txns,
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
    _encrypted_txns: Vec<SignedTransaction>,
    regular_txns: Vec<SignedTransaction>,
    secret_shared_key_rx: oneshot::Receiver<Option<SecretSharedKey>>,
    observer_decrypted_txns: Option<Vec<SignedTransaction>>,
    max_txns_from_block_to_execute: Option<u64>,
    block_gas_limit: Option<u64>,
) -> TaskResult<DecryptionResult> {
    let maybe_key = secret_shared_key_rx
        .await
        .map_err(|_| anyhow!("secret_shared_key_rx dropped in observer path"))?;

    // When the key is None the validator may still send failed-decryption txns
    // (e.g. DecryptionKeyUnavailable) via V2. Accept them if present; only
    // return an empty result when neither key nor txns are available.
    let dec_key = maybe_key
        .map(|key| BlockTxnDecryptionKey::from_secret_shared_key(&key))
        .transpose()?;

    let decrypted_txns = observer_decrypted_txns.unwrap_or_default();

    Ok(DecryptionResult {
        decrypted_txns,
        regular_txns,
        max_txns_from_block_to_execute,
        block_gas_limit,
        decryption_key: Some(dec_key),
    })
}

/// Validator path: derive key share, prepare ciphertexts, await the shared
/// decryption key, and decrypt all encrypted transactions.
async fn decrypt_validator_path(
    encrypted_txns: Vec<SignedTransaction>,
    regular_txns: Vec<SignedTransaction>,
    block: &Block,
    author: Author,
    secret_share_config: &SecretShareConfig,
    derived_self_key_share_tx: oneshot::Sender<Option<SecretShare>>,
    secret_shared_key_rx: oneshot::Receiver<Option<SecretSharedKey>>,
    max_txns_from_block_to_execute: Option<u64>,
    block_gas_limit: Option<u64>,
) -> TaskResult<DecryptionResult> {
    // Short-circuit if no encrypted transactions: skip all crypto operations
    if encrypted_txns.is_empty() {
        let _ = derived_self_key_share_tx.send(None);
        return Ok(DecryptionResult {
            decrypted_txns: Vec::new(),
            regular_txns,
            max_txns_from_block_to_execute,
            block_gas_limit,
            decryption_key: Some(None),
        });
    }

    let max_encrypted_txns = secret_share_config.digest_key().max_batch_size();
    let (encrypted_txns, batch_limit_exceeded_txns) = if encrypted_txns.len() > max_encrypted_txns {
        warn!(
            "Block {} has {} encrypted txns exceeding batch limit {}; marking excess as BatchLimitReached",
            block.round(),
            encrypted_txns.len(),
            max_encrypted_txns,
        );
        let mut all = encrypted_txns;
        let exceeded = all.split_off(max_encrypted_txns);
        (all, exceeded)
    } else {
        (encrypted_txns, Vec::new())
    };

    // Mark batch-limit-exceeded txns as failed decryption with retry reason.
    let batch_limit_exceeded_txns = mark_txns_failed_decryption(
        batch_limit_exceeded_txns,
        DecryptionFailureReason::BatchLimitReached,
    );

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
    let num_rounds = secret_share_config.digest_key().num_rounds() as u64;
    let encryption_round = block.round() % num_rounds;
    let digest_key = secret_share_config.digest_key_arc();
    let (txn_ciphertexts, digest, proofs_promise) = tokio::task::spawn_blocking(move || {
        monitor!(
            "decryption_digest",
            FPTXWeighted::digest(&digest_key, &txn_ciphertexts, encryption_round)
                .map(|(digest, proofs_promise)| (txn_ciphertexts, digest, proofs_promise))
        )
    })
    .await
    .map_err(|e| anyhow!("digest computation panicked: {e}"))??;

    let metadata = SecretShareMetadata::new(
        block.epoch(),
        block.round(),
        block.timestamp_usecs(),
        block.id(),
        digest.clone(),
    );

    let derived_key_share = monitor!(
        "decryption_derive_key_share",
        FPTXWeighted::derive_decryption_key_share(secret_share_config.msk_share(), &digest)?
    );
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

    // eval_proofs is CPU-heavy (O(n^2 log n)); spawn_blocking prevents starving the async runtime.
    // Pipeline: eval_proofs ──→ prepare_ct ──┐
    //           secret_shared_key_rx ─────────┴──→ decrypt
    let digest_key = secret_share_config.digest_key_arc();
    let proofs = monitor!(
        "decryption_eval_proofs",
        tokio::task::spawn_blocking(move || {
            FPTXWeighted::eval_proofs_compute_all(&proofs_promise, &digest_key)
        })
        .await
        .map_err(|e| anyhow!("proof computation panicked: {e}"))?
    );

    // prepare_ct is expensive (parallel pairings) and doesn't need the decryption key,
    // so run it concurrently with the remaining key wait.
    let prepare_handle = {
        let digest = digest.clone();
        let proofs = proofs.clone();
        tokio::task::spawn_blocking(move || {
            monitor!(
                "decryption_prepare_ct",
                txn_ciphertexts
                    .into_par_iter()
                    .map(|ciphertext| {
                        let prepared_or_err =
                            FPTXWeighted::prepare_ct(&ciphertext, &digest, &proofs);
                        let id: Id = prepared_or_err
                            .as_ref()
                            .map_or_else(|MissingEvalProofError(id)| *id, |ct| ct.id());
                        (id, prepared_or_err)
                    })
                    .collect::<Vec<_>>()
            )
        })
    };

    let (prepared_cts, maybe_decryption_key) = monitor!(
        "decryption_wait_prepare_and_key",
        tokio::try_join!(
            async {
                prepare_handle
                    .await
                    .map_err(|e| anyhow!("prepare_ct panicked: {e}"))
            },
            async {
                secret_shared_key_rx
                    .await
                    .map_err(|_| anyhow!("secret_shared_key_rx dropped"))
            },
        )?
    );

    // Handle missing decryption key gracefully instead of panicking.
    // Mark all encrypted txns as failed-decryption so downstream can handle them.
    let Some(decryption_key) = maybe_decryption_key else {
        error!(
            "Decryption key unavailable for block {}; marking {} encrypted txns as failed",
            block.round(),
            encrypted_txns.len()
        );
        let failed_txns: Vec<_> = encrypted_txns
            .into_par_iter()
            .zip(prepared_cts.into_par_iter())
            .map(|(txn, (id, _prepared))| {
                let eval_proof = proofs.get(&id).expect("must exist");
                mark_txn_failed_decryption(
                    txn,
                    Some(eval_proof),
                    DecryptionFailureReason::DecryptionKeyUnavailable,
                )
            })
            .collect();
        let decrypted_txns = [failed_txns, batch_limit_exceeded_txns].concat();
        return Ok(DecryptionResult {
            decrypted_txns,
            regular_txns,
            max_txns_from_block_to_execute,
            block_gas_limit,
            decryption_key: Some(None),
        });
    };

    info!(
        "Successfully received decryption key for block {}: metadata={:?}",
        block.round(),
        decryption_key.metadata
    );

    // Final decryption pass — needs both prepared ciphertexts and the decryption key.
    let num_failed_decryptions = AtomicUsize::new(0);
    let decrypted_txns: Vec<_> = monitor!(
        "decryption_decrypt",
        encrypted_txns
            .into_par_iter()
            .zip(prepared_cts.into_par_iter())
            .map(|(mut txn, (id, prepared_ciphertext_or_error))| {
                let eval_proof = proofs.get(&id).expect("must exist");

                match do_final_decryption(&decryption_key.key, prepared_ciphertext_or_error) {
                    Ok(payload) => {
                        let encrypted_payload = txn.payload_mut()
                            .as_encrypted_payload_mut()
                            .expect("must happen");
                        if !encrypted_payload.entry_fun_matches(&payload)
                            .expect("must be encrypted") {
                            warn!(
                                "transaction with ciphertext id {:?} has mismatching entry function",
                                id
                            );
                            num_failed_decryptions.fetch_add(1, Ordering::Relaxed);
                            mark_txn_failed_decryption(
                                txn,
                                Some(eval_proof),
                                DecryptionFailureReason::ClaimedEntryFunctionMismatch,
                            )
                        } else {
                            let (executable, nonce) = payload.unwrap();
                            encrypted_payload.into_decrypted(eval_proof, executable, nonce)
                                .expect("must happen");
                            txn
                        }
                    },
                    Err(e) => {
                        error!(
                            "Failed to decrypt transaction with ciphertext id {:?}: {:?}",
                            id, e
                        );
                        num_failed_decryptions.fetch_add(1, Ordering::Relaxed);
                        mark_txn_failed_decryption(
                            txn,
                            Some(eval_proof),
                            DecryptionFailureReason::CryptoFailure,
                        )
                    },
                }
            })
            .collect()
    );

    let num_failed = num_failed_decryptions.into_inner();
    let num_decrypted = decrypted_txns.len() - num_failed;
    info!(
        "Decryption complete for block {}: {} decrypted, {} failed, {} batch_limit_exceeded, {} unencrypted",
        block.round(),
        num_decrypted,
        num_failed,
        batch_limit_exceeded_txns.len(),
        regular_txns.len(),
    );
    let decrypted_txns = [decrypted_txns, batch_limit_exceeded_txns].concat();

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

fn record_decryption_metrics(result: &DecryptionResult) {
    let mut decrypted = 0u64;
    let mut failed_decryption = 0u64;
    let mut config_unavailable = 0u64;
    let mut key_unavailable = 0u64;
    let mut batch_limit_exceeded = 0u64;
    let mut entry_fun_mismatch = 0u64;

    for txn in &result.decrypted_txns {
        let Some(ep) = txn.payload().as_encrypted_payload() else {
            continue;
        };
        match ep {
            EncryptedPayload::Decrypted { .. } => decrypted += 1,
            EncryptedPayload::FailedDecryption { reason, .. } => match reason {
                DecryptionFailureReason::CryptoFailure => failed_decryption += 1,
                DecryptionFailureReason::BatchLimitReached => batch_limit_exceeded += 1,
                DecryptionFailureReason::ConfigUnavailable => config_unavailable += 1,
                DecryptionFailureReason::DecryptionKeyUnavailable => key_unavailable += 1,
                DecryptionFailureReason::ClaimedEntryFunctionMismatch => entry_fun_mismatch += 1,
            },
            EncryptedPayload::Encrypted { .. } => {
                // Still in encrypted state — shouldn't happen after decryption pipeline
                failed_decryption += 1;
            },
        }
    }

    let unencrypted = result.regular_txns.len() as u64;

    let pairs = [
        ("decrypted", decrypted),
        ("failed_decryption", failed_decryption),
        ("config_unavailable", config_unavailable),
        ("key_unavailable", key_unavailable),
        ("batch_limit_exceeded", batch_limit_exceeded),
        ("entry_fun_mismatch", entry_fun_mismatch),
        ("unencrypted", unencrypted),
    ];
    for (label, count) in pairs {
        counters::DECRYPTION_PIPELINE_TXNS_COUNT
            .with_label_values(&[label])
            .inc_by(count);
    }
}

fn mark_txns_failed_decryption(
    txns: Vec<SignedTransaction>,
    reason: DecryptionFailureReason,
) -> Vec<SignedTransaction> {
    txns.into_iter()
        .map(|txn| mark_txn_failed_decryption(txn, None, reason.clone()))
        .collect()
}

fn mark_txn_failed_decryption(
    mut txn: SignedTransaction,
    eval_proof: Option<EvalProof>,
    reason: DecryptionFailureReason,
) -> SignedTransaction {
    txn.payload_mut()
        .as_encrypted_payload_mut()
        .map(|p| {
            p.into_failed_decryption_with_reason(eval_proof, reason)
                .expect("must be in Encrypted state")
        })
        .expect("must be encrypted txn");
    txn
}

fn do_final_decryption(
    decryption_key: &DecryptionKey,
    prepared_ciphertext_or_error: Result<PreparedCiphertext, MissingEvalProofError>,
) -> anyhow::Result<DecryptedPayload> {
    FPTXWeighted::decrypt(decryption_key, &prepared_ciphertext_or_error?)
}
