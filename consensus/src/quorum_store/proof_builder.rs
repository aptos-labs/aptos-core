// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::quorum_store::{
    quorum_store::{ProofReturnChannel, QuorumStoreError},
    types::BatchId,
    utils::DigestTimeouts,
};
use aptos_crypto::HashValue;
use aptos_logger::debug;
use aptos_types::validator_verifier::ValidatorVerifier;
use consensus_types::proof_of_store::{ProofOfStore, SignedDigest, SignedDigestError};
use std::collections::HashMap;
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
pub(crate) enum ProofBuilderCommand {
    InitProof(SignedDigest, BatchId, ProofReturnChannel),
    AppendSignature(SignedDigest),
}

pub(crate) struct ProofBuilder {
    // peer_id: PeerId,
    proof_timeout_ms: usize,
    digest_to_proof: HashMap<HashValue, (ProofOfStore, BatchId, ProofReturnChannel)>,
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
        batch_id: BatchId,
        validator_verifier: &ValidatorVerifier,
        tx: ProofReturnChannel,
    ) -> Result<(), SignedDigestError> {
        let info = signed_digest.info.clone();

        self.timeouts.add_digest(info.digest, self.proof_timeout_ms);
        self.digest_to_proof
            .insert(info.digest, (ProofOfStore::new(info), batch_id, tx));

        self.add_signature(signed_digest, &validator_verifier)?;

        // TODO: should we do regular pull to check timeouts or is this Ok?
        self.expire();
        Ok(())
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
            .and_modify(|(proof, _, _)| {
                ret = proof.add_signature(signed_digest.peer_id, signed_digest.signature);
                if ret.is_ok() {
                    ready = proof.ready(validator_verifier);
                }
            });
        if ready {
            let (proof, batch_id, tx) = self.digest_to_proof.remove(&digest).unwrap();
            tx.send(Ok((proof, batch_id)))
                .expect("Unable to send the proof of store");
        }
        ret
    }

    fn expire(&mut self) {
        for digest in self.timeouts.expire() {
            if let Some((_, batch_id, tx)) = self.digest_to_proof.remove(&digest) {
                tx.send(Err(QuorumStoreError::Timeout(batch_id)))
                    .expect("Unable to send the timeout a proof of store");
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
                ProofBuilderCommand::InitProof(signed_digest, batch_id, tx) => {
                    self.init_proof(signed_digest, batch_id, &validator_verifier, tx)
                        .expect("Error initializing proof of store");
                }
                ProofBuilderCommand::AppendSignature(signed_digest) => {
                    if let Err(e) = self.add_signature(signed_digest, &validator_verifier) {
                        // Can happen if we already garbage collected
                        debug!("QS: could not add signature {:?}", e);
                        //TODO: do something
                    } else {
                        debug!("QS: added signature to proof");
                    }
                }
            }
        }
    }
}
