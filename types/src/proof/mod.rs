// Copyright © Aptos Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

pub mod accumulator;
pub mod definition;
pub mod position;
#[cfg(any(test, feature = "fuzzing"))]
pub mod proptest_proof;

#[cfg(test)]
mod unit_tests;

pub use self::definition::{
    AccumulatorConsistencyProof, AccumulatorExtensionProof, AccumulatorProof,
    AccumulatorRangeProof, SparseMerkleProof, SparseMerkleProofExt, SparseMerkleRangeProof,
    TransactionAccumulatorProof, TransactionAccumulatorRangeProof, TransactionAccumulatorSummary,
    TransactionInfoListWithProof, TransactionInfoWithProof,
};
#[cfg(any(test, feature = "fuzzing"))]
pub use self::definition::{TestAccumulatorProof, TestAccumulatorRangeProof};
use crate::{
    ledger_info::LedgerInfo,
    transaction::{TransactionInfo, Version},
};
use anyhow::{ensure, Result};
use aptos_crypto::{
    hash::{
        CryptoHash, CryptoHasher, EventAccumulatorHasher, SparseMerkleInternalHasher,
        TestOnlyHasher, TransactionAccumulatorHasher,
    },
    HashValue,
};
use aptos_crypto_derive::CryptoHasher;
#[cfg(any(test, feature = "fuzzing"))]
use proptest_derive::Arbitrary;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

/// Verifies that a given `transaction_info` exists in the ledger using provided proof.
fn verify_transaction_info(
    ledger_info: &LedgerInfo,
    transaction_version: Version,
    transaction_info: &TransactionInfo,
    ledger_info_to_transaction_info_proof: &TransactionAccumulatorProof,
) -> Result<()> {
    ensure!(
        transaction_version <= ledger_info.version(),
        "Transaction version {} is newer than LedgerInfo version {}.",
        transaction_version,
        ledger_info.version(),
    );

    let transaction_info_hash = transaction_info.hash();
    ledger_info_to_transaction_info_proof.verify(
        ledger_info.transaction_accumulator_hash(),
        transaction_info_hash,
        transaction_version,
    )?;

    Ok(())
}

pub struct MerkleTreeInternalNode<H> {
    left_child: HashValue,
    right_child: HashValue,
    hasher: PhantomData<H>,
}

impl<H: CryptoHasher> MerkleTreeInternalNode<H> {
    pub fn new(left_child: HashValue, right_child: HashValue) -> Self {
        Self {
            left_child,
            right_child,
            hasher: PhantomData,
        }
    }
}

impl<H: CryptoHasher> CryptoHash for MerkleTreeInternalNode<H> {
    type Hasher = H;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(self.left_child.as_ref());
        state.update(self.right_child.as_ref());
        state.finish()
    }
}

pub type SparseMerkleInternalNode = MerkleTreeInternalNode<SparseMerkleInternalHasher>;
pub type TransactionAccumulatorInternalNode = MerkleTreeInternalNode<TransactionAccumulatorHasher>;
pub type EventAccumulatorInternalNode = MerkleTreeInternalNode<EventAccumulatorHasher>;
pub type TestAccumulatorInternalNode = MerkleTreeInternalNode<TestOnlyHasher>;

#[derive(Clone, Copy, CryptoHasher, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "fuzzing"), derive(Arbitrary))]
pub struct SparseMerkleLeafNode {
    key: HashValue,
    value_hash: HashValue,
}

impl SparseMerkleLeafNode {
    pub fn new(key: HashValue, value_hash: HashValue) -> Self {
        SparseMerkleLeafNode { key, value_hash }
    }

    pub fn key(&self) -> &HashValue {
        &self.key
    }

    pub fn value_hash(&self) -> &HashValue {
        &self.value_hash
    }

    pub fn calc_hash(&self) -> HashValue {
        CryptoHash::hash(self)
    }
}

impl CryptoHash for SparseMerkleLeafNode {
    type Hasher = SparseMerkleLeafNodeHasher;

    fn hash(&self) -> HashValue {
        let mut state = Self::Hasher::default();
        state.update(self.key.as_ref());
        state.update(self.value_hash.as_ref());
        state.finish()
    }
}
