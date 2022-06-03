// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{corpus_from_strategy, fuzz_data_to_value, FuzzTargetImpl};
use aptos_crypto::HashValue;
use aptos_proptest_helpers::ValueGenerator;
use aptos_types::{
    ledger_info::LedgerInfo,
    proof::{
        SparseMerkleProof, TestAccumulatorProof, TestAccumulatorRangeProof,
        TransactionInfoListWithProof, TransactionInfoWithProof,
    },
    state_store::state_value::StateValue,
    transaction::Version,
};
use proptest::prelude::*;
use proptest_derive::Arbitrary;

#[derive(Clone, Debug, Default)]
pub struct TestAccumulatorProofFuzzer;

#[derive(Debug, Arbitrary)]
struct TestAccumulatorProofFuzzerInput {
    proof: TestAccumulatorProof,
    expected_root_hash: HashValue,
    element_hash: HashValue,
    element_index: u64,
}

impl FuzzTargetImpl for TestAccumulatorProofFuzzer {
    fn description(&self) -> &'static str {
        "Proof: TestAccumulatorProof"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(corpus_from_strategy(
            any::<TestAccumulatorProofFuzzerInput>(),
        ))
    }

    fn fuzz(&self, data: &[u8]) {
        let input = fuzz_data_to_value(data, any::<TestAccumulatorProofFuzzerInput>());
        let _res = input.proof.verify(
            input.expected_root_hash,
            input.element_hash,
            input.element_index,
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct SparseMerkleProofFuzzer;

#[derive(Debug, Arbitrary)]
struct SparseMerkleProofFuzzerInput {
    proof: SparseMerkleProof,
    expected_root_hash: HashValue,
    element_key: HashValue,
    element_blob: Option<StateValue>,
}

impl FuzzTargetImpl for SparseMerkleProofFuzzer {
    fn description(&self) -> &'static str {
        "Proof: SparseMerkleProof"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(corpus_from_strategy(any::<SparseMerkleProofFuzzerInput>()))
    }

    fn fuzz(&self, data: &[u8]) {
        let input = fuzz_data_to_value(data, any::<SparseMerkleProofFuzzerInput>());
        let _res = input.proof.verify(
            input.expected_root_hash,
            input.element_key,
            input.element_blob.as_ref(),
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct TestAccumulatorRangeProofFuzzer;

#[derive(Debug, Arbitrary)]
struct TestAccumulatorRangeProofFuzzerInput {
    proof: TestAccumulatorRangeProof,
    expected_root_hash: HashValue,
    first_leaf_index: Option<u64>,
    leaf_hashes: Vec<HashValue>,
}

impl FuzzTargetImpl for TestAccumulatorRangeProofFuzzer {
    fn description(&self) -> &'static str {
        "Proof: TestAccumulatorRangeProof"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(corpus_from_strategy(any::<
            TestAccumulatorRangeProofFuzzerInput,
        >()))
    }

    fn fuzz(&self, data: &[u8]) {
        let input = fuzz_data_to_value(data, any::<TestAccumulatorRangeProofFuzzerInput>());
        let _res = input.proof.verify(
            input.expected_root_hash,
            input.first_leaf_index,
            &input.leaf_hashes[..],
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct TransactionInfoWithProofFuzzer;

#[derive(Debug, Arbitrary)]
struct TransactionInfoWithProofFuzzerInput {
    proof: TransactionInfoWithProof,
    ledger_info: LedgerInfo,
    transaction_version: Version,
}

impl FuzzTargetImpl for TransactionInfoWithProofFuzzer {
    fn description(&self) -> &'static str {
        "Proof: TransactionInfoWithProof"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(corpus_from_strategy(any::<
            TransactionInfoWithProofFuzzerInput,
        >()))
    }

    fn fuzz(&self, data: &[u8]) {
        let input = fuzz_data_to_value(data, any::<TransactionInfoWithProofFuzzerInput>());
        let _res = input
            .proof
            .verify(&input.ledger_info, input.transaction_version);
    }
}

#[derive(Clone, Debug, Default)]
pub struct TransactionInfoListWithProofFuzzer;

#[derive(Debug, Arbitrary)]
struct TransactionInfoListWithProofFuzzerInput {
    proof: TransactionInfoListWithProof,
    ledger_info: LedgerInfo,
    first_transaction_version: Option<Version>,
}

impl FuzzTargetImpl for TransactionInfoListWithProofFuzzer {
    fn description(&self) -> &'static str {
        "Proof: TransactionInfoListWithProof"
    }

    fn generate(&self, _idx: usize, _gen: &mut ValueGenerator) -> Option<Vec<u8>> {
        Some(corpus_from_strategy(any::<
            TransactionInfoListWithProofFuzzerInput,
        >()))
    }

    fn fuzz(&self, data: &[u8]) {
        let input = fuzz_data_to_value(data, any::<TransactionInfoListWithProofFuzzerInput>());
        let _res = input
            .proof
            .verify(&input.ledger_info, input.first_transaction_version);
    }
}
