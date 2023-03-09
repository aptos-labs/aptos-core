// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    batch_generator::ProofError, counters, proof_manager::ProofManagerCommand, types::BatchId,
    utils::DigestTimeouts,
};
use aptos_consensus_types::proof_of_store::{
    ProofOfStore, SignedDigest, SignedDigestError, SignedDigestInfo,
};
use aptos_crypto::{bls12381, HashValue};
use aptos_logger::prelude::*;
use aptos_types::{
    aggregate_signature::PartialSignatures, validator_verifier::ValidatorVerifier, PeerId,
};
use futures::channel::oneshot;
use std::{
    collections::{BTreeMap, HashMap},
    time::Duration,
};
use tokio::{
    sync::{
        mpsc::{Receiver, Sender},
        oneshot as TokioOneshot,
    },
    time,
};

#[derive(Debug)]
pub(crate) enum ProofCoordinatorCommand {
    InitProof(SignedDigestInfo, BatchId, ProofReturnChannel),
    AppendSignature(SignedDigest),
    Shutdown(TokioOneshot::Sender<()>),
}

pub(crate) type ProofReturnChannel = oneshot::Sender<Result<(ProofOfStore, BatchId), ProofError>>;

struct IncrementalProofState {
    info: SignedDigestInfo,
    aggregated_signature: BTreeMap<PeerId, bls12381::Signature>,
    batch_id: BatchId,
    ret_tx: ProofReturnChannel,
}

impl IncrementalProofState {
    fn new(info: SignedDigestInfo, batch_id: BatchId, ret_tx: ProofReturnChannel) -> Self {
        Self {
            info,
            aggregated_signature: BTreeMap::new(),
            batch_id,
            ret_tx,
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

    fn ready(&self, validator_verifier: &ValidatorVerifier, my_peer_id: PeerId) -> bool {
        self.aggregated_signature.contains_key(&my_peer_id)
            && validator_verifier
                .check_voting_power(self.aggregated_signature.keys())
                .is_ok()
    }

    fn take(
        self,
        validator_verifier: &ValidatorVerifier,
    ) -> (ProofOfStore, BatchId, ProofReturnChannel) {
        let proof = match validator_verifier
            .aggregate_signatures(&PartialSignatures::new(self.aggregated_signature))
        {
            Ok(sig) => ProofOfStore::new(self.info, sig),
            Err(e) => unreachable!("Cannot aggregate signatures on digest err = {:?}", e),
        };
        (proof, self.batch_id, self.ret_tx)
    }

    fn send_timeout(self) {
        if self
            .ret_tx
            .send(Err(ProofError::Timeout(self.batch_id)))
            .is_err()
        {
            debug!("Failed to send timeout for batch {}", self.batch_id);
        }
    }
}

pub(crate) struct ProofCoordinator {
    peer_id: PeerId,
    proof_timeout_ms: usize,
    digest_to_proof: HashMap<HashValue, IncrementalProofState>,
    digest_to_time: HashMap<HashValue, u64>,
    // to record the batch creation time
    timeouts: DigestTimeouts,
}

//PoQS builder object - gather signed digest to form PoQS
impl ProofCoordinator {
    pub fn new(proof_timeout_ms: usize, peer_id: PeerId) -> Self {
        Self {
            peer_id,
            proof_timeout_ms,
            digest_to_proof: HashMap::new(),
            digest_to_time: HashMap::new(),
            timeouts: DigestTimeouts::new(),
        }
    }

    fn init_proof(&mut self, info: SignedDigestInfo, batch_id: BatchId, tx: ProofReturnChannel) {
        self.timeouts.add_digest(info.digest, self.proof_timeout_ms);
        self.digest_to_proof.insert(
            info.digest,
            IncrementalProofState::new(info.clone(), batch_id, tx),
        );
        self.digest_to_time
            .entry(info.digest)
            .or_insert(chrono::Utc::now().naive_utc().timestamp_micros() as u64);
    }

    fn add_signature(
        &mut self,
        signed_digest: SignedDigest,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<Option<ProofOfStore>, SignedDigestError> {
        if !self.digest_to_proof.contains_key(&signed_digest.digest()) {
            return Err(SignedDigestError::WrongInfo);
        }
        let mut ret = Ok(());
        let mut proof_changed_to_completed = false;
        let digest = signed_digest.digest();
        let my_id = self.peer_id;
        self.digest_to_proof
            .entry(signed_digest.digest())
            .and_modify(|state| {
                ret = state.add_signature(signed_digest);
                if ret.is_ok() {
                    proof_changed_to_completed = state.ready(validator_verifier, my_id);
                }
            });
        if proof_changed_to_completed {
            let (proof, batch_id, tx) = self
                .digest_to_proof
                .remove(&digest)
                .unwrap()
                .take(validator_verifier);

            // quorum store measurements
            let duration = chrono::Utc::now().naive_utc().timestamp_micros() as u64
                - self
                    .digest_to_time
                    .get(&digest)
                    .expect("Batch created without recording the time!");
            counters::BATCH_TO_POS_DURATION.observe_duration(Duration::from_micros(duration));

            // TODO: just send back an ack
            if tx.send(Ok((proof.clone(), batch_id))).is_err() {
                debug!("Failed to send back completion for batch {}", batch_id);
            }

            Ok(Some(proof))
        } else {
            Ok(None)
        }
    }

    fn expire(&mut self) {
        for digest in self.timeouts.expire() {
            if let Some(state) = self.digest_to_proof.remove(&digest) {
                state.send_timeout();
            }
        }
    }

    pub async fn start(
        mut self,
        mut rx: Receiver<ProofCoordinatorCommand>,
        tx: Sender<ProofManagerCommand>,
        validator_verifier: ValidatorVerifier,
    ) {
        let mut interval = time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                Some(command) = rx.recv() => {
                    match command {
                        ProofCoordinatorCommand::Shutdown(ack_tx) => {
                            ack_tx
                                .send(())
                                .expect("Failed to send shutdown ack to QuorumStore");
                            break;
                        },
                        ProofCoordinatorCommand::InitProof(info, batch_id, tx) => {
                            debug!("QS: init proof, batch_id {}, digest {}", batch_id, info.digest);
                            self.init_proof(info, batch_id, tx);
                        },
                        ProofCoordinatorCommand::AppendSignature(signed_digest) => {
                            let peer_id = signed_digest.signer();
                            let digest = signed_digest.digest();
                            match self.add_signature(signed_digest, &validator_verifier) {
                                Ok(result) => {
                                    if let Some(proof) = result {
                                        debug!("QS: received quorum of signatures, digest {}", digest);
                                        tx.send(ProofManagerCommand::LocalProof(proof)).await.unwrap();
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
                }
                _ = interval.tick() => {
                    self.expire();
                }
            }
        }
    }
}
