// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::quorum_store::{ProofReturnChannel, QuorumStoreError};
use crate::quorum_store::utils::DigestTimeouts;
use aptos_crypto::HashValue;
use aptos_types::validator_verifier::ValidatorVerifier;
use consensus_types::proof_of_store::{ProofOfStore, SignedDigest, SignedDigestError};
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
pub(crate) enum ProofBuilderCommand {
    InitProof(SignedDigest, ProofReturnChannel),
    AppendSignature(SignedDigest),
}

pub(crate) struct ProofBuilder {
    // peer_id: PeerId,
    proof_timeout_ms: usize,
    digest_to_proof: HashMap<HashValue, (ProofOfStore, ProofReturnChannel)>,
    timeouts: DigestTimeouts,
}

//PoQS builder object - gather signed digest to form PoQS
impl ProofBuilder {
    pub fn new(proof_timeout_ms: usize) -> Self {
        Self {
            // peer_id: my_peer_id,
            proof_timeout_ms,
            digest_to_proof: HashMap::new(),
            timeouts: DigestTimeouts::new(),
        }
    }

    fn init_proof(
        &mut self,
        signed_digest: SignedDigest,
        validator_verifier: &ValidatorVerifier,
        tx: ProofReturnChannel,
    ) {
        let info = signed_digest.info.clone();

        self.timeouts.add_digest(info.digest, self.proof_timeout_ms);
        self.digest_to_proof
            .insert(info.digest, (ProofOfStore::new(info), tx));

        if let Err(_) = self.add_signature(signed_digest, &validator_verifier) {
            //TODO: do something
        }
        self.expire();
    }

    fn add_signature(
        &mut self,
        signed_digest: SignedDigest,
        validator_verifier: &ValidatorVerifier,
    ) -> Result<(), SignedDigestError> {
        if !self
            .digest_to_proof
            .contains_key(&signed_digest.info.digest)
        {
            return Err(SignedDigestError::WrongDigest);
        }
        let mut ret = Ok(());
        let mut ready = false;
        let digest = signed_digest.info.digest.clone();
        self.digest_to_proof
            .entry(signed_digest.info.digest)
            .and_modify(|(proof, _)| {
                ret = proof.add_signature(signed_digest.peer_id, signed_digest.signature);
                if ret.is_ok() {
                    ready = proof.ready(validator_verifier);
                }
            });
        if ready {
            let (proof, tx) = self.digest_to_proof.remove(&digest).unwrap();
            tx.send(Ok(proof)).expect("Unable to send proof of store");
        }
        ret
    }

    fn expire(&mut self) {
        for digest in self.timeouts.expire() {
            if let Some((_, tx)) = self.digest_to_proof.remove(&digest) {
                tx.send(Err(QuorumStoreError::Timeout(digest)))
                    .expect("Unable to send proof of store");
            }
        }
    }

    pub async fn start(
        mut self,
        mut network_rx: Receiver<ProofBuilderCommand>,
        validator_verifier: ValidatorVerifier,
    ) {
        while let Some(command) = network_rx.recv().await {
            match command {
                ProofBuilderCommand::InitProof(signed_digest, tx) => {
                    self.init_proof(signed_digest, &validator_verifier, tx);
                }
                ProofBuilderCommand::AppendSignature(signed_digest) => {
                    if let Err(_) = self.add_signature(signed_digest, &validator_verifier) {
                        // Can happen if we already garbage collected
                        //TODO: do something
                    }
                }
            }
        }
    }
}
