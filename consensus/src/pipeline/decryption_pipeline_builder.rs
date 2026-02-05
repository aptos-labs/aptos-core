// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::pipeline::pipeline_builder::{PipelineBuilder, Tracker};
use anyhow::Context;
use aptos_batch_encryption::{
    schemes::fptx_weighted::FPTXWeighted, traits::BatchThresholdEncryption,
};
use aptos_consensus_types::{
    block::Block,
    common::Author,
    pipelined_block::{DecryptionResult, MaterializeResult, TaskFuture, TaskResult},
};
use aptos_types::{
    decryption::{BlockTxnDecryptionKey, DecKeyMetadata},
    secret_sharing::{
        Ciphertext, DigestKey, MasterSecretKeyShare, SecretShare, SecretShareConfig,
        SecretShareMetadata, SecretSharedKey,
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
        secret_share_config: Option<SecretShareConfig>,
        derived_self_key_share_tx: oneshot::Sender<Option<SecretShare>>,
        secret_shared_key_rx: oneshot::Receiver<Option<SecretSharedKey>>,
    ) -> TaskResult<DecryptionResult> {
        let mut tracker = Tracker::start_waiting("decrypt_encrypted_txns", &block);
        let (input_txns, max_txns_from_block_to_execute, block_gas_limit) = materialize_fut.await?;

        tracker.start_working();

        if secret_share_config.is_none() {
            return Ok((
                input_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
                None,
            ));
        }

        let (encrypted_txns, unencrypted_txns): (Vec<_>, Vec<_>) = input_txns
            .into_iter()
            .partition(|txn| txn.is_encrypted_txn());

        // TODO: figure out handling of
        if encrypted_txns.is_empty() {
            return Ok((
                unencrypted_txns,
                max_txns_from_block_to_execute,
                block_gas_limit,
                Some(None),
            ));
        }

        let digest_key: DigestKey = secret_share_config
            .as_ref()
            .expect("must exist")
            .digest_key()
            .clone();
        let msk_share: MasterSecretKeyShare = secret_share_config
            .as_ref()
            .expect("must exist")
            .msk_share()
            .clone();

        // TODO(ibalajiarun): FIXME
        let len = 10;
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
        let encryption_round = block.round();
        let (digest, proofs_promise) =
            FPTXWeighted::digest(&digest_key, &txn_ciphertexts, encryption_round)?;

        let metadata = SecretShareMetadata::new(
            block.epoch(),
            block.round(),
            block.timestamp_usecs(),
            block.id(),
            digest.clone(),
        );

        let derived_key_share = FPTXWeighted::derive_decryption_key_share(&msk_share, &digest)?;
        derived_self_key_share_tx
            .send(Some(SecretShare::new(
                author,
                metadata.clone(),
                derived_key_share,
            )))
            .expect("must send properly");

        // TODO(ibalajiarun): improve perf
        let proofs = FPTXWeighted::eval_proofs_compute_all(&proofs_promise, &digest_key);

        let maybe_decryption_key = secret_shared_key_rx
            .await
            .expect("decryption key should be available");
        // TODO(ibalajiarun): account for the case where decryption key is not available
        let decryption_key = maybe_decryption_key.expect("decryption key should be available");

        let decrypted_txns = encrypted_txns
            .into_par_iter()
            .zip(txn_ciphertexts)
            .map(|(mut txn, ciphertext)| {
                let eval_proof = proofs.get(&ciphertext.id()).expect("must exist");
                if let Ok(payload) = FPTXWeighted::decrypt_individual::<DecryptedPayload>(
                    &decryption_key.key,
                    &ciphertext,
                    &digest,
                    &eval_proof,
                ) {
                    let (executable, nonce) = payload.unwrap();
                    txn.payload_mut()
                        .as_encrypted_payload_mut()
                        .map(|p| {
                            p.into_decrypted(eval_proof, executable, nonce)
                                .expect("must happen")
                        })
                        .expect("must exist");
                } else {
                    txn.payload_mut()
                        .as_encrypted_payload_mut()
                        .map(|p| p.into_failed_decryption(eval_proof).expect("must happen"))
                        .expect("must exist");
                }
                txn
            })
            .collect();

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
