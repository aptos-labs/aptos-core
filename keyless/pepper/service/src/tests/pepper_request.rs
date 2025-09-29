// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dedicated_handlers::pepper_request::handle_pepper_request,
    error::PepperServiceError,
    external_resources::{
        jwk_fetcher, keyless_config::OnChainKeylessConfiguration, resource_fetcher::CachedResources,
    },
    tests::utils,
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_infallible::Mutex;
use aptos_types::{
    account_address::AccountAddress,
    keyless::{
        circuit_testcases::{
            sample_jwt_payload_json_overrides, SAMPLE_EXP_DATE, SAMPLE_JWT_EXTRA_FIELD,
            SAMPLE_NONCE, SAMPLE_TEST_ISS_VALUE, SAMPLE_UID_VAL,
        },
        test_utils::{get_sample_epk_blinder, get_sample_esk, get_sample_jwt_token_from_payload},
        OpenIdSig,
    },
    transaction::authenticator::EphemeralPublicKey,
};
use std::{
    collections::HashMap,
    ops::Deref,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[tokio::test]
async fn request_ephemeral_public_key_expired() {
    // Generate a VUF private key
    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    // Create a JWK cache and resource cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let cached_resources = CachedResources::new_for_testing();

    // Update the keyless config cached resource
    set_on_chain_keyless_configuration(&cached_resources);

    // Create a test JWT
    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        &SAMPLE_JWT_EXTRA_FIELD,
        get_current_time_secs(),
        &SAMPLE_NONCE,
    );
    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    // Handle the pepper request
    let pepper_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        jwt,
        generate_ephemeral_public_key(),
        get_current_time_secs() - 10, // Expiry time in the past
        get_sample_epk_blinder(),
        None,
        None,
        None,
        utils::get_mock_account_recovery_db(),
    )
    .await;

    // Expect an error
    verify_error_string(
        pepper_result,
        "The ephemeral public key expiry date has passed",
    );
}

#[tokio::test]
async fn request_ephemeral_public_key_expiry_too_large() {
    // Generate a VUF private key
    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    // Create a JWK cache and resource cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let cached_resources = CachedResources::new_for_testing();

    // Update the keyless config cached resource
    set_on_chain_keyless_configuration(&cached_resources);

    // Create a test JWT
    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        &SAMPLE_JWT_EXTRA_FIELD,
        get_current_time_secs(),
        &SAMPLE_NONCE,
    );
    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    // Handle the pepper request
    let pepper_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        jwt,
        generate_ephemeral_public_key(),
        u64::MAX / 2, // Large expiry time
        get_sample_epk_blinder(),
        None,
        None,
        None,
        utils::get_mock_account_recovery_db(),
    )
    .await;

    // Expect an error
    verify_error_string(
        pepper_result,
        "The ephemeral public key expiry date is too far in the future",
    );
}

#[tokio::test]
async fn request_invalid_oath_nonce() {
    // Generate a VUF private key
    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    // Create a JWK cache and resource cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let cached_resources = CachedResources::new_for_testing();

    // Update the keyless config cached resource
    set_on_chain_keyless_configuration(&cached_resources);

    // Create a test JWT
    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        &SAMPLE_JWT_EXTRA_FIELD,
        get_current_time_secs(),
        &SAMPLE_NONCE,
    );
    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    // Handle the pepper request
    let pepper_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        jwt,
        generate_ephemeral_public_key(),
        get_current_time_secs() + 10,
        get_sample_epk_blinder(),
        None,
        None,
        None,
        utils::get_mock_account_recovery_db(),
    )
    .await;

    // Expect an error
    verify_error_string(
        pepper_result,
        "The oauth nonce in the JWT does not match the recalculated nonce",
    );
}

#[tokio::test]
async fn request_invalid_jwt() {
    // Generate a VUF private key
    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    // Create a JWK cache and resource cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let cached_resources = CachedResources::new_for_testing();

    // Update the keyless config cached resource
    set_on_chain_keyless_configuration(&cached_resources);

    // Handle the pepper request
    let pepper_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        "invalid jwt string".into(),
        generate_ephemeral_public_key(),
        SAMPLE_EXP_DATE,
        get_sample_epk_blinder(),
        None,
        None,
        None,
        utils::get_mock_account_recovery_db(),
    )
    .await;

    // Expect an error
    verify_error_string(pepper_result, "JWT decoding error");
}

#[tokio::test]
async fn request_invalid_jwt_signature() {
    // Generate a VUF private key
    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    // Create a JWK cache and resource cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let cached_resources = CachedResources::new_for_testing();

    // Update the JWK cache
    jwk_fetcher::insert_test_jwk(jwk_cache.clone());

    // Update the keyless config cached resource
    set_on_chain_keyless_configuration(&cached_resources);

    // Create an oauth nonce
    let ephemeral_public_key = generate_ephemeral_public_key();
    let exp_date_secs = get_current_time_secs() + 10;
    let epk_blinder = get_sample_epk_blinder();
    let keyless_configuration = OnChainKeylessConfiguration::new_for_testing()
        .get_keyless_configuration()
        .unwrap();
    let oauth_nonce = OpenIdSig::reconstruct_oauth_nonce(
        &epk_blinder,
        exp_date_secs,
        &ephemeral_public_key,
        &keyless_configuration,
    )
    .unwrap();

    // Create a test JWT
    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        &SAMPLE_JWT_EXTRA_FIELD,
        get_current_time_secs(),
        &oauth_nonce,
    );
    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    // Handle the pepper request
    let pepper_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        jwt,
        ephemeral_public_key,
        exp_date_secs,
        epk_blinder,
        None,
        None,
        None,
        utils::get_mock_account_recovery_db(),
    )
    .await;

    // Expect an error
    verify_error_string(pepper_result, "JWT signature verification failed");
}

#[tokio::test]
async fn request_max_exp_data_secs_overflowed() {
    // Generate a VUF private key
    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    // Create a JWK cache and resource cache
    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let cached_resources = CachedResources::new_for_testing();

    // Update the keyless config cached resource
    set_on_chain_keyless_configuration(&cached_resources);

    // Create a test JWT
    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        &SAMPLE_JWT_EXTRA_FIELD,
        u64::MAX - 1, // Large issue time to cause overflow
        &SAMPLE_NONCE,
    );
    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    // Handle the pepper request
    let pepper_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        jwt,
        generate_ephemeral_public_key(),
        SAMPLE_EXP_DATE,
        get_sample_epk_blinder(),
        None,
        None,
        None,
        utils::get_mock_account_recovery_db(),
    )
    .await;

    // Expect an error
    verify_error_string(pepper_result, "The maximum allowed expiry date overflowed");
}

/// Generates an ephemeral public key for testing
fn generate_ephemeral_public_key() -> EphemeralPublicKey {
    let sk = get_sample_esk();
    let pk = Ed25519PublicKey::from(&sk);
    EphemeralPublicKey::ed25519(pk)
}

/// Gets the current time in seconds since the UNIX epoch
fn get_current_time_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Sets a test on-chain keyless configuration in the cached resources
fn set_on_chain_keyless_configuration(cached_resources: &CachedResources) {
    let on_chain_keyless_configuration = OnChainKeylessConfiguration::new_for_testing();
    cached_resources.set_on_chain_keyless_configuration(on_chain_keyless_configuration);
}

/// Verifies that the error from a pepper request contains the expected message substring
fn verify_error_string(
    pepper_result: Result<(Vec<u8>, Vec<u8>, AccountAddress), PepperServiceError>,
    expected_message: &str,
) {
    let pepper_service_error = pepper_result.unwrap_err();
    assert!(pepper_service_error.to_string().contains(expected_message));
}
