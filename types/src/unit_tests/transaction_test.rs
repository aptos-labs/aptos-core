// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    account_address::AccountAddress,
    chain_id::ChainId,
    state_store::state_key::StateKey,
    transaction::{
        AccountOrderedTransactionsWithProof, RawTransaction, Script, SignedTransaction,
        Transaction, TransactionInfo, TransactionListWithProof, TransactionOutput,
        TransactionOutputListWithAuxiliaryInfosAndHotness, TransactionOutputListWithProof,
        TransactionOutputListWithProofV2, TransactionPayload, TransactionWithProof,
    },
    write_set::WriteSet,
};
use aptos_crypto::{
    ed25519::{self, Ed25519PrivateKey, Ed25519Signature},
    hash::HashValue,
    PrivateKey, Uniform,
};
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    convert::TryFrom,
};

#[test]
fn test_invalid_signature() {
    let txn: SignedTransaction = SignedTransaction::new(
        RawTransaction::new_script(
            AccountAddress::random(),
            0,
            Script::new(vec![], vec![], vec![]),
            0,
            0,
            0,
            ChainId::test(),
        ),
        Ed25519PrivateKey::generate_for_testing().public_key(),
        Ed25519Signature::try_from(&[1u8; 64][..]).unwrap(),
    );
    assert!(
        txn.verify_signature().is_err(),
        "Signature checking should fail"
    )
}

proptest! {
    #[test]
    fn test_sign_raw_transaction(raw_txn in any::<RawTransaction>(), keypair in ed25519::keypair_strategy()) {
        let txn = raw_txn.sign(&keypair.private_key, keypair.public_key).unwrap();
        let signed_txn = txn.into_inner();
        assert!(signed_txn.check_signature().is_ok());
    }

    #[test]
    fn transaction_payload_bcs_roundtrip(txn_payload in any::<TransactionPayload>()) {
        assert_canonical_encode_decode(txn_payload);
    }

    #[test]
    fn raw_transaction_bcs_roundtrip(raw_txn in any::<RawTransaction>()) {
        assert_canonical_encode_decode(raw_txn);
    }

    #[test]
    fn signed_transaction_bcs_roundtrip(signed_txn in any::<SignedTransaction>()) {
        assert_canonical_encode_decode(signed_txn);
    }

    #[test]
    fn transaction_info_bcs_roundtrip(txn_info in any::<TransactionInfo>()) {
        assert_canonical_encode_decode(txn_info);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]

    #[test]
    fn transaction_list_with_proof_bcs_roundtrip(txn_list in any::<TransactionListWithProof>()) {
        assert_canonical_encode_decode(txn_list);
    }

    #[test]
    fn transaction_bcs_roundtrip(txn in any::<Transaction>()) {
        assert_canonical_encode_decode(txn);
    }

    #[test]
    fn transaction_with_proof_bcs_roundtrip(txn_with_proof in any::<TransactionWithProof>()) {
        assert_canonical_encode_decode(txn_with_proof);
    }

    #[test]
    fn acct_txns_with_proof_bcs_roundtrip(acct_txns_with_proof in any::<AccountOrderedTransactionsWithProof>()) {
        assert_canonical_encode_decode(acct_txns_with_proof);
    }
}

#[test]
fn transaction_output_list_v2_hotness_roundtrip() {
    // Two outputs: only the second carries hotness keys. This mirrors the typical case
    // where hotness only lives in the block epilogue at the end of a chunk.
    let txn0 = Transaction::StateCheckpoint(HashValue::zero());
    let output0 = TransactionOutput::new_success_with_write_set(WriteSet::default());

    let txn1 = Transaction::StateCheckpoint(HashValue::random());
    let output1 = TransactionOutput::new_success_with_write_set(WriteSet::default());

    let output_list = TransactionOutputListWithProof::new(
        vec![(txn0, output0), (txn1, output1)],
        Some(0),
        crate::proof::TransactionInfoListWithProof::new_empty(),
    );

    let hot_keys: BTreeSet<StateKey> =
        vec![StateKey::raw(b"hot_key_a"), StateKey::raw(b"hot_key_b")]
            .into_iter()
            .collect();
    let mut hotness = BTreeMap::new();
    hotness.insert(1u32, hot_keys.clone());

    let v2 = TransactionOutputListWithProofV2::new_with_hotness(
        TransactionOutputListWithAuxiliaryInfosAndHotness::new(
            output_list,
            vec![
                crate::transaction::PersistedAuxiliaryInfo::None,
                crate::transaction::PersistedAuxiliaryInfo::None,
            ],
            hotness,
        ),
    );

    // Round-trip through BCS (simulating the wire). `WriteSet::Serialize` drops in-WriteSet
    // hotness, so the sidecar map is the only carrier across the boundary.
    let encoded = bcs::to_bytes(&v2).expect("v2 BCS encode");
    let decoded: TransactionOutputListWithProofV2 =
        bcs::from_bytes(&encoded).expect("v2 BCS decode");

    // After consuming the V2, the sidecar should be spliced back into the right output's
    // WriteSet, and only that output.
    let (output_list, _aux) = decoded.into_parts();
    let outputs = &output_list.transactions_and_outputs;
    assert_eq!(outputs.len(), 2);
    assert_eq!(outputs[0].1.write_set().hotness_keys().count(), 0);
    let restored: BTreeSet<StateKey> = outputs[1].1.write_set().hotness_keys().cloned().collect();
    assert_eq!(restored, hot_keys);
}

#[test]
fn transaction_output_list_v2_hotness_out_of_range_ignored() {
    // Hotness entries pointing past the output list are silently dropped; hotness is best-
    // effort and unverified, so a malformed peer must not be able to crash decode.
    let txn = Transaction::StateCheckpoint(HashValue::zero());
    let output = TransactionOutput::new_success_with_write_set(WriteSet::default());

    let output_list = TransactionOutputListWithProof::new(
        vec![(txn, output)],
        Some(0),
        crate::proof::TransactionInfoListWithProof::new_empty(),
    );

    let mut hotness = BTreeMap::new();
    hotness.insert(7u32, vec![StateKey::raw(b"bogus")].into_iter().collect());

    let v2 = TransactionOutputListWithProofV2::new_with_hotness(
        TransactionOutputListWithAuxiliaryInfosAndHotness::new(
            output_list,
            vec![crate::transaction::PersistedAuxiliaryInfo::None],
            hotness,
        ),
    );

    let encoded = bcs::to_bytes(&v2).unwrap();
    let decoded: TransactionOutputListWithProofV2 = bcs::from_bytes(&encoded).unwrap();
    let (output_list, _) = decoded.into_parts();
    assert_eq!(
        output_list.transactions_and_outputs[0]
            .1
            .write_set()
            .hotness_keys()
            .count(),
        0
    );
}
