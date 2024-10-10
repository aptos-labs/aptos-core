// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{process_common, ProcessingFailure};
use aptos_crypto::ed25519::Ed25519PublicKey;
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
use uuid::Uuid;

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

    let jwt = get_sample_jwt_token_from_payload(&jwt_payload);

    let process_result = process_common(
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
        matches!(process_result, Err(ProcessingFailure::BadRequest(e)) if e.as_str() == "max_exp_data_secs overflowed")
    );
}
