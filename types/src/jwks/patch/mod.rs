// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    jwks::jwk::{JWKMoveStruct, JWK},
    move_any::{Any as MoveAny, AsMoveAny},
    move_utils::as_move_value::AsMoveValue,
};
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

pub struct PatchJWKMoveStruct {
    pub variant: MoveAny,
}

impl AsMoveValue for PatchJWKMoveStruct {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![self.variant.as_move_value()]))
    }
}

impl From<PatchUpsertJWK> for PatchJWKMoveStruct {
    fn from(patch: PatchUpsertJWK) -> Self {
        Self {
            variant: patch.as_move_any(),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct PatchUpsertJWK {
    pub issuer: String,
    pub jwk: JWKMoveStruct,
}

impl AsMoveAny for PatchUpsertJWK {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwks::PatchUpsertJWK";
}

/// A variant representation used in genesis layout.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IssuerJWK {
    pub issuer: String,
    pub jwk: JWK,
}
