// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::unsupported::UnsupportedJWK,
    move_any::{Any as MoveAny, AsMoveAny},
    move_utils::as_move_value::AsMoveValue,
};
use velor_crypto::HashValue;
use std::str::FromStr;

#[test]
fn convert_json_to_unsupported_jwk() {
    // Some unknown JWK format
    let compact_json_str = "{\"key0\":\"val0\",\"key1\":999}";
    let expected_payload = compact_json_str.as_bytes().to_vec();
    let expected_id = HashValue::sha3_256_of(expected_payload.as_slice()).to_vec();
    let json = serde_json::Value::from_str(compact_json_str).unwrap();
    let actual = UnsupportedJWK::from(json);
    let expected = UnsupportedJWK {
        id: expected_id,
        payload: expected_payload,
    };
    assert_eq!(expected, actual);
}

#[test]
fn unsupported_jwk_as_move_value() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("AAA", "BBBB");
    let move_value = unsupported_jwk.as_move_value();
    assert_eq!(
        vec![3, 65, 65, 65, 4, 66, 66, 66, 66],
        move_value.simple_serialize().unwrap()
    );
}

#[test]
fn unsupported_jwk_as_move_any() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("AAA", "BBBB");
    let actual = unsupported_jwk.as_move_any();
    let expected = MoveAny {
        type_name: "0x1::jwks::UnsupportedJWK".to_string(),
        data: bcs::to_bytes(&unsupported_jwk).unwrap(),
    };
    assert_eq!(expected, actual);
}
