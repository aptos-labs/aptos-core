// Copyright © Aptos Foundation

#[cfg(test)]
use crate::move_any::Any as MoveAny;
use crate::{move_any::AsMoveAny, move_utils::as_move_value::AsMoveValue};
use aptos_crypto::HashValue;
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};
#[cfg(test)]
use std::str::FromStr;

/// Move type `0x1::jwks::UnsupportedJWK` in rust.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UnsupportedJWK {
    pub id: Vec<u8>,
    pub payload: Vec<u8>,
}

impl UnsupportedJWK {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_testing(id: &str, payload: &str) -> Self {
        Self {
            id: id.as_bytes().to_vec(),
            payload: payload.as_bytes().to_vec(),
        }
    }
}

impl TryFrom<&serde_json::Value> for UnsupportedJWK {
    type Error = anyhow::Error;

    fn try_from(json_value: &serde_json::Value) -> Result<Self, Self::Error> {
        let payload = json_value.to_string().into_bytes(); //TODO: canonical to_string.
        let ret = Self {
            id: HashValue::sha3_256_of(payload.as_slice()).to_vec(),
            payload,
        };
        Ok(ret)
    }
}

impl AsMoveValue for UnsupportedJWK {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.id.as_move_value(),
            self.payload.as_move_value(),
        ]))
    }
}

impl AsMoveAny for UnsupportedJWK {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwks::UnsupportedJWK";
}

#[test]
fn test_unsupported_jwk_from_json() {
    // Some unknown JWK format
    let compact_json_str = "{\"key0\":\"val0\",\"key1\":999}";
    let expected_payload = compact_json_str.as_bytes().to_vec();
    let expected_id = HashValue::sha3_256_of(expected_payload.as_slice()).to_vec();
    let json = serde_json::Value::from_str(compact_json_str).unwrap();
    let actual = UnsupportedJWK::try_from(&json).unwrap();
    let expected = UnsupportedJWK {
        id: expected_id,
        payload: expected_payload,
    };
    assert_eq!(expected, actual);
}

#[test]
fn test_unsupported_jwk_as_move_value() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("AAA", "BBBB");
    let move_value = unsupported_jwk.as_move_value();
    assert_eq!(
        vec![3, 65, 65, 65, 4, 66, 66, 66, 66],
        move_value.simple_serialize().unwrap()
    );
}

#[test]
fn test_unsupported_jwk_as_move_any() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("AAA", "BBBB");
    let actual = unsupported_jwk.as_move_any();
    let expected = MoveAny {
        type_name: "0x1::jwks::UnsupportedJWK".to_string(),
        data: bcs::to_bytes(&unsupported_jwk).unwrap(),
    };
    assert_eq!(expected, actual);
}
