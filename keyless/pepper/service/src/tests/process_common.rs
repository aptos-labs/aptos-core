// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    dedicated_handlers::pepper_request::handle_pepper_request, error::PepperServiceError,
    external_resources::resource_fetcher::CachedResources, tests::utils,
};
use aptos_crypto::ed25519::Ed25519PublicKey;
use aptos_infallible::Mutex;
use aptos_types::{
    keyless::{
        circuit_testcases::{
            sample_jwt_payload_json_overrides, SAMPLE_EXP_DATE, SAMPLE_JWT_EXTRA_FIELD,
            SAMPLE_NONCE, SAMPLE_TEST_ISS_VALUE, SAMPLE_UID_VAL,
        },
        test_utils::{get_sample_epk_blinder, get_sample_esk, get_sample_jwt_token_from_payload},
    },
    transaction::authenticator::EphemeralPublicKey,
};
use std::{collections::HashMap, ops::Deref, sync::Arc};

// TODO: clean this up and add missing tests!

#[tokio::test]
async fn process_common_should_fail_if_max_exp_data_secs_overflowed() {
    let sk = get_sample_esk();
    let pk = Ed25519PublicKey::from(&sk);

    let jwk_cache = Arc::new(Mutex::new(HashMap::new()));
    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        SAMPLE_JWT_EXTRA_FIELD.as_str(),
        u64::MAX - 1, // unusual iat
        SAMPLE_NONCE.as_str(),
    );

    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    let cached_resources = CachedResources::default();

    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    let process_result = handle_pepper_request(
        vuf_private_key,
        jwk_cache,
        cached_resources,
        jwt,
        EphemeralPublicKey::ed25519(pk),
        SAMPLE_EXP_DATE,
        get_sample_epk_blinder(),
        None,
        None,
        None,
        false,
    )
    .await;
    assert!(
        matches!(process_result, Err(PepperServiceError::BadRequest(e)) if e.as_str() == "max_exp_data_secs overflowed")
    );
}
