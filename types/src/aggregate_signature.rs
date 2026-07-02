// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::lazy_bls::LazyBlsSignature;
use aptos_bitvec::BitVec;
use aptos_crypto::{bls12381, CryptoMaterialError};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::account_address::AccountAddress;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// This struct represents a BLS multi-signature or aggregated signature:
/// it stores a bit mask representing the set of validators participating in the signing process
/// and the multi-signature/aggregated signature itself,
/// which was aggregated from these validators' partial BLS signatures.
///
/// The aggregated signature is stored lazily as compressed bytes
/// ([`LazyBlsSignature`]): its G2 point is only decompressed when actually
/// needed for verification (via [`AggregateSignature::decompressed_sig`]), after
/// cheaper structural checks have run. This bounds the per-message CPU work a
/// peer-supplied payload can force on the receiver. The on-wire encoding is
/// byte-identical to storing a `bls12381::Signature` directly.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct AggregateSignature {
    validator_bitmask: BitVec,
    sig: Option<LazyBlsSignature>,
}

impl AggregateSignature {
    pub fn new(
        validator_bitmask: BitVec,
        aggregated_signature: Option<bls12381::Signature>,
    ) -> Self {
        Self {
            validator_bitmask,
            sig: aggregated_signature
                .as_ref()
                .map(LazyBlsSignature::from_signature),
        }
    }

    pub fn empty() -> Self {
        Self {
            validator_bitmask: BitVec::default(),
            sig: None,
        }
    }

    pub fn get_signers_bitvec(&self) -> &BitVec {
        &self.validator_bitmask
    }

    pub fn get_signers_addresses(
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

    /// The aggregated signature in its lazy, still-compressed form. Callers that
    /// only need the raw bytes (e.g. API hex export) can use this without paying
    /// for decompression.
    pub fn sig(&self) -> &Option<LazyBlsSignature> {
        &self.sig
    }

    /// Decompress the aggregated signature into a `bls12381::Signature`,
    /// performing the deferred G2-point decompression. Returns `Ok(None)` if no
    /// signature is present. This is the verification entry point — call it only
    /// after cheaper structural checks (bitmask, voting power) have passed.
    pub fn decompressed_sig(&self) -> Result<Option<bls12381::Signature>, CryptoMaterialError> {
        self.sig.as_ref().map(|s| s.decompress()).transpose()
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

    pub fn remove_signature(&mut self, validator: AccountAddress) -> Option<bls12381::Signature> {
        self.signatures.remove(&validator)
    }

    pub fn add_signature(&mut self, validator: AccountAddress, signature: bls12381::Signature) {
        self.signatures.insert(validator, signature);
    }

    pub fn unpack(self) -> BTreeMap<AccountAddress, bls12381::Signature> {
        self.signatures
    }

    pub fn signatures_iter(&self) -> impl Iterator<Item = (&AccountAddress, &bls12381::Signature)> {
        self.signatures.iter()
    }

    pub fn signatures(&self) -> &BTreeMap<AccountAddress, bls12381::Signature> {
        &self.signatures
    }

    pub fn contains_voter(&self, voter: &AccountAddress) -> bool {
        self.signatures.contains_key(voter)
    }
}
