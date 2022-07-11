// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::bls12381;
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use std::collections::HashMap;

use move_deps::move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};

/// This struct represents the aggregated BLS signature representation that contains an aggregated
/// BLS signature and a bit mask representing the set of validators participating in the signing
/// process
#[derive(
    Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash,
)]
pub struct MultiSignature {
    validator_bitmask: Vec<bool>,
    multi_sig: Option<bls12381::Signature>,
}

impl MultiSignature {
    pub fn new(
        validator_bitmask: Vec<bool>,
        aggregated_signature: Option<bls12381::Signature>,
    ) -> Self {
        Self {
            validator_bitmask,
            multi_sig: aggregated_signature,
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn get_voters_bitmap(&self) -> &Vec<bool> {
        &self.validator_bitmask
    }

    pub fn get_voter_addresses(
        &self,
        validator_addresses: &Vec<AccountAddress>,
    ) -> Vec<AccountAddress> {
        self.validator_bitmask
            .iter()
            .zip(validator_addresses)
            .filter_map(|(voted, address)| if *voted { Some(*address) } else { None })
            .collect()
    }

    pub fn get_num_voters(&self) -> usize {
        self.validator_bitmask.iter().filter(|x| **x).count()
    }
    pub fn multi_sig(&self) -> &Option<bls12381::Signature> {
        &self.multi_sig
    }
}

/// Contains the ledger info and partially aggregated signature from a set of validators, this data
/// is only used during the aggregating the votes from different validators and is not persisted in
/// DB.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PartialSignatures {
    signatures: HashMap<AccountAddress, bls12381::Signature>,
}

impl PartialSignatures {
    pub fn new(signatures: HashMap<AccountAddress, bls12381::Signature>) -> Self {
        Self { signatures }
    }

    pub fn empty() -> Self {
        Self::new(HashMap::new())
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

    pub fn signatures(&self) -> &HashMap<AccountAddress, bls12381::Signature> {
        &self.signatures
    }
}
