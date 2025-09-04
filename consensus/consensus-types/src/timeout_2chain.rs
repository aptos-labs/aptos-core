// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::{common::Author, quorum_cert::QuorumCert};
use anyhow::ensure;
use velor_crypto::{bls12381, CryptoMaterialError};
use velor_crypto_derive::{BCSCryptoHash, CryptoHasher};
use velor_types::{
    account_address::AccountAddress,
    aggregate_signature::{AggregateSignature, PartialSignatures},
    block_info::Round,
    validator_signer::ValidatorSigner,
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::{Display, Formatter},
};

/// This structure contains all the information necessary to construct a signature
/// on the equivalent of a VelorBFT v4 timeout message.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct TwoChainTimeout {
    /// Epoch number corresponds to the set of validators that are active for this round.
    epoch: u64,
    /// The consensus protocol executes proposals (blocks) in rounds, which monotonically increase per epoch.
    round: Round,
    /// The highest quorum cert the signer has seen.
    quorum_cert: QuorumCert,
}

impl TwoChainTimeout {
    pub fn new(epoch: u64, round: Round, quorum_cert: QuorumCert) -> Self {
        Self {
            epoch,
            round,
            quorum_cert,
        }
    }

    pub fn epoch(&self) -> u64 {
        self.epoch
    }

    pub fn round(&self) -> Round {
        self.round
    }

    pub fn hqc_round(&self) -> Round {
        self.quorum_cert.certified_block().round()
    }

    pub fn quorum_cert(&self) -> &QuorumCert {
        &self.quorum_cert
    }

    pub fn sign(
        &self,
        signer: &ValidatorSigner,
    ) -> Result<bls12381::Signature, CryptoMaterialError> {
        signer.sign(&self.signing_format())
    }

    pub fn signing_format(&self) -> TimeoutSigningRepr {
        TimeoutSigningRepr {
            epoch: self.epoch(),
            round: self.round(),
            hqc_round: self.hqc_round(),
        }
    }

    pub fn verify(&self, validators: &ValidatorVerifier) -> anyhow::Result<()> {
        ensure!(
            self.hqc_round() < self.round(),
            "Timeout round should be larger than the QC round"
        );
        self.quorum_cert.verify(validators)?;
        Ok(())
    }
}

impl Display for TwoChainTimeout {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "Timeout: [epoch: {}, round: {}, hqc_round: {}]",
            self.epoch,
            self.round,
            self.hqc_round(),
        )
    }
}

/// Validators sign this structure that allows the TwoChainTimeoutCertificate to store a round number
/// instead of a quorum cert per validator in the signatures field.
#[derive(Serialize, Deserialize, Debug, CryptoHasher, BCSCryptoHash)]
pub struct TimeoutSigningRepr {
    pub epoch: u64,
    pub round: Round,
    pub hqc_round: Round,
}

/// TimeoutCertificate is a proof that 2f+1 participants in epoch i
/// have voted in round r and we can now move to round r+1. VelorBFT v4 requires signature to sign on
/// the TimeoutSigningRepr and carry the TimeoutWithHighestQC with highest quorum cert among 2f+1.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct TwoChainTimeoutCertificate {
    timeout: TwoChainTimeout,
    signatures_with_rounds: AggregateSignatureWithRounds,
}

impl Display for TwoChainTimeoutCertificate {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "TimeoutCertificate[epoch: {}, round: {}, hqc_round: {}]",
            self.timeout.epoch(),
            self.timeout.round(),
            self.timeout.hqc_round(),
        )
    }
}

impl TwoChainTimeoutCertificate {
    /// Creates new TimeoutCertificate
    pub fn new(timeout: TwoChainTimeout) -> Self {
        Self {
            timeout,
            signatures_with_rounds: AggregateSignatureWithRounds::empty(),
        }
    }

    /// Verifies the signatures for each validator, the signature is on the TimeoutSigningRepr where the
    /// hqc_round is in the signature map.
    /// We verify the following:
    /// 1. the highest quorum cert is valid
    /// 2. all signatures are properly formed (timeout.epoch, timeout.round, round)
    /// 3. timeout.hqc_round == max(signed round)
    pub fn verify(&self, validators: &ValidatorVerifier) -> anyhow::Result<()> {
        let hqc_round = self.timeout.hqc_round();
        // Verify the highest timeout validity.
        let (timeout_result, sig_result) = rayon::join(
            || self.timeout.verify(validators),
            || {
                let timeout_messages: Vec<_> = self
                    .signatures_with_rounds
                    .get_voters_and_rounds(
                        &validators
                            .get_ordered_account_addresses_iter()
                            .collect_vec(),
                    )
                    .into_iter()
                    .map(|(_, round)| TimeoutSigningRepr {
                        epoch: self.timeout.epoch(),
                        round: self.timeout.round(),
                        hqc_round: round,
                    })
                    .collect();
                let timeout_messages_ref: Vec<_> = timeout_messages.iter().collect();
                validators.verify_aggregate_signatures(
                    &timeout_messages_ref,
                    self.signatures_with_rounds.sig(),
                )
            },
        );
        timeout_result?;
        sig_result?;
        let signed_hqc = self
            .signatures_with_rounds
            .rounds()
            .iter()
            .max()
            .ok_or_else(|| anyhow::anyhow!("Empty rounds"))?;
        ensure!(
            hqc_round == *signed_hqc,
            "Inconsistent hqc round, qc has round {}, highest signed round {}",
            hqc_round,
            *signed_hqc
        );
        Ok(())
    }

    /// The epoch of the timeout.
    pub fn epoch(&self) -> u64 {
        self.timeout.epoch()
    }

    /// The round of the timeout.
    pub fn round(&self) -> Round {
        self.timeout.round()
    }

    /// The highest hqc round of the 2f+1 participants
    pub fn highest_hqc_round(&self) -> Round {
        self.timeout.hqc_round()
    }

    pub fn signatures_with_rounds(&self) -> &AggregateSignatureWithRounds {
        &self.signatures_with_rounds
    }
}

/// Contains two chain timeout with partial signatures from the validators. This is only used during
/// signature aggregation and does not go through the wire.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct TwoChainTimeoutWithPartialSignatures {
    timeout: TwoChainTimeout,
    signatures: PartialSignaturesWithRound,
}

impl TwoChainTimeoutWithPartialSignatures {
    pub fn new(timeout: TwoChainTimeout) -> Self {
        Self {
            timeout,
            signatures: PartialSignaturesWithRound::empty(),
        }
    }

    /// The epoch of the timeout.
    pub fn epoch(&self) -> u64 {
        self.timeout.epoch()
    }

    /// The round of the timeout.
    pub fn round(&self) -> Round {
        self.timeout.round()
    }

    /// The highest hqc round of the 2f+1 participants
    pub fn highest_hqc_round(&self) -> Round {
        self.timeout.hqc_round()
    }

    /// Returns the signatures certifying the round
    pub fn signers(&self) -> impl Iterator<Item = &Author> {
        self.signatures.signatures().iter().map(|(k, _)| k)
    }

    /// Add a new timeout message from author, the timeout should already be verified in upper layer.
    pub fn add(
        &mut self,
        author: Author,
        timeout: TwoChainTimeout,
        signature: bls12381::Signature,
    ) {
        debug_assert_eq!(
            self.timeout.epoch(),
            timeout.epoch(),
            "Timeout should have the same epoch as TimeoutCert"
        );
        debug_assert_eq!(
            self.timeout.round(),
            timeout.round(),
            "Timeout should have the same round as TimeoutCert"
        );
        let hqc_round = timeout.hqc_round();
        if timeout.hqc_round() > self.timeout.hqc_round() {
            self.timeout = timeout;
        }
        self.signatures.add_signature(author, hqc_round, signature);
    }

    /// Aggregates the partial signature into `TwoChainTimeoutCertificate`. This is done when we
    /// have quorum voting power in the partial signature.
    pub fn aggregate_signatures(
        &self,
        verifier: &ValidatorVerifier,
    ) -> Result<TwoChainTimeoutCertificate, VerifyError> {
        let (partial_sig, ordered_rounds) = self
            .signatures
            .get_partial_sig_with_rounds(verifier.address_to_validator_index());
        let aggregated_sig = verifier.aggregate_signatures(partial_sig.signatures_iter())?;
        Ok(TwoChainTimeoutCertificate {
            timeout: self.timeout.clone(),
            signatures_with_rounds: AggregateSignatureWithRounds::new(
                aggregated_sig,
                ordered_rounds,
            ),
        })
    }
}

/// This struct represents partial signatures along with corresponding rounds collected during
/// timeout aggregation.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct PartialSignaturesWithRound {
    signatures: BTreeMap<AccountAddress, (Round, bls12381::Signature)>,
}

impl PartialSignaturesWithRound {
    pub fn new(signatures: BTreeMap<AccountAddress, (Round, bls12381::Signature)>) -> Self {
        Self { signatures }
    }

    pub fn empty() -> Self {
        Self::new(BTreeMap::new())
    }

    pub fn signatures(&self) -> &BTreeMap<AccountAddress, (Round, bls12381::Signature)> {
        &self.signatures
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn replace_signature(
        &mut self,
        validator: AccountAddress,
        round: Round,
        signature: bls12381::Signature,
    ) {
        self.signatures.insert(validator, (round, signature));
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn remove_signature(&mut self, validator: &AccountAddress) {
        self.signatures.remove(validator);
    }

    pub fn add_signature(
        &mut self,
        validator: AccountAddress,
        round: Round,
        signature: bls12381::Signature,
    ) {
        self.signatures
            .entry(validator)
            .or_insert((round, signature));
    }

    /// Returns partial signature and a vector of rounds ordered by validator index in the validator
    /// verifier.
    pub fn get_partial_sig_with_rounds(
        &self,
        address_to_validator_index: &HashMap<AccountAddress, usize>,
    ) -> (PartialSignatures, Vec<Round>) {
        let mut partial_sig = PartialSignatures::empty();
        let mut index_to_rounds = BTreeMap::new();
        self.signatures.iter().for_each(|(address, (round, sig))| {
            if let Some(index) = address_to_validator_index.get(address) {
                partial_sig.add_signature(*address, sig.clone());
                index_to_rounds.insert(index, *round);
            }
        });
        (partial_sig, index_to_rounds.into_values().collect_vec())
    }
}

/// This struct stores the aggregated signatures and corresponding rounds for timeout messages. Please
/// note that the order of the round is same as the bitmask in the aggregated signature i.e.,
/// first entry in the rounds corresponds to validator address with the first bitmask set in the
/// aggregated signature and so on. The ordering is crucial for verification of the timeout messages.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AggregateSignatureWithRounds {
    sig: AggregateSignature,
    rounds: Vec<Round>,
}

impl AggregateSignatureWithRounds {
    pub fn new(sig: AggregateSignature, rounds: Vec<Round>) -> Self {
        assert_eq!(sig.get_num_voters(), rounds.len());
        Self { sig, rounds }
    }

    pub fn empty() -> Self {
        Self {
            sig: AggregateSignature::empty(),
            rounds: vec![],
        }
    }

    pub fn get_voters(
        &self,
        ordered_validator_addresses: &[AccountAddress],
    ) -> Vec<AccountAddress> {
        self.sig.get_signers_addresses(ordered_validator_addresses)
    }

    pub fn get_voters_and_rounds(
        &self,
        ordered_validator_addresses: &[AccountAddress],
    ) -> Vec<(AccountAddress, Round)> {
        self.sig
            .get_signers_addresses(ordered_validator_addresses)
            .into_iter()
            .zip(self.rounds.clone())
            .collect()
    }

    pub fn sig(&self) -> &AggregateSignature {
        &self.sig
    }

    pub fn rounds(&self) -> &Vec<Round> {
        &self.rounds
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        quorum_cert::QuorumCert,
        timeout_2chain::{TwoChainTimeout, TwoChainTimeoutWithPartialSignatures},
    };
    use velor_crypto::bls12381;

    #[test]
    fn test_2chain_timeout_certificate() {
        use crate::vote_data::VoteData;
        use velor_crypto::hash::CryptoHash;
        use velor_types::{
            aggregate_signature::PartialSignatures,
            block_info::BlockInfo,
            ledger_info::{LedgerInfo, LedgerInfoWithVerifiedSignatures},
            validator_verifier::random_validator_verifier,
        };

        let num_nodes = 4;
        let (signers, validators) = random_validator_verifier(num_nodes, None, false);
        let quorum_size = validators.quorum_voting_power() as usize;
        let generate_quorum = |round, num_of_signature| {
            let vote_data = VoteData::new(BlockInfo::random(round), BlockInfo::random(0));
            let mut ledger_info = LedgerInfoWithVerifiedSignatures::new(
                LedgerInfo::new(BlockInfo::empty(), vote_data.hash()),
                PartialSignatures::empty(),
            );
            for signer in &signers[0..num_of_signature] {
                let signature = signer.sign(ledger_info.ledger_info()).unwrap();
                ledger_info.add_signature(signer.author(), signature);
            }
            QuorumCert::new(
                vote_data,
                ledger_info.aggregate_signatures(&validators).unwrap(),
            )
        };
        let generate_timeout = |round, qc_round| {
            TwoChainTimeout::new(1, round, generate_quorum(qc_round, quorum_size))
        };

        let timeouts: Vec<_> = (1..=3)
            .map(|qc_round| generate_timeout(4, qc_round))
            .collect();
        // timeout cert with (round, hqc round) = (4, 1), (4, 2), (4, 3)
        let mut tc_with_partial_sig =
            TwoChainTimeoutWithPartialSignatures::new(timeouts[0].clone());
        for (timeout, signer) in timeouts.iter().zip(&signers) {
            tc_with_partial_sig.add(
                signer.author(),
                timeout.clone(),
                timeout.sign(signer).unwrap(),
            );
        }

        let tc_with_sig = tc_with_partial_sig
            .aggregate_signatures(&validators)
            .unwrap();
        tc_with_sig.verify(&validators).unwrap();

        // timeout round < hqc round
        let mut invalid_tc_with_partial_sig = tc_with_partial_sig.clone();
        invalid_tc_with_partial_sig.timeout.round = 1;

        let invalid_tc_with_sig = invalid_tc_with_partial_sig
            .aggregate_signatures(&validators)
            .unwrap();
        invalid_tc_with_sig.verify(&validators).unwrap_err();

        // invalid signature
        let mut invalid_timeout_cert = invalid_tc_with_partial_sig.clone();
        invalid_timeout_cert.signatures.replace_signature(
            signers[0].author(),
            0,
            bls12381::Signature::dummy_signature(),
        );

        let invalid_tc_with_sig = invalid_timeout_cert
            .aggregate_signatures(&validators)
            .unwrap();
        invalid_tc_with_sig.verify(&validators).unwrap_err();

        // not enough signatures
        let mut invalid_timeout_cert = invalid_tc_with_partial_sig.clone();
        invalid_timeout_cert
            .signatures
            .remove_signature(&signers[0].author());
        let invalid_tc_with_sig = invalid_timeout_cert
            .aggregate_signatures(&validators)
            .unwrap();

        invalid_tc_with_sig.verify(&validators).unwrap_err();

        // hqc round does not match signed round
        let mut invalid_timeout_cert = invalid_tc_with_partial_sig.clone();
        invalid_timeout_cert.timeout.quorum_cert = generate_quorum(2, quorum_size);

        let invalid_tc_with_sig = invalid_timeout_cert
            .aggregate_signatures(&validators)
            .unwrap();
        invalid_tc_with_sig.verify(&validators).unwrap_err();

        // invalid quorum cert
        let mut invalid_timeout_cert = invalid_tc_with_partial_sig;
        invalid_timeout_cert.timeout.quorum_cert = generate_quorum(3, quorum_size - 1);
        let invalid_tc_with_sig = invalid_timeout_cert
            .aggregate_signatures(&validators)
            .unwrap();

        invalid_tc_with_sig.verify(&validators).unwrap_err();
    }
}
