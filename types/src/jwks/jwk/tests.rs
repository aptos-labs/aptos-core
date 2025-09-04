// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::{
        jwk::{JWKMoveStruct, JWK},
        rsa::RSA_JWK,
        unsupported::UnsupportedJWK,
    },
    move_any::{Any as MoveAny, AsMoveAny},
};
use velor_crypto::HashValue;
use std::str::FromStr;

#[test]
fn convert_jwk_move_struct_to_jwk() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("id1", "payload1");
    let jwk_move_struct = JWKMoveStruct {
        variant: unsupported_jwk.as_move_any(),
    };
    assert_eq!(
        JWK::Unsupported(unsupported_jwk),
        JWK::try_from(&jwk_move_struct).unwrap()
    );

    let rsa_jwk = RSA_JWK::new_from_strs("kid1", "kty1", "alg1", "e1", "n1");
    let jwk_move_struct = JWKMoveStruct {
        variant: rsa_jwk.as_move_any(),
    };
    assert_eq!(JWK::RSA(rsa_jwk), JWK::try_from(&jwk_move_struct).unwrap());

    let unknown_jwk_variant = MoveAny {
        type_name: "type1".to_string(),
        data: vec![],
    };
    assert!(JWK::try_from(&JWKMoveStruct {
        variant: unknown_jwk_variant
    })
    .is_err());

    let jwk_with_mauled_data_0 = MoveAny {
        type_name: RSA_JWK::MOVE_TYPE_NAME.to_string(),
        data: vec![],
    };
    assert!(JWK::try_from(&JWKMoveStruct {
        variant: jwk_with_mauled_data_0
    })
    .is_err());

    let jwk_with_mauled_data_1 = MoveAny {
        type_name: UnsupportedJWK::MOVE_TYPE_NAME.to_string(),
        data: vec![],
    };
    assert!(JWK::try_from(&JWKMoveStruct {
        variant: jwk_with_mauled_data_1
    })
    .is_err());
}

#[test]
fn convert_jwk_to_jwk_move_struct() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("id1", "payload1");
    let jwk = JWK::Unsupported(unsupported_jwk.clone());
    let jwk_move_struct = JWKMoveStruct {
        variant: unsupported_jwk.as_move_any(),
    };
    assert_eq!(jwk_move_struct, JWKMoveStruct::from(jwk));

    let rsa_jwk = RSA_JWK::new_from_strs("kid1", "kty1", "alg1", "e1", "n1");
    let jwk = JWK::RSA(rsa_jwk.clone());
    let jwk_move_struct = JWKMoveStruct {
        variant: rsa_jwk.as_move_any(),
    };
    assert_eq!(jwk_move_struct, JWKMoveStruct::from(jwk));
}

#[test]
fn convert_json_value_to_jwk() {
    let json_str =
        r#"{"alg": "RS256", "kid": "kid1", "e": "AQAB", "use": "sig", "kty": "RSA", "n": "13131"}"#;
    let json = serde_json::Value::from_str(json_str).unwrap();
    let actual = JWK::from(json);
    let expected = JWK::RSA(RSA_JWK::new_from_strs(
        "kid1", "RSA", "RS256", "AQAB", "13131",
    ));
    assert_eq!(expected, actual);

    let compact_json_str = r#"{"alg":13131}"#;
    let json = serde_json::Value::from_str(compact_json_str).unwrap();
    let actual = JWK::from(json);
    let expected_payload = compact_json_str.as_bytes().to_vec();
    let expected_id = HashValue::sha3_256_of(expected_payload.as_slice()).to_vec();
    let expected = JWK::Unsupported(UnsupportedJWK {
        id: expected_id,
        payload: expected_payload,
    });
    assert_eq!(expected, actual);
}
