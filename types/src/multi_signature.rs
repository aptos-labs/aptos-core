// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::bls12381;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use itertools::Itertools;
use std::collections::{BTreeMap, HashMap};

use crate::block_info::Round;
use aptos_bitvec::BitVec;
use move_deps::move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// This struct represents a BLS multi-signature or aggregated signature:
/// it stores a bit mask representing the set of validators participating in the signing process
/// and the multi-signature/aggregated signature itself,
/// which was aggregated from these validators' partial BLS signatures.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AggregatedSignature {
    validator_bitmask: BitVec,
    aggregated_sig: Option<bls12381::Signature>,
}

impl AggregatedSignature {
    pub fn new(
        validator_bitmask: BitVec,
        aggregated_signature: Option<bls12381::Signature>,
    ) -> Self {
        Self {
            validator_bitmask,
            aggregated_sig: aggregated_signature,
        }
    }

    pub fn empty() -> Self {
        Self {
            validator_bitmask: BitVec::default(),
            aggregated_sig: None,
        }
    }

    pub fn get_voters_bitvec(&self) -> &BitVec {
        &self.validator_bitmask
    }

    pub fn get_voter_addresses(
        &self,
        validator_addresses: &[AccountAddress],
    ) -> Vec<AccountAddress> {
        validator_addresses
            .iter()
            .enumerate()
            .filter_map(|(index, addr)| {
                if self.validator_bitmask.is_set(index as u16) {
                    Some(*addr)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn get_num_voters(&self) -> usize {
        self.validator_bitmask.count_ones() as usize
    }

    pub fn aggregated_sig(&self) -> &Option<bls12381::Signature> {
        &self.aggregated_sig
    }
}

/// Partial signature from a set of validators. This struct is only used when aggregating the votes
/// from different validators. It is only kept in memory and never sent through the network.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
pub struct PartialSignatures {
    signatures: BTreeMap<AccountAddress, bls12381::Signature>,
}

impl PartialSignatures {
    pub fn new(signatures: BTreeMap<AccountAddress, bls12381::Signature>) -> Self {
        Self { signatures }
    }

    pub fn empty() -> Self {
        Self::new(BTreeMap::new())
    }

    pub fn is_empty(&self) -> bool {
        self.signatures.is_empty()
    }

    pub fn remove_signature(&mut self, validator: AccountAddress) {
        self.signatures.remove(&validator);
    }

    pub fn add_signature(&mut self, validator: AccountAddress, signature: bls12381::Signature) {
        self.signatures.entry(validator).or_insert(signature);
    }

    pub fn signatures(&self) -> &BTreeMap<AccountAddress, bls12381::Signature> {
        &self.signatures
    }
}

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

    //#[cfg(test)]
    pub fn replace_signature(
        &mut self,
        validator: AccountAddress,
        round: Round,
        signature: bls12381::Signature,
    ) {
        self.signatures.insert(validator, (round, signature));
    }

    //#[cfg(test)]
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

    /// Returns partial signature and a vector of rounds ordered by validator index
    pub fn get_partial_sig_with_rounds(
        &self,
        address_to_validator_index: &HashMap<AccountAddress, usize>,
    ) -> (PartialSignatures, Vec<Round>) {
        let mut partial_sig = PartialSignatures::empty();
        let mut index_to_rounds = BTreeMap::new();
        self.signatures.iter().for_each(|(address, (round, sig))| {
            address_to_validator_index
                .get(address)
                .into_iter()
                .for_each(|index| {
                    partial_sig.add_signature(*address, sig.clone());
                    index_to_rounds.insert(index, round.clone());
                });
        });
        (partial_sig, index_to_rounds.into_values().collect_vec())
    }
}

/// This struct stores the aggregated signatures and corresponding rounds for timeout messages. Please
/// note that the order of the round is same as the bitmask in the aggregated signature i.e.,
/// first entry in the rounds corresponds to validator address with the first bitmask set in the
/// aggregated signature and so on. The ordering is crucial for verification of the timeout messages.
#[derive(Debug, Clone, Serialize, Deserialize, Eq, PartialEq)]
pub struct AggregatedSignatureWithRounds {
    aggregated_sig: AggregatedSignature,
    rounds: Vec<Round>,
}

impl AggregatedSignatureWithRounds {
    pub fn new(aggregated_sig: AggregatedSignature, rounds: Vec<Round>) -> Self {
        assert_eq!(aggregated_sig.get_num_voters(), rounds.len());
        Self {
            aggregated_sig,
            rounds,
        }
    }

    pub fn empty() -> Self {
        Self {
            aggregated_sig: AggregatedSignature::empty(),
            rounds: vec![],
        }
    }

    pub fn get_voters(&self, validator_addresses: &[AccountAddress]) -> Vec<AccountAddress> {
        self.aggregated_sig.get_voter_addresses(validator_addresses)
    }

    pub fn get_voters_and_rounds(
        &self,
        validator_addresses: &[AccountAddress],
    ) -> Vec<(AccountAddress, Round)> {
        self.aggregated_sig
            .get_voter_addresses(validator_addresses)
            .into_iter()
            .zip(self.rounds.clone())
            .collect()
    }

    pub fn aggregated_sig(&self) -> &AggregatedSignature {
        &self.aggregated_sig
    }

    pub fn rounds(&self) -> &Vec<Round> {
        &self.rounds
    }
}
