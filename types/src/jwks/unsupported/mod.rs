// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{jwks::KID, move_any::AsMoveAny, move_utils::as_move_value::AsMoveValue};
use aptos_crypto::HashValue;
use move_core_types::value::{MoveStruct, MoveValue};
use poem_openapi_derive::Object;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Formatter};

/// Move type `0x1::jwks::UnsupportedJWK` in rust.
/// See its doc in Move for more details.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct UnsupportedJWK {
    pub id: Vec<u8>,
    pub payload: Vec<u8>,
}

impl Debug for UnsupportedJWK {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UnsupportedJWK")
            .field("id", &hex::encode(self.id.as_slice()))
            .field("payload", &String::from_utf8(self.payload.clone()))
            .finish()
    }
}

impl UnsupportedJWK {
    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_for_testing(id: &str, payload: &str) -> Self {
        Self {
            id: id.as_bytes().to_vec(),
            payload: payload.as_bytes().to_vec(),
        }
    }

    #[cfg(any(test, feature = "fuzzing"))]
    pub fn new_with_payload(payload: &str) -> Self {
        let id = HashValue::sha3_256_of(payload.as_bytes()).to_vec();
        Self {
            id,
            payload: payload.as_bytes().to_vec(),
        }
    }

    pub fn id(&self) -> KID {
        self.id.clone()
    }
}

impl From<serde_json::Value> for UnsupportedJWK {
    fn from(json_value: serde_json::Value) -> Self {
        let payload = json_value.to_string().into_bytes(); //TODO: canonical to_string.
        Self {
            id: HashValue::sha3_256_of(payload.as_slice()).to_vec(),
            payload,
        }
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

#[cfg(test)]
mod tests;
