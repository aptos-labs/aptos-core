// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::rsa::RSA_JWK,
    move_any::{Any as MoveAny, AsMoveAny},
    move_utils::as_move_value::AsMoveValue,
};
use std::str::FromStr;

#[test]
fn convert_json_to_rsa_jwk() {
    // Valid JWK JSON should be accepted.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    let actual = RSA_JWK::try_from(&json);
    let expected = RSA_JWK::new_from_strs("kid1", "RSA", "RS256", "AQAB", "13131");
    assert_eq!(expected, actual.unwrap());

    // JWK JSON without `kid` should be rejected.
    let json_str = r#"{"alg": "RS256", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `kid` type should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": {}, "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `alg` should be rejected.
    let json_str = r#"{"kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `alg` type should be rejected.
    let json_str =
        r#"{"alg": 0, "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `kty` should be rejected.
    let json_str = r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `kty` value should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSB", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `e` should be rejected.
    let json_str = r#"{"alg": "RS256", "kid": "kid1", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `e` type should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": 65537, "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON without `n` should be rejected.
    let json_str = r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());

    // JWK JSON with wrong `n` type should be rejected.
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": false}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    assert!(RSA_JWK::try_from(&json).is_err());
}

#[test]
fn rsa_jwk_as_move_value() {
    let rsa_jwk = RSA_JWK::new_from_strs("kid1", "RSA", "RS256", "AQAB", "13131");
    let move_value = rsa_jwk.as_move_value();
    assert_eq!(
        vec![
            4, 107, 105, 100, 49, 3, 82, 83, 65, 5, 82, 83, 50, 53, 54, 4, 65, 81, 65, 66, 5, 49,
            51, 49, 51, 49
        ],
        move_value.simple_serialize().unwrap()
    );
}

#[test]
fn rsa_jwk_as_move_any() {
    let rsa_jwk = RSA_JWK::new_from_strs("kid1", "RSA", "RS256", "AQAB", "1313131313131");
    let actual = rsa_jwk.as_move_any();
    let expected = MoveAny {
        type_name: "0x1::jwks::RSA_JWK".to_string(),
        data: bcs::to_bytes(&rsa_jwk).unwrap(),
    };
    assert_eq!(expected, actual);
}
