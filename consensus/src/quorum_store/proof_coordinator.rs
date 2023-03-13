// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    monitor,
    network::QuorumStoreSender,
    quorum_store::{
        batch_generator::BatchGeneratorCommand, batch_store::BatchReader, counters, utils::Timeouts,
    },
};
use aptos_consensus_types::proof_of_store::{
    ProofOfStore, SignedDigest, SignedDigestError, SignedDigestInfo,
};
use aptos_crypto::{bls12381, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    aggregate_signature::PartialSignatures, validator_verifier::ValidatorVerifier, PeerId,
};
use std::{
    collections::{hash_map::Entry, BTreeMap, HashMap},
    sync::Arc,
    time::Duration,
};
use tokio::{
    sync::{mpsc::Receiver, oneshot as TokioOneshot},
    time,
};

#[derive(Debug)]
pub(crate) enum ProofCoordinatorCommand {
    AppendSignature(SignedDigest),
    Shutdown(TokioOneshot::Sender<()>),
}

struct IncrementalProofState {
    info: SignedDigestInfo,
    aggregated_signature: BTreeMap<PeerId, bls12381::Signature>,
}

impl IncrementalProofState {
    fn new(info: SignedDigestInfo) -> Self {
        Self {
            info,
            aggregated_signature: BTreeMap::new(),
        }
    }

    fn add_signature(&mut self, signed_digest: SignedDigest) -> Result<(), SignedDigestError> {
        if signed_digest.info() != &self.info {
            return Err(SignedDigestError::WrongInfo);
        }

        if self
            .aggregated_signature
            .contains_key(&signed_digest.signer())
        {
            return Err(SignedDigestError::DuplicatedSignature);
        }

        self.aggregated_signature
            .insert(signed_digest.signer(), signed_digest.signature());
        Ok(())
    }

    fn ready(&self, validator_verifier: &ValidatorVerifier) -> bool {
        validator_verifier
            .check_voting_power(self.aggregated_signature.keys())
            .is_ok()
    }

    fn take(self, validator_verifier: &ValidatorVerifier) -> ProofOfStore {
        let proof = match validator_verifier
            .aggregate_signatures(&PartialSignatures::new(self.aggregated_signature))
        {
            Ok(sig) => ProofOfStore::new(self.info, sig),
            Err(e) => unreachable!("Cannot aggregate signatures on digest err = {:?}", e),
        };
        proof
    }
}

pub(crate) struct ProofCoordinator {
    peer_id: PeerId,
    proof_timeout_ms: usize,
    digest_to_proof: HashMap<HashValue, IncrementalProofState>,
    digest_to_time: HashMap<HashValue, u64>,
    // to record the batch creation time
    timeouts: Timeouts<SignedDigestInfo>,
    batch_reader: Arc<dyn BatchReader>,
    batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
}

//PoQS builder object - gather signed digest to form PoQS
impl ProofCoordinator {
    pub fn new(
        proof_timeout_ms: usize,
        peer_id: PeerId,
        batch_reader: Arc<dyn BatchReader>,
        batch_generator_cmd_tx: tokio::sync::mpsc::Sender<BatchGeneratorCommand>,
    ) -> Self {
        Self {
            peer_id,
            proof_timeout_ms,
            digest_to_proof: HashMap::new(),
            digest_to_time: HashMap::new(),
            timeouts: Timeouts::new(),
            batch_reader,
            batch_generator_cmd_tx,
        }
    }

    fn init_proof(&mut self, signed_digest: &SignedDigest) -> Result<(), SignedDigestError> {
        // Check if the signed digest corresponding to our batch
        if signed_digest.info().batch_author != self.peer_id {
            return Err(SignedDigestError::WrongAuthor);
        }
        let batch_author = self
            .batch_reader
            .exists(&signed_digest.digest())
            .ok_or(SignedDigestError::WrongAuthor)?;
        if batch_author != signed_digest.info().batch_author {
            return Err(SignedDigestError::WrongAuthor);
        }

        self.timeouts
            .add(signed_digest.info().clone(), self.proof_timeout_ms);
        self.digest_to_proof.insert(
            signed_digest.digest(),
            IncrementalProofState::new(signed_digest.info().clone()),
        );
        self.digest_to_time
            .entry(signed_digest.digest())
            .or_insert(chrono::Utc::now().naive_utc().timestamp_micros() as u64);
        Ok(())
    }

    fn add_signature(
        &mut self,
        signed_digest: SignedDigest,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<Option<ProofOfStore>, SignedDigestError> {
        if !self.digest_to_proof.contains_key(&signed_digest.digest()) {
            self.init_proof(&signed_digest)?;
        }
        let digest = signed_digest.digest();

        match self.digest_to_proof.entry(signed_digest.digest()) {
            Entry::Occupied(mut entry) => {
                entry.get_mut().add_signature(signed_digest)?;
                if entry.get_mut().ready(validator_verifier) {
                    let (_, state) = entry.remove_entry();
                    let proof = state.take(validator_verifier);
                    // quorum store measurements
                    let duration = chrono::Utc::now().naive_utc().timestamp_micros() as u64
                        - self
                            .digest_to_time
                            .remove(&digest)
                            .expect("Batch created without recording the time!");
                    counters::BATCH_TO_POS_DURATION
                        .observe_duration(Duration::from_micros(duration));
                    return Ok(Some(proof));
                }
            },
            Entry::Vacant(_) => (),
        }
        Ok(None)
    }

    async fn expire(&mut self) {
        let mut batch_ids = vec![];
        for signed_digest_info in self.timeouts.expire() {
            counters::TIMEOUT_BATCHES_COUNT.inc();
            self.digest_to_proof.remove(&signed_digest_info.digest);
            batch_ids.push(signed_digest_info.batch_id);
        }
        if self
            .batch_generator_cmd_tx
            .send(BatchGeneratorCommand::ProofExpiration(batch_ids))
            .await
            .is_err()
        {
            warn!("Failed to send proof expiration to batch generator");
        }
    }

    pub async fn start(
        mut self,
        mut rx: Receiver<ProofCoordinatorCommand>,
        mut network_sender: impl QuorumStoreSender,
        validator_verifier: ValidatorVerifier,
    ) {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                Some(command) = rx.recv() => monitor!("proof_coordinator_handle_command", {
                    match command {
                        ProofCoordinatorCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack to QuorumStore");
                            break;
                        },
                        ProofCoordinatorCommand::AppendSignature(signed_digest) => {
                            let peer_id = signed_digest.signer();
                            let digest = signed_digest.digest();
                            match self.add_signature(signed_digest, &validator_verifier) {
                                Ok(result) => {
                                    if let Some(proof) = result {
                                        debug!("QS: received quorum of signatures, digest {}", digest);
                                        network_sender.broadcast_proof_of_store(proof).await;
                                    }
                                },
                                Err(e) => {
                                    // TODO: better error messages
                                    // Can happen if we already garbage collected
                                    if peer_id == self.peer_id {
                                        debug!("QS: could not add signature from self, err = {:?}", e);
                                    }
                                },
                            }
                        },
                    }
                }),
                _ = interval.tick() => {
                    monitor!("proof_coordinator_handle_tick", self.expire().await);
                }
            }
        }
    }
}
