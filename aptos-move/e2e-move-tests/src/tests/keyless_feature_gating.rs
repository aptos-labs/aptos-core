// Copyright © Aptos Foundation

use crate::MoveHarness;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::SigningKey;
use aptos_language_e2e_tests::account::{Account, TransactionBuilder};
use aptos_types::{
    keyless::{test_utils::get_sample_groth16_sig_and_pk, KeylessPublicKey, KeylessSignature},
    on_chain_config::FeatureFlag,
    transaction::{
        authenticator::{AnyPublicKey, AuthenticationKey},
        SignedTransaction, TransactionStatus,
    },
};
use aptos_types::keyless::test_utils::{get_sample_esk, get_sample_jwk};
use aptos_types::keyless::{Configuration, get_public_inputs_hash, Groth16ZkpAndStatement, ZkpOrOpenIdSig};
use aptos_types::transaction::authenticator::EphemeralSignature;
use move_core_types::{
    account_address::AccountAddress, vm_status::StatusCode::FEATURE_UNDER_GATING,
};
use ark_ff::{BigInteger, PrimeField};

// TODO(keyless): Initialize keyless_account.move

#[test]
fn test_keyless_feature_gating() {
    let mut h = MoveHarness::new_with_features(vec![], vec![FeatureFlag::KEYLESS_ACCOUNTS]);

    let (sig, pk) = get_sample_groth16_sig_and_pk();
    let bob = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    let transaction = get_keyless_signed_txn(&mut h, sig, pk, bob);

    let output = h.run_raw(transaction);
    match output.status() {
        TransactionStatus::Discard(status) => {
            assert_eq!(*status, FEATURE_UNDER_GATING)
        },
        _ => {
            panic!("Expected to get FEATURE_UNDER_GATING DiscardedVMStatus")
        },
    }
}

/// Creates and funds a new account at `pk` and sends coins to `recipient`.
fn get_keyless_signed_txn(
    h: &mut MoveHarness,
    mut sig: KeylessSignature,
    pk: KeylessPublicKey,
    recipient: Account,
) -> SignedTransaction {
    let apk = AnyPublicKey::keyless(pk.clone());
    let addr = AuthenticationKey::any_key(apk.clone()).account_address();
    let account = h.store_and_fund_account(&Account::new_from_addr(addr), 100000000, 0);

    let payload = aptos_stdlib::aptos_coin_transfer(*recipient.address(), 1);
    let raw_txn = TransactionBuilder::new(account.clone())
        .payload(payload)
        .sequence_number(h.sequence_number(account.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign() // dummy signature, which we discard below
        .into_raw_transaction();

    let esk = get_sample_esk();
    sig.ephemeral_signature = EphemeralSignature::ed25519(esk.sign(&raw_txn).unwrap());
    let jwk = get_sample_jwk();
    // TODO: might need new_for_devnet() here; forget how the MoveHarness is initialized
    let config = Configuration::new_for_testing();

    let public_inputs_hash: Option<[u8; 32]> = if let ZkpOrOpenIdSig::Groth16Zkp(_) = &sig.sig {
        // This will only calculate the hash if it's needed, avoiding unnecessary computation.
        Some(
            get_public_inputs_hash(&sig, &pk, &jwk, &config)
                .unwrap()
                .into_bigint()
                .to_bytes_le()
                .try_into()
                .expect("expected 32-byte public inputs hash"),
        )
    } else {
        None
    };

    // Compute the training wheels signature if not present
    match &mut sig.sig {
        ZkpOrOpenIdSig::Groth16Zkp(proof) => {
            // Training wheels should be disabled.
            proof.training_wheels_signature = None
        },
        ZkpOrOpenIdSig::OpenIdSig(_) => {},
    }

    let transaction = SignedTransaction::new_keyless(raw_txn, pk, sig);
    transaction
}
