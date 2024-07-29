// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{assert_success, build_package, tests::common, MoveHarness};
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{hash::CryptoHash, SigningKey};
use aptos_language_e2e_tests::account::{Account, AccountPublicKey, TransactionBuilder};
use aptos_types::{
    keyless::{
        test_utils::{
            get_groth16_sig_and_pk_for_upgraded_vk, get_sample_esk, get_sample_groth16_sig_and_pk,
            get_sample_iss, get_sample_jwk, get_sample_openid_sig_and_pk, get_upgraded_vk,
        },
        Configuration, EphemeralCertificate, Groth16VerificationKey, KeylessPublicKey,
        KeylessSignature, TransactionAndProof,
    },
    on_chain_config::FeatureFlag,
    transaction::{
        authenticator::{AnyPublicKey, AuthenticationKey, EphemeralSignature},
        Script, SignedTransaction, Transaction, TransactionStatus,
    },
};
use move_core_types::{
    account_address::AccountAddress,
    transaction_argument::TransactionArgument,
    vm_status::{StatusCode, StatusCode::FEATURE_UNDER_GATING},
};

fn init_feature_gating(
    enabled_features: Vec<FeatureFlag>,
    disabled_features: Vec<FeatureFlag>,
) -> (MoveHarness, Account, Account) {
    let mut h = MoveHarness::new_with_features(enabled_features, disabled_features);

    let recipient = h.new_account_at(AccountAddress::from_hex_literal("0xb0b").unwrap());

    // initialize JWKs
    let core_resources = run_jwk_and_config_script(&mut h);

    (h, recipient, core_resources)
}

fn test_feature_gating(
    h: &mut MoveHarness,
    recipient: &Account,
    get_sig_and_pk: fn() -> (KeylessSignature, KeylessPublicKey),
    should_succeed: bool,
) {
    let (sig, pk) = get_sig_and_pk();

    let transaction = create_and_spend_keyless_account(h, sig, pk, *recipient.address());
    let output = h.run_raw(transaction);

    if !should_succeed {
        match output.status() {
            TransactionStatus::Discard(status) => {
                assert_eq!(
                    *status, FEATURE_UNDER_GATING,
                    "Expected TransactionStatus::Discard to be FEATURE_UNDER_GATING, but got: {:?}",
                    status
                )
            },
            _ => {
                panic!(
                    "Expected to get a TransactionStatus::Discard, but got: {:?}",
                    output.status()
                )
            },
        }
    } else {
        assert_success!(
            output.status().clone(),
            "Expected TransactionStatus::Keep(ExecutionStatus::Success), but got: {:?}",
            output.status()
        );
    }
}

#[test]
fn test_rotate_vk() {
    let (mut h, recipient, core_resources) = init_feature_gating(
        vec![
            FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES,
            FeatureFlag::BN254_STRUCTURES,
            FeatureFlag::KEYLESS_ACCOUNTS,
        ],
        vec![],
    );

    // Old proof for old VK
    let (old_sig, pk) = get_sample_groth16_sig_and_pk();
    let account = create_keyless_account(&mut h, pk.clone());
    let transaction =
        spend_keyless_account(&mut h, old_sig.clone(), &account, *recipient.address());
    let output = h.run_raw(transaction);
    assert_success!(output.status().clone());

    // New proof for old VK
    let (new_sig, _) = get_groth16_sig_and_pk_for_upgraded_vk();
    let transaction =
        spend_keyless_account(&mut h, new_sig.clone(), &account, *recipient.address());
    let output = h.run_raw(transaction);
    //println!("TXN status: {:?}", output.status());
    match output.status() {
        TransactionStatus::Discard(sc) => assert_eq!(*sc, StatusCode::INVALID_SIGNATURE),
        TransactionStatus::Keep(es) => {
            panic!("Expected TransactionStatus::Discard, got Keep({:?})", es)
        },
        TransactionStatus::Retry => panic!("Expected TransactionStatus::Discard, got Retry"),
    }

    // Upgrade the VK
    run_upgrade_vk_script(
        &mut h,
        core_resources,
        Groth16VerificationKey::from(get_upgraded_vk()),
    );

    // New proof for new VK
    let transaction = spend_keyless_account(&mut h, new_sig, &account, *recipient.address());
    let output = h.run_raw(transaction);
    assert_success!(output.status().clone());

    // Old proof for old VK
    let transaction = spend_keyless_account(&mut h, old_sig, &account, *recipient.address());
    let output = h.run_raw(transaction);
    // println!("TXN status: {:?}", output.status());
    match output.status() {
        TransactionStatus::Discard(sc) => assert_eq!(*sc, StatusCode::INVALID_SIGNATURE),
        TransactionStatus::Keep(es) => {
            panic!("Expected TransactionStatus::Discard, got Keep({:?})", es)
        },
        TransactionStatus::Retry => panic!("Expected TransactionStatus::Discard, got Retry"),
    }
}

#[test]
fn test_feature_gating_with_zk_on() {
    //
    // ZK & ZKless
    let (mut h, recipient, _) = init_feature_gating(
        vec![
            FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES,
            FeatureFlag::BN254_STRUCTURES,
            FeatureFlag::KEYLESS_ACCOUNTS,
            FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS,
        ],
        vec![],
    );
    // Groth16-based sig => success
    test_feature_gating(&mut h, &recipient, get_sample_groth16_sig_and_pk, true);
    // OIDC-based sig => success
    test_feature_gating(&mut h, &recipient, get_sample_openid_sig_and_pk, true);

    //
    // ZK & !ZKless
    let (mut h, recipient, _) = init_feature_gating(
        vec![
            FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES,
            FeatureFlag::BN254_STRUCTURES,
            FeatureFlag::KEYLESS_ACCOUNTS,
        ],
        vec![FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS],
    );
    // Groth16-based sig => success
    test_feature_gating(&mut h, &recipient, get_sample_groth16_sig_and_pk, true);
    // OIDC-based sig => discard
    test_feature_gating(&mut h, &recipient, get_sample_openid_sig_and_pk, false);
}

#[test]
fn test_feature_gating_with_zk_off() {
    //
    // !ZK & ZKless
    let (mut h, recipient, _) = init_feature_gating(
        vec![
            FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES,
            FeatureFlag::BN254_STRUCTURES,
            FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS,
        ],
        vec![FeatureFlag::KEYLESS_ACCOUNTS],
    );
    // Groth16-based sig => discard
    test_feature_gating(&mut h, &recipient, get_sample_groth16_sig_and_pk, false);
    // OIDC-based sig => success
    test_feature_gating(&mut h, &recipient, get_sample_openid_sig_and_pk, true);

    //
    // !ZK & !ZKless
    let (mut h, recipient, _) = init_feature_gating(
        vec![
            FeatureFlag::CRYPTOGRAPHY_ALGEBRA_NATIVES,
            FeatureFlag::BN254_STRUCTURES,
        ],
        vec![
            FeatureFlag::KEYLESS_ACCOUNTS,
            FeatureFlag::KEYLESS_BUT_ZKLESS_ACCOUNTS,
        ],
    );
    // Groth16-based sig => discard
    test_feature_gating(&mut h, &recipient, get_sample_groth16_sig_and_pk, false);
    // OIDC-based sig => discard
    test_feature_gating(&mut h, &recipient, get_sample_openid_sig_and_pk, false);
}

fn create_keyless_account(h: &mut MoveHarness, pk: KeylessPublicKey) -> Account {
    let apk = AnyPublicKey::keyless(pk.clone());
    let addr = AuthenticationKey::any_key(apk.clone()).account_address();
    let account = h.store_and_fund_account(
        &Account::new_from_addr(addr, AccountPublicKey::Keyless(pk)),
        100000000,
        0,
    );

    println!("Actual address: {}", addr.to_hex());
    println!("Account address: {}", account.address().to_hex());

    account
}

fn spend_keyless_account(
    h: &mut MoveHarness,
    mut sig: KeylessSignature,
    account: &Account,
    recipient: AccountAddress,
) -> SignedTransaction {
    let payload = aptos_stdlib::aptos_coin_transfer(recipient, 1);
    //println!("Payload: {:?}", payload);
    let raw_txn = TransactionBuilder::new(account.clone())
        .payload(payload)
        .sequence_number(h.sequence_number(account.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .raw();

    println!("RawTxn sender: {:?}", raw_txn.sender());

    let mut txn_and_zkp = TransactionAndProof {
        message: raw_txn.clone(),
        proof: None,
    };
    let esk = get_sample_esk();

    match &mut sig.cert {
        EphemeralCertificate::ZeroKnowledgeSig(proof) => {
            // Training wheels should be disabled.
            proof.training_wheels_signature = None;
            txn_and_zkp.proof = Some(proof.proof);
        },
        EphemeralCertificate::OpenIdSig(_) => {},
    }
    sig.ephemeral_signature = EphemeralSignature::ed25519(esk.sign(&txn_and_zkp).unwrap());

    let transaction =
        SignedTransaction::new_keyless(raw_txn, account.pubkey.as_keyless().unwrap(), sig);
    println!(
        "Submitted TXN hash: {}",
        Transaction::UserTransaction(transaction.clone()).hash()
    );
    transaction
}

/// Creates and funds a new account at `pk` and sends coins to `recipient`.
fn create_and_spend_keyless_account(
    h: &mut MoveHarness,
    sig: KeylessSignature,
    pk: KeylessPublicKey,
    recipient: AccountAddress,
) -> SignedTransaction {
    let account = create_keyless_account(h, pk.clone());

    spend_keyless_account(h, sig, &account, recipient)
}

fn run_jwk_and_config_script(h: &mut MoveHarness) -> Account {
    let core_resources = h.new_account_at(AccountAddress::from_hex_literal("0xA550C18").unwrap());

    let package = build_package(
        common::test_dir_path("keyless_setup.data/pack"),
        aptos_framework::BuildOptions::default(),
    )
    .expect("building package must succeed");

    let txn = h.create_publish_built_package(&core_resources, &package, |_| {});
    assert_success!(h.run(txn));

    let script = package.extract_script_code()[0].clone();

    let iss = get_sample_iss();
    let jwk = get_sample_jwk();
    let config = Configuration::new_for_testing();

    let txn = TransactionBuilder::new(core_resources.clone())
        .script(Script::new(script, vec![], vec![
            TransactionArgument::U8Vector(iss.into_bytes()),
            TransactionArgument::U8Vector(jwk.kid.into_bytes()),
            TransactionArgument::U8Vector(jwk.alg.into_bytes()),
            TransactionArgument::U8Vector(jwk.e.into_bytes()),
            TransactionArgument::U8Vector(jwk.n.into_bytes()),
            TransactionArgument::U64(config.max_exp_horizon_secs),
        ]))
        .sequence_number(h.sequence_number(core_resources.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    // NOTE: We cannot write the Configuration and Groth16Verification key via MoveHarness::set_resource
    // because it does not (yet) work with resource groups.

    assert_success!(h.run(txn));

    core_resources
}

fn run_upgrade_vk_script(h: &mut MoveHarness, core_resources: Account, vk: Groth16VerificationKey) {
    let package = build_package(
        common::test_dir_path("keyless_new_vk.data/pack"),
        aptos_framework::BuildOptions::default(),
    )
    .expect("building package must succeed");

    let txn = h.create_publish_built_package(&core_resources, &package, |_| {});
    assert_success!(h.run(txn));

    let script = package.extract_script_code()[0].clone();

    let txn = TransactionBuilder::new(core_resources.clone())
        .script(Script::new(script, vec![], vec![
            TransactionArgument::U8Vector(vk.alpha_g1),
            TransactionArgument::U8Vector(vk.beta_g2),
            TransactionArgument::U8Vector(vk.gamma_g2),
            TransactionArgument::U8Vector(vk.delta_g2),
            TransactionArgument::U8Vector(vk.gamma_abc_g1[0].clone()),
            TransactionArgument::U8Vector(vk.gamma_abc_g1[1].clone()),
        ]))
        .sequence_number(h.sequence_number(core_resources.address()))
        .max_gas_amount(1_000_000)
        .gas_unit_price(1)
        .sign();

    // NOTE: We cannot write the Groth16Verification key via MoveHarness::set_resource
    // because it does not (yet) work with resource groups.

    assert_success!(h.run(txn));
}
