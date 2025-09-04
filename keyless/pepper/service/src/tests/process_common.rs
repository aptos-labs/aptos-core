// Copyright (c) Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{error::PepperServiceError, process_common, tests::utils};
use velor_crypto::ed25519::Ed25519PublicKey;
use velor_types::{
    keyless::{
        circuit_testcases::{
            sample_jwt_payload_json_overrides, SAMPLE_EXP_DATE, SAMPLE_JWT_EXTRA_FIELD,
            SAMPLE_NONCE, SAMPLE_TEST_ISS_VALUE, SAMPLE_UID_VAL,
        },
        test_utils::{get_sample_epk_blinder, get_sample_esk, get_sample_jwt_token_from_payload},
    },
    transaction::authenticator::EphemeralPublicKey,
};
use std::ops::Deref;
use uuid::Uuid;

// TODO: clean this up and add missing tests!

#[tokio::test]
async fn process_common_should_fail_if_max_exp_data_secs_overflowed() {
    let session_id = Uuid::new_v4();
    let sk = get_sample_esk();
    let pk = Ed25519PublicKey::from(&sk);

    let jwt_payload = sample_jwt_payload_json_overrides(
        SAMPLE_TEST_ISS_VALUE,
        SAMPLE_UID_VAL,
        SAMPLE_JWT_EXTRA_FIELD.as_str(),
        u64::MAX - 1, // unusual iat
        SAMPLE_NONCE.as_str(),
    );

    let vuf_keypair = utils::create_vuf_public_private_keypair();
    let (_, vuf_private_key) = vuf_keypair.deref();

    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    let process_result = process_common(
        vuf_private_key,
        &session_id,
        jwt,
        EphemeralPublicKey::ed25519(pk),
        SAMPLE_EXP_DATE,
        get_sample_epk_blinder(),
        None,
        None,
        false,
        None,
        false,
    )
    .await;
    assert!(
        matches!(process_result, Err(PepperServiceError::BadRequest(e)) if e.as_str() == "max_exp_data_secs overflowed")
    );
}
