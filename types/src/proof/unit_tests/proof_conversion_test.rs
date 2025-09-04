// Copyright © Velor Foundation
// Parts of the project are originally copyright © Meta Platforms, Inc.
// SPDX-License-Identifier: Apache-2.0

use crate::proof::{
    definition::{TransactionInfoListWithProof, TransactionInfoWithProof},
    AccumulatorConsistencyProof, SparseMerkleRangeProof, TestAccumulatorProof,
    TestAccumulatorRangeProof,
};
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;

type SparseMerkleProof = crate::proof::SparseMerkleProof;

proptest! {


    #[test]
    fn test_accumulator_bcs_roundtrip(proof in any::<TestAccumulatorProof>()) {
        assert_canonical_encode_decode(proof);
    }


    #[test]
    fn test_sparse_merkle_bcs_roundtrip(proof in any::<SparseMerkleProof>()) {
        assert_canonical_encode_decode(proof);
    }


    #[test]
    fn test_accumulator_consistency_bcs_roundtrip(
        proof in any::<AccumulatorConsistencyProof>(),
    ) {
        assert_canonical_encode_decode(proof);
    }


    #[test]
    fn test_accumulator_range_bcs_roundtrip(
        proof in any::<TestAccumulatorRangeProof>(),
    ) {
        assert_canonical_encode_decode(proof);
    }


    #[test]
    fn test_sparse_merkle_range_bcs_roundtrip(
        proof in any::<SparseMerkleRangeProof>(),
    ) {
        assert_canonical_encode_decode(proof);
    }


    #[test]
    fn test_transaction_proof_bcs_roundtrip(proof in any::<TransactionInfoWithProof>()) {
        assert_canonical_encode_decode(proof);
    }


    #[test]
    fn test_transaction_list_proof_bcs_roundtrip(proof in any::<TransactionInfoListWithProof>()) {
        assert_canonical_encode_decode(proof);
    }
}
