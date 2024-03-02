use crate::common::get_test_circuit_config;
use common::{
    convert_prove_and_verify,
    types::{ProofTestCase, TestJWTPayload},
};
use serial_test::serial;

mod common;

// TODO So far it seems extra_field has to be Some(..) otherwise things don't work right. Write
// some tests for this once I fix it.
// TODO write function that loads verification_key.json into a PreparedVerifyingKey<Bn254>

#[test]
#[serial]
fn default_request() {
    let testcase = ProofTestCase::default_with_payload(TestJWTPayload::default())
        .compute_nonce(&get_test_circuit_config());

    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
fn request_with_email() {
    let jwt_payload = TestJWTPayload {
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        uid_key: String::from("email"),
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());
    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
#[should_panic]
fn request_sub_is_required_in_jwt() {
    let jwt_payload = TestJWTPayload {
        sub: None,
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        uid_key: String::from("email"),
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());
    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
fn request_with_sub() {
    let jwt_payload = TestJWTPayload {
        email: None,
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        uid_key: String::from("sub"),
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());
    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
fn request_with_sub_no_email_verified() {
    let jwt_payload = TestJWTPayload {
        email: None,
        email_verified: None,
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        uid_key: String::from("sub"),
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());
    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
#[should_panic]
fn request_with_wrong_uid_key() {
    let jwt_payload = TestJWTPayload {
        email: None,
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        uid_key: String::from("email"),
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());

    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
#[should_panic]
fn request_with_invalid_exp_date() {
    let jwt_payload = TestJWTPayload {
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        epk_expiry_horizon_secs: 100,
        epk_expiry_time_secs: 200,
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());

    convert_prove_and_verify(&testcase).unwrap();
}

#[test]
#[serial]
fn request_jwt_exp_field_does_not_matter() {
    let jwt_payload = TestJWTPayload {
        exp: 234342342428348284,
        ..TestJWTPayload::default()
    };

    let testcase = ProofTestCase {
        ..ProofTestCase::default_with_payload(jwt_payload)
    }
    .compute_nonce(&get_test_circuit_config());

    convert_prove_and_verify(&testcase).unwrap();
}
