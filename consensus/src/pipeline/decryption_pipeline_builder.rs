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
        encrypted_payload::{DecryptedPlaintext, DecryptionFailureReason, EncryptedPayload},
        SignedTransaction,
    },
    validator_txn::ValidatorTransaction,
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

fn block_has_epoch_end_vtxn(block: &Block) -> bool {
    block.validator_txns().is_some_and(|vtxns| {
        vtxns.iter().any(|v| {
            matches!(
                v,
                ValidatorTransaction::DKGResult(_) | ValidatorTransaction::ChunkyDKGResult(_)
            )
        })
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

    // Decrypting txns that the VM will skip after a NewEpochEvent leaks sender
    // intent. If the block carries an epoch-ending vtxn, mark every encrypted
    // txn as retry without performing any crypto work.
    if block_has_epoch_end_vtxn(block) {
        info!(
            "Block {} has epoch-ending vtxn; marking {} encrypted txns as EpochEndRetry",
            block.round(),
            encrypted_txns.len()
        );
        let _ = derived_self_key_share_tx.send(None);
        let failed_txns =
            mark_txns_failed_decryption(encrypted_txns, DecryptionFailureReason::EpochEndRetry);
        return Ok(DecryptionResult {
            decrypted_txns: failed_txns,
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
                let payload_encryption_epoch = txn
                    .payload()
                    .as_encrypted_payload()
                    .expect("must happen")
                    .encryption_epoch();

                if payload_encryption_epoch != decryption_key.metadata.epoch {
                    warn!(
                        "transaction with ciphertext id {:?} has encryption epoch {} but decryption key epoch {}",
                        id,
                        payload_encryption_epoch,
                        decryption_key.metadata.epoch,
                    );
                    num_failed_decryptions.fetch_add(1, Ordering::Relaxed);
                    return mark_txn_failed_decryption(
                        txn,
                        Some(eval_proof),
                        DecryptionFailureReason::EpochMismatch,
                    );
                }

                match do_final_decryption(&decryption_key.key, prepared_ciphertext_or_error) {
                    Ok(payload) => {
                        let encrypted_payload = txn.payload_mut()
                            .as_encrypted_payload_mut()
                            .expect("must happen");
                        match encrypted_payload
                            .try_into_decrypted(eval_proof.clone(), payload)
                        {
                            Ok(()) => txn,
                            Err(reason) => {
                                warn!(
                                    "transaction with ciphertext id {:?} rejected post-decryption: {:?}",
                                    id, reason
                                );
                                num_failed_decryptions.fetch_add(1, Ordering::Relaxed);
                                mark_txn_failed_decryption(txn, Some(eval_proof), reason)
                            },
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
    let mut payload_hash_mismatch = 0u64;
    let mut epoch_mismatch = 0u64;
    let mut epoch_end_retry = 0u64;
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
                DecryptionFailureReason::PayloadHashMismatch => payload_hash_mismatch += 1,
                DecryptionFailureReason::EpochMismatch => epoch_mismatch += 1,
                DecryptionFailureReason::EpochEndRetry => epoch_end_retry += 1,
                DecryptionFailureReason::ClaimedEntryFunctionMismatch => entry_fun_mismatch += 1,
            },
            EncryptedPayload::Encrypted(_) => {
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
        ("payload_hash_mismatch", payload_hash_mismatch),
        ("epoch_mismatch", epoch_mismatch),
        ("epoch_end_retry", epoch_end_retry),
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
) -> anyhow::Result<DecryptedPlaintext> {
    FPTXWeighted::decrypt(decryption_key, &prepared_ciphertext_or_error?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rand::secret_sharing::test_utils::TestContext;
    use aptos_consensus_types::{
        block::block_test_utils::certificate_for_genesis, block_data::BlockData, common::Payload,
        pipelined_block::TaskError,
    };
    use aptos_crypto::{
        ed25519::Ed25519PrivateKey,
        hash::{CryptoHash, HashValue},
        PrivateKey, SigningKey, Uniform,
    };
    use aptos_types::{
        aggregate_signature::AggregateSignature,
        chain_id::ChainId,
        dkg::{
            chunky_dkg::{CertifiedAggregatedChunkySubtranscript, CertifiedChunkyDKGOutput},
            DKGTranscriptMetadata,
        },
        jwks::QuorumCertifiedUpdate,
        secret_sharing::{Ciphertext, EvalProof},
        transaction::{
            encrypted_payload::EncryptedInner, EntryFunction, RawTransaction, Script,
            TransactionExecutable, TransactionExtraConfig, TransactionPayload,
        },
        validator_txn::ValidatorTransaction,
    };
    use futures::FutureExt;
    use move_core_types::{account_address::AccountAddress, ident_str, language_storage::ModuleId};
    use rand::thread_rng;

    // ---------- Test helpers ----------

    fn sign_payload(payload: TransactionPayload) -> SignedTransaction {
        let raw = RawTransaction::new(
            AccountAddress::random(),
            0,
            payload,
            0,
            0,
            0,
            ChainId::new(10),
        );
        let private_key = Ed25519PrivateKey::generate(&mut thread_rng());
        let public_key = private_key.public_key();
        SignedTransaction::new(raw.clone(), public_key, private_key.sign(&raw).unwrap())
    }

    fn encrypted_inner() -> EncryptedInner {
        EncryptedInner {
            ciphertext: Ciphertext::random(),
            extra_config: TransactionExtraConfig::V1 {
                multisig_address: None,
                replay_protection_nonce: None,
            },
            payload_hash: HashValue::random(),
            encryption_epoch: 1,
            claimed_entry_fun: None,
        }
    }

    fn make_encrypted_txn() -> SignedTransaction {
        sign_payload(TransactionPayload::EncryptedPayload(
            EncryptedPayload::Encrypted(encrypted_inner()),
        ))
    }

    fn make_failed_decryption_txn(reason: DecryptionFailureReason) -> SignedTransaction {
        sign_payload(TransactionPayload::EncryptedPayload(
            EncryptedPayload::FailedDecryption {
                original: encrypted_inner(),
                eval_proof: Some(EvalProof::random()),
                reason,
            },
        ))
    }

    fn make_decrypted_txn() -> SignedTransaction {
        let executable = TransactionExecutable::EntryFunction(EntryFunction::new(
            ModuleId::new(AccountAddress::ONE, ident_str!("coin").to_owned()),
            ident_str!("transfer").to_owned(),
            vec![],
            vec![],
        ));
        let plaintext = DecryptedPlaintext::new(executable, [0; 16]);
        let mut original = encrypted_inner();
        original.payload_hash = CryptoHash::hash(&plaintext);
        sign_payload(TransactionPayload::EncryptedPayload(
            EncryptedPayload::Decrypted {
                original,
                eval_proof: EvalProof::random(),
                decrypted: plaintext,
            },
        ))
    }

    fn make_regular_txn() -> SignedTransaction {
        sign_payload(TransactionPayload::Script(Script::new(
            vec![],
            vec![],
            vec![],
        )))
    }

    fn make_block(vtxns: Option<Vec<ValidatorTransaction>>) -> Arc<Block> {
        let qc = certificate_for_genesis();
        let block_data = match vtxns {
            Some(vtxns) => BlockData::new_proposal_ext(
                vtxns,
                Payload::empty(false),
                AccountAddress::random(),
                Vec::new(),
                1,
                1,
                qc,
            ),
            None => BlockData::new_proposal(
                Payload::empty(false),
                AccountAddress::random(),
                Vec::new(),
                1,
                1,
                qc,
            ),
        };
        let id = block_data.hash();
        Arc::new(Block::new_for_testing(id, block_data, None))
    }

    fn dkg_vtxn() -> ValidatorTransaction {
        ValidatorTransaction::dummy(b"dkg".to_vec())
    }

    fn chunky_dkg_vtxn() -> ValidatorTransaction {
        ValidatorTransaction::ChunkyDKGResult(CertifiedChunkyDKGOutput {
            certified_transcript: CertifiedAggregatedChunkySubtranscript {
                metadata: DKGTranscriptMetadata {
                    epoch: 0,
                    author: AccountAddress::ZERO,
                },
                transcript_bytes: vec![],
                signature: AggregateSignature::empty(),
            },
            encryption_key: vec![],
        })
    }

    fn jwk_vtxn() -> ValidatorTransaction {
        ValidatorTransaction::ObservedJWKUpdate(QuorumCertifiedUpdate::dummy())
    }

    fn materialize_ok(txns: Vec<SignedTransaction>) -> TaskFuture<MaterializeResult> {
        async move { Ok((txns, None, None)) }.boxed().shared()
    }

    fn materialize_err() -> TaskFuture<MaterializeResult> {
        async {
            Err(TaskError::InternalError(Arc::new(anyhow!(
                "materialize failed"
            ))))
        }
        .boxed()
        .shared()
    }

    fn assert_failed_with(txn: &SignedTransaction, expected: &DecryptionFailureReason) {
        match txn
            .payload()
            .as_encrypted_payload()
            .expect("must be encrypted payload")
        {
            EncryptedPayload::FailedDecryption { reason, .. } => assert_eq!(reason, expected),
            other => panic!("expected FailedDecryption, got {:?}", other),
        }
    }

    // ---------- Helper functions and metrics ----------

    #[test]
    fn block_has_epoch_end_vtxn_returns_true_for_dkg_result() {
        let block = make_block(Some(vec![dkg_vtxn()]));
        assert!(block_has_epoch_end_vtxn(&block));
    }

    #[test]
    fn block_has_epoch_end_vtxn_returns_true_for_chunky_dkg_result() {
        let block = make_block(Some(vec![chunky_dkg_vtxn()]));
        assert!(block_has_epoch_end_vtxn(&block));
    }

    #[test]
    fn block_has_epoch_end_vtxn_returns_false_for_jwk_update_only() {
        let block = make_block(Some(vec![jwk_vtxn()]));
        assert!(!block_has_epoch_end_vtxn(&block));
    }

    #[test]
    fn block_has_epoch_end_vtxn_returns_false_for_no_vtxns() {
        let block = make_block(None);
        assert!(!block_has_epoch_end_vtxn(&block));
    }

    #[test]
    fn block_has_epoch_end_vtxn_returns_false_for_empty_vtxns() {
        let block = make_block(Some(vec![]));
        assert!(!block_has_epoch_end_vtxn(&block));
    }

    #[test]
    fn block_has_epoch_end_vtxn_returns_true_for_mixed() {
        let block = make_block(Some(vec![jwk_vtxn(), dkg_vtxn()]));
        assert!(block_has_epoch_end_vtxn(&block));
    }

    #[test]
    fn mark_txn_failed_decryption_transitions_state() {
        let txn = make_encrypted_txn();
        let proof = EvalProof::random();
        let marked = mark_txn_failed_decryption(
            txn,
            Some(proof.clone()),
            DecryptionFailureReason::CryptoFailure,
        );
        match marked
            .payload()
            .as_encrypted_payload()
            .expect("must be encrypted")
        {
            EncryptedPayload::FailedDecryption {
                eval_proof, reason, ..
            } => {
                assert_eq!(*reason, DecryptionFailureReason::CryptoFailure);
                assert_eq!(eval_proof.as_ref(), Some(&proof));
            },
            other => panic!("expected FailedDecryption, got {:?}", other),
        }
    }

    #[test]
    fn mark_txns_failed_decryption_marks_all() {
        let txns: Vec<_> = (0..3).map(|_| make_encrypted_txn()).collect();
        let marked = mark_txns_failed_decryption(txns, DecryptionFailureReason::EpochEndRetry);
        assert_eq!(marked.len(), 3);
        for txn in &marked {
            assert_failed_with(txn, &DecryptionFailureReason::EpochEndRetry);
            // No eval_proof when bulk-marking before crypto.
            match txn.payload().as_encrypted_payload().unwrap() {
                EncryptedPayload::FailedDecryption { eval_proof, .. } => {
                    assert!(eval_proof.is_none());
                },
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn record_decryption_metrics_counts_every_variant_exhaustively() {
        // Snapshot every label so we can assert deltas (the counter is global).
        let labels = [
            "decrypted",
            "failed_decryption",
            "config_unavailable",
            "key_unavailable",
            "batch_limit_exceeded",
            "payload_hash_mismatch",
            "epoch_mismatch",
            "epoch_end_retry",
            "entry_fun_mismatch",
            "unencrypted",
        ];
        let before: Vec<u64> = labels
            .iter()
            .map(|l| {
                counters::DECRYPTION_PIPELINE_TXNS_COUNT
                    .with_label_values(&[l])
                    .get()
            })
            .collect();

        // One txn per FailedDecryption variant + one Decrypted + two regular.
        let result = DecryptionResult {
            decrypted_txns: vec![
                make_decrypted_txn(),
                make_failed_decryption_txn(DecryptionFailureReason::CryptoFailure),
                make_failed_decryption_txn(DecryptionFailureReason::BatchLimitReached),
                make_failed_decryption_txn(DecryptionFailureReason::ConfigUnavailable),
                make_failed_decryption_txn(DecryptionFailureReason::DecryptionKeyUnavailable),
                make_failed_decryption_txn(DecryptionFailureReason::PayloadHashMismatch),
                make_failed_decryption_txn(DecryptionFailureReason::EpochMismatch),
                make_failed_decryption_txn(DecryptionFailureReason::EpochEndRetry),
                make_failed_decryption_txn(DecryptionFailureReason::ClaimedEntryFunctionMismatch),
            ],
            regular_txns: vec![make_regular_txn(), make_regular_txn()],
            max_txns_from_block_to_execute: None,
            block_gas_limit: None,
            decryption_key: Some(None),
        };
        record_decryption_metrics(&result);

        let after: Vec<u64> = labels
            .iter()
            .map(|l| {
                counters::DECRYPTION_PIPELINE_TXNS_COUNT
                    .with_label_values(&[l])
                    .get()
            })
            .collect();
        let deltas: Vec<u64> = before
            .iter()
            .zip(after.iter())
            .map(|(b, a)| a - b)
            .collect();
        let expected = [1, 1, 1, 1, 1, 1, 1, 1, 1, 2];
        assert_eq!(deltas, expected, "labels: {:?}", labels);
    }

    // ---------- decrypt_encrypted_txns_inner routing ----------

    #[tokio::test]
    async fn routing_disabled_marks_config_unavailable() {
        let encrypted = vec![make_encrypted_txn(), make_encrypted_txn()];
        let regular = vec![make_regular_txn()];
        let mut input = encrypted.clone();
        input.extend(regular.clone());

        let (key_share_tx, key_share_rx) = oneshot::channel();
        let (_skey_tx, skey_rx) = oneshot::channel();

        let result = PipelineBuilder::decrypt_encrypted_txns_inner(
            materialize_ok(input),
            make_block(None),
            AccountAddress::random(),
            false, // is_decryption_enabled
            None,
            key_share_tx,
            skey_rx,
            false,
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(result.decrypted_txns.len(), 2);
        for txn in &result.decrypted_txns {
            assert_failed_with(txn, &DecryptionFailureReason::ConfigUnavailable);
        }
        assert_eq!(result.regular_txns.len(), 1);
        assert_eq!(result.decryption_key, None);
        assert!(key_share_rx.await.unwrap().is_none());
    }

    #[tokio::test]
    async fn routing_no_config_no_observer_marks_config_unavailable() {
        let encrypted = vec![make_encrypted_txn()];
        let regular = vec![make_regular_txn()];
        let mut input = encrypted.clone();
        input.extend(regular.clone());

        let (key_share_tx, key_share_rx) = oneshot::channel();
        let (_skey_tx, skey_rx) = oneshot::channel();

        let result = PipelineBuilder::decrypt_encrypted_txns_inner(
            materialize_ok(input),
            make_block(None),
            AccountAddress::random(),
            true,
            None, // maybe_secret_share_config
            key_share_tx,
            skey_rx,
            false, // observer_enabled
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(result.decrypted_txns.len(), 1);
        assert_failed_with(
            &result.decrypted_txns[0],
            &DecryptionFailureReason::ConfigUnavailable,
        );
        assert_eq!(result.regular_txns.len(), 1);
        assert_eq!(result.decryption_key, Some(None));
        assert!(key_share_rx.await.unwrap().is_none());
    }

    #[tokio::test]
    async fn routing_no_config_with_observer_uses_observer_path() {
        // Observer path: encrypted txns are NOT marked here (the observer
        // consumes pre-decrypted txns provided by the validator).
        let encrypted = vec![make_encrypted_txn()];
        let observer_decrypted = vec![make_decrypted_txn()];
        let mut input = encrypted.clone();
        input.push(make_regular_txn());

        let (key_share_tx, _key_share_rx) = oneshot::channel();
        let (skey_tx, skey_rx) = oneshot::channel();
        skey_tx.send(None).unwrap();

        let result = PipelineBuilder::decrypt_encrypted_txns_inner(
            materialize_ok(input),
            make_block(None),
            AccountAddress::random(),
            true,
            None,
            key_share_tx,
            skey_rx,
            true, // observer_enabled
            Some(observer_decrypted.clone()),
        )
        .await
        .expect("should succeed");

        // Observer-path output: decrypted_txns mirror the provided observer txns
        // (Decrypted state, not FailedDecryption).
        assert_eq!(result.decrypted_txns.len(), 1);
        assert!(matches!(
            result.decrypted_txns[0]
                .payload()
                .as_encrypted_payload()
                .unwrap(),
            EncryptedPayload::Decrypted { .. }
        ));
        assert_eq!(result.decryption_key, Some(None));
    }

    #[tokio::test]
    async fn routing_propagates_materialize_fut_error() {
        let (key_share_tx, _key_share_rx) = oneshot::channel();
        let (_skey_tx, skey_rx) = oneshot::channel();

        let err = PipelineBuilder::decrypt_encrypted_txns_inner(
            materialize_err(),
            make_block(None),
            AccountAddress::random(),
            true,
            None,
            key_share_tx,
            skey_rx,
            false,
            None,
        )
        .await
        .expect_err("should propagate error");

        assert!(format!("{}", err).contains("materialize failed"));
    }

    // ---------- decrypt_observer_path ----------

    #[tokio::test]
    async fn observer_with_key_and_decrypted_txns_passes_through() {
        let ctx = TestContext::new(vec![100]);
        let metadata = crate::rand::secret_sharing::test_utils::create_metadata(1, 1);
        let key =
            crate::rand::secret_sharing::test_utils::create_secret_shared_key(&ctx, &metadata);

        let regular = vec![make_regular_txn()];
        let observer_decrypted = vec![make_decrypted_txn(), make_decrypted_txn()];

        let (skey_tx, skey_rx) = oneshot::channel();
        skey_tx.send(Some(key)).unwrap();

        let result = decrypt_observer_path(
            vec![],
            regular.clone(),
            skey_rx,
            Some(observer_decrypted.clone()),
            None,
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(result.decrypted_txns.len(), 2);
        assert!(matches!(result.decryption_key, Some(Some(_))));
        assert_eq!(result.regular_txns.len(), 1);
    }

    #[tokio::test]
    async fn observer_with_no_key_but_decrypted_txns_returns_some_none() {
        let observer_decrypted = vec![make_decrypted_txn()];
        let (skey_tx, skey_rx) = oneshot::channel();
        skey_tx.send(None).unwrap();

        let result = decrypt_observer_path(
            vec![],
            vec![],
            skey_rx,
            Some(observer_decrypted.clone()),
            None,
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(result.decrypted_txns.len(), 1);
        assert_eq!(result.decryption_key, Some(None));
    }

    #[tokio::test]
    async fn observer_with_no_key_no_txns_returns_empty() {
        let (skey_tx, skey_rx) = oneshot::channel();
        skey_tx.send(None).unwrap();

        let result = decrypt_observer_path(vec![], vec![], skey_rx, None, None, None)
            .await
            .expect("should succeed");

        assert!(result.decrypted_txns.is_empty());
        assert_eq!(result.decryption_key, Some(None));
    }

    #[tokio::test]
    async fn observer_with_dropped_rx_returns_error() {
        let (skey_tx, skey_rx) = oneshot::channel::<Option<SecretSharedKey>>();
        drop(skey_tx);

        let err = decrypt_observer_path(vec![], vec![], skey_rx, None, None, None)
            .await
            .expect_err("should error");
        assert!(format!("{}", err).contains("secret_shared_key_rx dropped"));
    }

    #[tokio::test]
    async fn observer_preserves_regular_txns() {
        let regular = vec![make_regular_txn(), make_regular_txn(), make_regular_txn()];
        let (skey_tx, skey_rx) = oneshot::channel();
        skey_tx.send(None).unwrap();

        let result = decrypt_observer_path(vec![], regular.clone(), skey_rx, None, None, None)
            .await
            .expect("should succeed");

        assert_eq!(result.regular_txns.len(), regular.len());
    }

    // ---------- decrypt_validator_path (pre-crypto branches) ----------

    #[tokio::test]
    async fn validator_path_empty_encrypted_txns_short_circuits() {
        let ctx = TestContext::new(vec![100]);
        let block = make_block(None);
        let regular = vec![make_regular_txn()];

        let (key_share_tx, key_share_rx) = oneshot::channel();
        let (_skey_tx, skey_rx) = oneshot::channel();

        let result = decrypt_validator_path(
            vec![], // empty encrypted_txns
            regular.clone(),
            &block,
            ctx.authors[0],
            &ctx.secret_share_config,
            key_share_tx,
            skey_rx,
            None,
            None,
        )
        .await
        .expect("should succeed");

        assert!(result.decrypted_txns.is_empty());
        assert_eq!(result.regular_txns.len(), 1);
        assert_eq!(result.decryption_key, Some(None));
        assert!(key_share_rx.await.unwrap().is_none());
    }

    #[tokio::test]
    async fn validator_path_dkg_result_vtxn_marks_all_epoch_end_retry() {
        let ctx = TestContext::new(vec![100]);
        let block = make_block(Some(vec![dkg_vtxn()]));
        let encrypted = vec![make_encrypted_txn(), make_encrypted_txn()];
        let regular = vec![make_regular_txn()];

        let (key_share_tx, key_share_rx) = oneshot::channel();
        let (_skey_tx, skey_rx) = oneshot::channel();

        let result = decrypt_validator_path(
            encrypted.clone(),
            regular.clone(),
            &block,
            ctx.authors[0],
            &ctx.secret_share_config,
            key_share_tx,
            skey_rx,
            None,
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(result.decrypted_txns.len(), 2);
        for txn in &result.decrypted_txns {
            assert_failed_with(txn, &DecryptionFailureReason::EpochEndRetry);
        }
        assert_eq!(result.regular_txns.len(), 1);
        assert_eq!(result.decryption_key, Some(None));
        assert!(key_share_rx.await.unwrap().is_none());
    }

    #[tokio::test]
    async fn validator_path_chunky_dkg_result_vtxn_marks_all_epoch_end_retry() {
        let ctx = TestContext::new(vec![100]);
        let block = make_block(Some(vec![chunky_dkg_vtxn()]));
        let encrypted = vec![make_encrypted_txn()];

        let (key_share_tx, key_share_rx) = oneshot::channel();
        let (_skey_tx, skey_rx) = oneshot::channel();

        let result = decrypt_validator_path(
            encrypted,
            vec![],
            &block,
            ctx.authors[0],
            &ctx.secret_share_config,
            key_share_tx,
            skey_rx,
            None,
            None,
        )
        .await
        .expect("should succeed");

        assert_eq!(result.decrypted_txns.len(), 1);
        assert_failed_with(
            &result.decrypted_txns[0],
            &DecryptionFailureReason::EpochEndRetry,
        );
        assert_eq!(result.decryption_key, Some(None));
        assert!(key_share_rx.await.unwrap().is_none());
    }
}
