// Copyright (c) Aptos
// SPDX-License-Identifier: Apache-2.0

use crate::{
    account_address::AccountAddress,
    chain_id::ChainId,
    transaction::{
        AccountTransactionsWithProof, RawTransaction, Script, SignedTransaction, Transaction,
        TransactionInfo, TransactionListWithProof, TransactionPayload, TransactionWithProof,
    },
};
use aptos_crypto::{
    ed25519::{self, Ed25519PrivateKey, Ed25519Signature},
    PrivateKey, Uniform,
};
use bcs::test_helpers::assert_canonical_encode_decode;
use proptest::prelude::*;
use std::convert::TryFrom;

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
    assert!(!txn.signature_is_valid(), "Signature checking should fail")
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
    fn acct_txns_with_proof_bcs_roundtrip(acct_txns_with_proof in any::<AccountTransactionsWithProof>()) {
        assert_canonical_encode_decode(acct_txns_with_proof);
    }
}
