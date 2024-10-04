// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

#[cfg(any(test, feature = "fuzzing"))]
use crate::validator_signer::ValidatorSigner;
use crate::{
    account_address::AccountAddress,
    block_info::{BlockInfo, Round},
    epoch_state::EpochState,
    on_chain_config::ValidatorSet,
    transaction::Version,
    validator_verifier::{ValidatorVerifier, VerifyError},
};
use aptos_crypto::{bls12381, hash::HashValue};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::{Display, Formatter},
    mem,
    ops::{Deref, DerefMut},
    sync::Arc,
};

/// This structure serves a dual purpose.
///
/// First, if this structure is signed by 2f+1 validators it signifies the state of the ledger at
/// version `version` -- it contains the transaction accumulator at that version which commits to
/// all historical transactions. This structure may be expanded to include other information that
/// is derived from that accumulator (e.g. the current time according to the time contract) to
/// reduce the number of proofs a client must get.
///
/// Second, the structure contains a `consensus_data_hash` value. This is the hash of an internal
/// data structure that represents a block that is voted on in Consensus. If 2f+1 signatures are
/// gathered on the same ledger info that represents a Quorum Certificate (QC) on the consensus
/// data.
///
/// Combining these two concepts, when a validator votes on a block, B it votes for a
/// LedgerInfo with the `version` being the latest version that will be committed if B gets 2f+1
/// votes. It sets `consensus_data_hash` to represent B so that if those 2f+1 votes are gathered a
/// QC is formed on B.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct LedgerInfo {
    commit_info: BlockInfo,

    /// Hash of consensus specific data that is opaque to all parts of the system other than
    /// consensus.
    consensus_data_hash: HashValue,
}

impl Display for LedgerInfo {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(
            f,
            "LedgerInfo: [commit_info: {}] [Consensus data hash: {}]",
            self.commit_info(),
            self.consensus_data_hash()
        )
    }
}

impl LedgerInfo {
    pub fn dummy() -> Self {
        Self {
            commit_info: BlockInfo::empty(),
            consensus_data_hash: HashValue::zero(),
        }
    }

    pub fn is_dummy(&self) -> bool {
        self.commit_info.is_empty() && self.consensus_data_hash == HashValue::zero()
    }

    /// Constructs a `LedgerInfo` object based on the given commit info and vote data hash.
    pub fn new(commit_info: BlockInfo, consensus_data_hash: HashValue) -> Self {
        Self {
            commit_info,
            consensus_data_hash,
        }
    }

    /// Create a new LedgerInfo at genesis with the given genesis state and
    /// initial validator set.
    pub fn genesis(genesis_state_root_hash: HashValue, validator_set: ValidatorSet) -> Self {
        Self::new(
            BlockInfo::genesis(genesis_state_root_hash, validator_set),
            HashValue::zero(),
        )
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn mock_genesis(validator_set: Option<ValidatorSet>) -> Self {
        Self::new(BlockInfo::mock_genesis(validator_set), HashValue::zero())
    }

    /// The `BlockInfo` of a committed block.
    pub fn commit_info(&self) -> &BlockInfo {
        &self.commit_info
    }

    /// A series of wrapper functions for the data stored in the commit info. For the detailed
    /// information, please refer to `BlockInfo`
    pub fn epoch(&self) -> u64 {
        self.commit_info.epoch()
    }

    pub fn next_block_epoch(&self) -> u64 {
        self.commit_info.next_block_epoch()
    }

    pub fn round(&self) -> Round {
        self.commit_info.round()
    }

    pub fn consensus_block_id(&self) -> HashValue {
        self.commit_info.id()
    }

    pub fn transaction_accumulator_hash(&self) -> HashValue {
        self.commit_info.executed_state_id()
    }

    pub fn version(&self) -> Version {
        self.commit_info.version()
    }

    pub fn timestamp_usecs(&self) -> u64 {
        self.commit_info.timestamp_usecs()
    }

    pub fn next_epoch_state(&self) -> Option<&EpochState> {
        self.commit_info.next_epoch_state()
    }

    pub fn ends_epoch(&self) -> bool {
        self.next_epoch_state().is_some()
    }

    /// Returns hash of consensus voting data in this `LedgerInfo`.
    pub fn consensus_data_hash(&self) -> HashValue {
        self.consensus_data_hash
    }

    pub fn set_consensus_data_hash(&mut self, consensus_data_hash: HashValue) {
        self.consensus_data_hash = consensus_data_hash;
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn set_executed_state_id(&mut self, id: HashValue) {
        self.commit_info.set_executed_state_id(id)
    }
}

/// Wrapper around LedgerInfoWithScheme to support future upgrades, this is the data being persisted.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LedgerInfoWithSignatures {
    V0(LedgerInfoWithV0),
}

impl Display for LedgerInfoWithSignatures {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        match self {
            LedgerInfoWithSignatures::V0(ledger) => write!(f, "{}", ledger),
        }
    }
}

// proxy to create LedgerInfoWithbls12381::
impl LedgerInfoWithSignatures {
    pub fn new(ledger_info: LedgerInfo, signatures: AggregateSignature) -> Self {
        LedgerInfoWithSignatures::V0(LedgerInfoWithV0::new(ledger_info, signatures))
    }

    pub fn genesis(genesis_state_root_hash: HashValue, validator_set: ValidatorSet) -> Self {
        LedgerInfoWithSignatures::V0(LedgerInfoWithV0::genesis(
            genesis_state_root_hash,
            validator_set,
        ))
    }
}

/// Helper function to generate LedgerInfoWithSignature from a set of validator signers used for testing
#[cfg(any(test, feature = "fuzzing"))]
pub fn generate_ledger_info_with_sig(
    validators: &[ValidatorSigner],
    ledger_info: LedgerInfo,
) -> LedgerInfoWithSignatures {
    let partial_sig = PartialSignatures::new(
        validators
            .iter()
            .map(|signer| (signer.author(), signer.sign(&ledger_info).unwrap()))
            .collect(),
    );

    let validator_verifier = generate_validator_verifier(validators);

    LedgerInfoWithSignatures::new(
        ledger_info,
        validator_verifier
            .aggregate_signatures(partial_sig.signatures_iter())
            .unwrap(),
    )
}

// Temporary hack to avoid massive changes, it won't work when new variant comes and needs proper
// dispatch at that time.
impl Deref for LedgerInfoWithSignatures {
    type Target = LedgerInfoWithV0;

    fn deref(&self) -> &LedgerInfoWithV0 {
        match &self {
            LedgerInfoWithSignatures::V0(ledger) => ledger,
        }
    }
}

impl DerefMut for LedgerInfoWithSignatures {
    fn deref_mut(&mut self) -> &mut LedgerInfoWithV0 {
        match self {
            LedgerInfoWithSignatures::V0(ref mut ledger) => ledger,
        }
    }
}

/// The validator node returns this structure which includes signatures
/// from validators that confirm the state.  The client needs to only pass back
/// the LedgerInfo element since the validator node doesn't need to know the signatures
/// again when the client performs a query, those are only there for the client
/// to be able to verify the state
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct LedgerInfoWithV0 {
    ledger_info: LedgerInfo,
    /// Aggregated BLS signature of all the validators that signed the message. The bitmask in the
    /// aggregated signature can be used to find out the individual validators signing the message
    signatures: AggregateSignature,
}

impl Display for LedgerInfoWithV0 {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.ledger_info)
    }
}

impl LedgerInfoWithV0 {
    pub fn new(ledger_info: LedgerInfo, signatures: AggregateSignature) -> Self {
        LedgerInfoWithV0 {
            ledger_info,
            signatures,
        }
    }

    pub fn dummy() -> Self {
        Self {
            ledger_info: LedgerInfo::dummy(),
            signatures: AggregateSignature::empty(),
        }
    }

    /// Create a new `LedgerInfoWithSignatures` at genesis with the given genesis
    /// state and initial validator set.
    ///
    /// Note that the genesis `LedgerInfoWithSignatures` is unsigned. Validators
    /// and FullNodes are configured with the same genesis transaction and generate
    /// an identical genesis `LedgerInfoWithSignatures` independently. In contrast,
    /// Clients will likely use a waypoint generated from the genesis `LedgerInfo`.
    pub fn genesis(genesis_state_root_hash: HashValue, validator_set: ValidatorSet) -> Self {
        Self::new(
            LedgerInfo::genesis(genesis_state_root_hash, validator_set),
            AggregateSignature::empty(),
        )
    }

    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    pub fn commit_info(&self) -> &BlockInfo {
        self.ledger_info.commit_info()
    }

    pub fn get_voters(&self, validator_addresses: &[AccountAddress]) -> Vec<AccountAddress> {
        self.signatures.get_signers_addresses(validator_addresses)
    }

    pub fn get_num_voters(&self) -> usize {
        self.signatures.get_num_voters()
    }

    pub fn get_voters_bitvec(&self) -> &BitVec {
        self.signatures.get_signers_bitvec()
    }

    pub fn verify_signatures(
        &self,
        validator: &ValidatorVerifier,
    ) -> ::std::result::Result<(), VerifyError> {
        validator.verify_multi_signatures(self.ledger_info(), &self.signatures)
    }

    pub fn check_voting_power(
        &self,
        validator: &ValidatorVerifier,
    ) -> ::std::result::Result<u128, VerifyError> {
        validator.check_voting_power(
            self.get_voters(&validator.get_ordered_account_addresses_iter().collect_vec())
                .iter(),
            true,
        )
    }

    pub fn signatures(&self) -> &AggregateSignature {
        &self.signatures
    }
}

/// Contains the ledger info and partially aggregated signature from a set of validators, this data
/// is only used during the aggregating the votes from different validators and is not persisted in DB.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerInfoWithVerifiedSignatures {
    ledger_info: LedgerInfo,
    partial_sigs: PartialSignatures,
}

impl Display for LedgerInfoWithVerifiedSignatures {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.ledger_info)
    }
}

impl LedgerInfoWithVerifiedSignatures {
    pub fn new(ledger_info: LedgerInfo, signatures: PartialSignatures) -> Self {
        Self {
            ledger_info,
            partial_sigs: signatures,
        }
    }

    pub fn commit_info(&self) -> &BlockInfo {
        self.ledger_info.commit_info()
    }

    pub fn remove_signature(&mut self, validator: AccountAddress) {
        self.partial_sigs.remove_signature(validator);
    }

    pub fn add_signature(&mut self, validator: AccountAddress, signature: bls12381::Signature) {
        self.partial_sigs.add_signature(validator, signature);
    }

    pub fn signatures(&self) -> &BTreeMap<AccountAddress, bls12381::Signature> {
        self.partial_sigs.signatures()
    }

    pub fn aggregate_signatures(
        &self,
        verifier: &ValidatorVerifier,
    ) -> Result<LedgerInfoWithSignatures, VerifyError> {
        let aggregated_sig = verifier.aggregate_signatures(self.partial_sigs.signatures_iter())?;
        Ok(LedgerInfoWithSignatures::new(
            self.ledger_info.clone(),
            aggregated_sig,
        ))
    }

    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }

    pub fn partial_sigs(&self) -> &PartialSignatures {
        &self.partial_sigs
    }
}

pub enum SignatureWithStatus {
    Verified(bls12381::Signature),
    Unverified(bls12381::Signature),
}

/// This data structure is used to support the optimistic signature verification feature.
/// Contains the ledger info and the signatures received on the ledger info from different validators.
/// Some of the signatures could be verified before inserting into this data structure. Some of the signatures
/// are not verified. Rather than verifying the signatures immediately, we aggregate all the signatures and
/// verify the aggregated signature at once. If the aggregated signature is invalid, then we verify each individual
/// unverified signature and remove the invalid signatures.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LedgerInfoWithUnverifiedSignatures {
    ledger_info: LedgerInfo,
    // These signatures are not yet verified. For efficiency, once enough unverified signatures are collected,
    // they will be aggregated and verified.
    unverified_signatures: PartialSignatures,
    verified_signatures: PartialSignatures,
}

impl Display for LedgerInfoWithUnverifiedSignatures {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{}", self.ledger_info)
    }
}

impl LedgerInfoWithUnverifiedSignatures {
    pub fn new(ledger_info: LedgerInfo) -> Self {
        Self {
            ledger_info,
            unverified_signatures: PartialSignatures::empty(),
            verified_signatures: PartialSignatures::empty(),
        }
    }

    pub fn commit_info(&self) -> &BlockInfo {
        self.ledger_info.commit_info()
    }

    fn add_verified_signature(
        &mut self,
        validator: AccountAddress,
        signature: bls12381::Signature,
    ) {
        self.verified_signatures.add_signature(validator, signature);
        self.unverified_signatures.remove_signature(validator);
    }

    fn add_unverified_signature(
        &mut self,
        validator: AccountAddress,
        signature: bls12381::Signature,
    ) {
        if self.verified_signatures.contains_voter(&validator) {
            return;
        }
        self.unverified_signatures
            .add_signature(validator, signature);
    }

    pub fn add_signature(
        &mut self,
        validator: AccountAddress,
        signature_with_status: SignatureWithStatus,
    ) {
        match signature_with_status {
            SignatureWithStatus::Verified(signature) => {
                self.add_verified_signature(validator, signature)
            },
            SignatureWithStatus::Unverified(signature) => {
                self.add_unverified_signature(validator, signature)
            },
        };
    }

    pub fn verified_voters(&self) -> impl Iterator<Item = &AccountAddress> {
        self.verified_signatures.signatures().keys()
    }

    pub fn unverified_voters(&self) -> impl Iterator<Item = &AccountAddress> {
        self.unverified_signatures.signatures().keys()
    }

    // Collecting all the authors from verified signatures, unverified signatures and the aggregated signature.
    pub fn all_voters(&self) -> impl Iterator<Item = &AccountAddress> {
        self.verified_voters().chain(self.unverified_voters())
    }

    pub fn check_voting_power(
        &self,
        verifier: &ValidatorVerifier,
        check_super_majority: bool,
    ) -> std::result::Result<u128, VerifyError> {
        let all_voters = self.all_voters();
        verifier.check_voting_power(all_voters, check_super_majority)
    }

    fn try_aggregate(
        &mut self,
        epoch_state: &EpochState,
    ) -> Result<AggregateSignature, VerifyError> {
        self.check_voting_power(&epoch_state.verifier, true)?;

        let all_signatures = self
            .verified_signatures
            .signatures_iter()
            .chain(self.unverified_signatures.signatures_iter());

        epoch_state.verifier.aggregate_signatures(all_signatures)
    }

    /// Merge unverified signatures into verified signatures if they are valid.
    fn merge_signatures(&mut self, verifier: &ValidatorVerifier, need_verify: bool) {
        let unverified_signatures =
            mem::replace(&mut self.unverified_signatures, PartialSignatures::empty()).unpack();
        let valid_signatures: Vec<_> = unverified_signatures
            .into_par_iter()
            .flat_map(|(account_address, signature)| {
                if !need_verify
                    || verifier
                        .verify(account_address, self.ledger_info(), &signature)
                        .is_ok()
                {
                    Some((account_address, signature))
                } else {
                    verifier.add_pessimistic_verify_set(account_address);
                    None
                }
            })
            .collect();
        for (account_address, signature) in valid_signatures {
            self.verified_signatures
                .add_signature(account_address, signature);
        }
    }

    /// Try to aggregate all signatures if the voting power is enough. If the aggregated signature is
    /// valid, return the LedgerInfoWithSignatures. Also merge valid unverified signatures into verified.
    pub fn aggregate_and_verify(
        &mut self,
        epoch_state: Arc<EpochState>,
    ) -> Result<LedgerInfoWithSignatures, VerifyError> {
        let aggregated_sig = self.try_aggregate(&epoch_state)?;

        match epoch_state
            .verifier
            .verify_multi_signatures(self.ledger_info(), &aggregated_sig)
        {
            Ok(_) => {
                self.merge_signatures(&epoch_state.verifier, false);
                Ok(LedgerInfoWithSignatures::new(
                    self.ledger_info.clone(),
                    aggregated_sig,
                ))
            },
            Err(_) => {
                self.merge_signatures(&epoch_state.verifier, true);

                let aggregate_sig = self.try_aggregate(&epoch_state)?;
                Ok(LedgerInfoWithSignatures::new(
                    self.ledger_info.clone(),
                    aggregate_sig,
                ))
            },
        }
    }

    pub fn ledger_info(&self) -> &LedgerInfo {
        &self.ledger_info
    }
}

//
// Arbitrary implementation of LedgerInfoWithV0 (for fuzzing)
//

use crate::aggregate_signature::{AggregateSignature, PartialSignatures};
#[cfg(any(test, feature = "fuzzing"))]
use crate::validator_verifier::generate_validator_verifier;
#[cfg(any(test, feature = "fuzzing"))]
use crate::validator_verifier::random_validator_verifier;
use aptos_bitvec::BitVec;
use itertools::Itertools;
#[cfg(any(test, feature = "fuzzing"))]
use proptest::prelude::*;

#[cfg(any(test, feature = "fuzzing"))]
impl Arbitrary for LedgerInfoWithV0 {
    type Parameters = ();
    type Strategy = BoxedStrategy<Self>;

    fn arbitrary_with(_args: Self::Parameters) -> Self::Strategy {
        let dummy_signature = bls12381::Signature::dummy_signature();
        (any::<LedgerInfo>(), (1usize..100))
            .prop_map(move |(ledger_info, num_validators)| {
                let (signers, verifier) = random_validator_verifier(num_validators, None, true);
                let mut partial_sig = PartialSignatures::empty();
                for signer in signers {
                    let signature = dummy_signature.clone();
                    partial_sig.add_signature(signer.author(), signature);
                }
                let aggregated_sig = verifier
                    .aggregate_signatures(partial_sig.signatures_iter())
                    .unwrap();
                Self {
                    ledger_info,
                    signatures: aggregated_sig,
                }
            })
            .boxed()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{validator_signer::ValidatorSigner, validator_verifier::ValidatorConsensusInfo};

    #[test]
    fn test_signatures_hash() {
        let ledger_info = LedgerInfo::new(BlockInfo::empty(), HashValue::random());

        const NUM_SIGNERS: u8 = 7;
        // Generate NUM_SIGNERS random signers.
        let validator_signers: Vec<ValidatorSigner> = (0..NUM_SIGNERS)
            .map(|i| ValidatorSigner::random([i; 32]))
            .collect();
        let mut partial_sig = PartialSignatures::empty();
        let mut validator_infos = vec![];

        for validator in validator_signers.iter() {
            validator_infos.push(ValidatorConsensusInfo::new(
                validator.author(),
                validator.public_key(),
                1,
            ));
            partial_sig.add_signature(validator.author(), validator.sign(&ledger_info).unwrap());
        }

        // Let's assume our verifier needs to satisfy at least 5 quorum voting power
        let validator_verifier =
            ValidatorVerifier::new_with_quorum_voting_power(validator_infos, 5)
                .expect("Incorrect quorum size.");

        let mut aggregated_signature = validator_verifier
            .aggregate_signatures(partial_sig.signatures_iter())
            .unwrap();

        let ledger_info_with_signatures =
            LedgerInfoWithV0::new(ledger_info.clone(), aggregated_signature);

        // Add the signatures in reverse order and ensure the serialization matches
        partial_sig = PartialSignatures::empty();
        for validator in validator_signers.iter().rev() {
            partial_sig.add_signature(validator.author(), validator.sign(&ledger_info).unwrap());
        }

        aggregated_signature = validator_verifier
            .aggregate_signatures(partial_sig.signatures_iter())
            .unwrap();

        let ledger_info_with_signatures_reversed =
            LedgerInfoWithV0::new(ledger_info, aggregated_signature);

        let ledger_info_with_signatures_bytes =
            bcs::to_bytes(&ledger_info_with_signatures).expect("block serialization failed");
        let ledger_info_with_signatures_reversed_bytes =
            bcs::to_bytes(&ledger_info_with_signatures_reversed)
                .expect("block serialization failed");

        assert_eq!(
            ledger_info_with_signatures_bytes,
            ledger_info_with_signatures_reversed_bytes
        );
    }

    #[test]
    fn test_ledger_info_with_mixed_signatures() {
        let ledger_info = LedgerInfo::new(BlockInfo::empty(), HashValue::random());
        const NUM_SIGNERS: u8 = 7;
        // Generate NUM_SIGNERS random signers.
        let validator_signers: Vec<ValidatorSigner> = (0..NUM_SIGNERS)
            .map(|i| ValidatorSigner::random([i; 32]))
            .collect();
        let mut validator_infos = vec![];

        for validator in validator_signers.iter() {
            validator_infos.push(ValidatorConsensusInfo::new(
                validator.author(),
                validator.public_key(),
                1,
            ));
        }

        let validator_verifier =
            ValidatorVerifier::new_with_quorum_voting_power(validator_infos, 5)
                .expect("Incorrect quorum size.");
        let epoch_state = Arc::new(EpochState::new(10, validator_verifier));

        let mut ledger_info_with_mixed_signatures =
            LedgerInfoWithUnverifiedSignatures::new(ledger_info.clone());

        let mut partial_sig = PartialSignatures::empty();

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[0].author(),
            SignatureWithStatus::Verified(validator_signers[0].sign(&ledger_info).unwrap()),
        );
        partial_sig.add_signature(
            validator_signers[0].author(),
            validator_signers[0].sign(&ledger_info).unwrap(),
        );

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[1].author(),
            SignatureWithStatus::Unverified(validator_signers[1].sign(&ledger_info).unwrap()),
        );
        partial_sig.add_signature(
            validator_signers[1].author(),
            validator_signers[1].sign(&ledger_info).unwrap(),
        );

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[2].author(),
            SignatureWithStatus::Verified(validator_signers[2].sign(&ledger_info).unwrap()),
        );
        partial_sig.add_signature(
            validator_signers[2].author(),
            validator_signers[2].sign(&ledger_info).unwrap(),
        );

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[3].author(),
            SignatureWithStatus::Unverified(validator_signers[3].sign(&ledger_info).unwrap()),
        );
        partial_sig.add_signature(
            validator_signers[3].author(),
            validator_signers[3].sign(&ledger_info).unwrap(),
        );

        assert_eq!(ledger_info_with_mixed_signatures.all_voters().count(), 4);
        assert_eq!(
            ledger_info_with_mixed_signatures
                .unverified_signatures
                .signatures()
                .len(),
            2
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .verified_signatures
                .signatures()
                .len(),
            2
        );
        assert_eq!(
            ledger_info_with_mixed_signatures.check_voting_power(&epoch_state.verifier, true),
            Err(VerifyError::TooLittleVotingPower {
                voting_power: 4,
                expected_voting_power: 5
            })
        );

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[4].author(),
            SignatureWithStatus::Unverified(bls12381::Signature::dummy_signature()),
        );

        assert_eq!(ledger_info_with_mixed_signatures.all_voters().count(), 5);
        assert_eq!(
            ledger_info_with_mixed_signatures
                .unverified_signatures
                .signatures()
                .len(),
            3
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .verified_signatures
                .signatures()
                .len(),
            2
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .check_voting_power(&epoch_state.verifier, true)
                .unwrap(),
            5
        );
        assert_eq!(
            ledger_info_with_mixed_signatures.aggregate_and_verify(epoch_state.clone()),
            Err(VerifyError::TooLittleVotingPower {
                voting_power: 4,
                expected_voting_power: 5
            })
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .unverified_signatures
                .signatures()
                .len(),
            0
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .verified_signatures
                .signatures()
                .len(),
            4
        );
        assert_eq!(ledger_info_with_mixed_signatures.all_voters().count(), 4);
        assert_eq!(epoch_state.verifier.pessimistic_verify_set().len(), 1);

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[5].author(),
            SignatureWithStatus::Unverified(validator_signers[5].sign(&ledger_info).unwrap()),
        );
        partial_sig.add_signature(
            validator_signers[5].author(),
            validator_signers[5].sign(&ledger_info).unwrap(),
        );

        assert_eq!(ledger_info_with_mixed_signatures.all_voters().count(), 5);
        assert_eq!(
            ledger_info_with_mixed_signatures
                .unverified_signatures
                .signatures()
                .len(),
            1
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .verified_signatures
                .signatures()
                .len(),
            4
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .check_voting_power(&epoch_state.verifier, true)
                .unwrap(),
            5
        );
        let aggregate_sig = LedgerInfoWithSignatures::new(
            ledger_info.clone(),
            epoch_state
                .verifier
                .aggregate_signatures(partial_sig.signatures_iter())
                .unwrap(),
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .aggregate_and_verify(epoch_state.clone())
                .unwrap(),
            aggregate_sig
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .unverified_signatures
                .signatures()
                .len(),
            0
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .verified_signatures
                .signatures()
                .len(),
            5
        );
        assert_eq!(epoch_state.verifier.pessimistic_verify_set().len(), 1);

        ledger_info_with_mixed_signatures.add_signature(
            validator_signers[6].author(),
            SignatureWithStatus::Unverified(bls12381::Signature::dummy_signature()),
        );

        assert_eq!(ledger_info_with_mixed_signatures.all_voters().count(), 6);
        assert_eq!(
            ledger_info_with_mixed_signatures
                .check_voting_power(&epoch_state.verifier, true)
                .unwrap(),
            6
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .aggregate_and_verify(epoch_state.clone())
                .unwrap(),
            aggregate_sig
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .unverified_signatures
                .signatures()
                .len(),
            0
        );
        assert_eq!(
            ledger_info_with_mixed_signatures
                .verified_signatures
                .signatures()
                .len(),
            5
        );
        assert_eq!(ledger_info_with_mixed_signatures.all_voters().count(), 5);
        assert_eq!(epoch_state.verifier.pessimistic_verify_set().len(), 2);
    }
}
