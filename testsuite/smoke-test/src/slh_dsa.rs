// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::smoke_test_environment::SwarmBuilder;
use aptos_cached_packages::aptos_stdlib;
use aptos_crypto::{
    slh_dsa_sha2_128s::{PrivateKey, PublicKey},
    traits::Uniform,
};
use aptos_forge::{AptosPublicInfo, LocalSwarm, Swarm};
use aptos_logger::{debug, info};
use aptos_types::{
    on_chain_config::{FeatureFlag, Features},
    transaction::authenticator::{AnyPublicKey, AuthenticationKey},
};
use rand::rngs::OsRng;
use std::sync::Arc;

#[tokio::test]
async fn test_slh_dsa_feature_disabled() {
    slh_dsa_scenario(false).await
}

#[tokio::test]
async fn test_slh_dsa_feature_enabled() {
    slh_dsa_scenario(true).await
}

/// Config the chain, run an SLH-DSA txn, and assert txn result.
async fn slh_dsa_scenario(enable_feature: bool) {
    let (swarm, mut info) = setup_local_net(enable_feature).await;

    // Generate SLH-DSA keypair
    let mut rng = OsRng;
    let private_key = PrivateKey::generate(&mut rng);
    let public_key: PublicKey = (&private_key).into();

    // Create account address from public key
    let auth_key = AuthenticationKey::any_key(AnyPublicKey::slh_dsa_sha2_128s(public_key.clone()));
    let account_address = auth_key.account_address();

    info!(
        "SLH-DSA account address: {}",
        account_address.to_hex_literal()
    );

    // Create the account on-chain
    info.sync_root_account_sequence_number().await;
    info.create_user_account_with_any_key(&AnyPublicKey::slh_dsa_sha2_128s(public_key.clone()))
        .await
        .unwrap();
    info.sync_root_account_sequence_number().await;

    // Fund the account
    info.mint(account_address, 10_000_000_000).await.unwrap();
    info.sync_root_account_sequence_number().await;

    // Create a recipient account
    let recipient = info
        .create_and_fund_user_account(20_000_000_000)
        .await
        .unwrap();

    // Create and sign a transaction
    let raw_txn = info
        .transaction_factory()
        .payload(aptos_stdlib::aptos_coin_transfer(
            recipient.address(),
            1_000_000,
        ))
        .sender(account_address)
        .sequence_number(0)
        .build();

    let signed_txn = raw_txn
        .sign_slh_dsa_sha2_128s(&private_key, public_key)
        .unwrap()
        .into_inner();

    // Submit the transaction
    info!(
        "Submitting SLH-DSA transaction (feature enabled: {})",
        enable_feature
    );
    let result = swarm
        .aptos_public_info()
        .client()
        .submit_without_deserializing_response(&signed_txn)
        .await;

    debug!("result={:?}", result);

    if enable_feature {
        if let Err(e) = result {
            panic!(
                "SLH-DSA transaction should have succeeded when feature is enabled, but got error: {:?}",
                e
            );
        }
    } else {
        if result.is_ok() {
            panic!("SLH-DSA transaction should have failed with FEATURE_UNDER_GATING when feature is disabled");
        }

        // Verify the error is FEATURE_UNDER_GATING
        let error = result.unwrap_err();
        let error_msg = format!("{:?}", error);
        assert!(
            error_msg.contains("FEATURE_UNDER_GATING"),
            "Expected FEATURE_UNDER_GATING error, but got: {:?}",
            error
        );
    }
}

async fn setup_local_net(enable_slh_dsa: bool) -> (LocalSwarm, AptosPublicInfo) {
    let (swarm, mut _cli, _faucet) = SwarmBuilder::new_local(1)
        .with_init_genesis_config(Arc::new(move |conf| {
            let mut features = Features::default();
            if enable_slh_dsa {
                features.enable(FeatureFlag::SLH_DSA_SHA2_128S_SIGNATURE);
            } else {
                features.disable(FeatureFlag::SLH_DSA_SHA2_128S_SIGNATURE);
            }
            conf.initial_features_override = Some(features);
        }))
        .with_aptos()
        .build_with_cli(0)
        .await;

    let info = swarm.aptos_public_info();
    (swarm, info)
}
