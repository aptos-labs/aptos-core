// Copyright © Aptos Foundation

use crate::{
    jwks::{rsa::RSA_JWK, unsupported::UnsupportedJWK},
    move_any::{Any as MoveAny, AsMoveAny},
};
use anyhow::anyhow;
use serde::{Deserialize, Serialize};

/// Reflection of Move type `0x1::jwks::JWK`.
/// When you load an on-chain config that contains some JWK(s), the JWK will be of this type.
/// When you call a Move function from rust that takes some JWKs as input, pass in JWKs of this type.
/// Otherwise, it is recommended to convert this to the rust enum `JWK` below for better rust experience.
#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct JWKMoveStruct {
    pub variant: MoveAny,
}

/// The JWK type that can be converted from/to `JWKMoveStruct` but easier to use in rust.
#[derive(Debug, PartialEq)]
pub enum JWK {
    RSA(RSA_JWK),
    Unsupported(UnsupportedJWK),
}

impl From<JWK> for JWKMoveStruct {
    fn from(jwk: JWK) -> Self {
        let variant = match jwk {
            JWK::RSA(variant) => variant.as_move_any(),
            JWK::Unsupported(variant) => variant.as_move_any(),
        };
        JWKMoveStruct { variant }
    }
}

impl TryFrom<JWKMoveStruct> for JWK {
    type Error = anyhow::Error;

    fn try_from(value: JWKMoveStruct) -> Result<Self, Self::Error> {
        match value.variant.type_name.as_str() {
            RSA_JWK::MOVE_TYPE_NAME => {
                let rsa_jwk = MoveAny::unpack(RSA_JWK::MOVE_TYPE_NAME, value.variant).unwrap();
                Ok(Self::RSA(rsa_jwk))
            },
            UnsupportedJWK::MOVE_TYPE_NAME => {
                let unsupported_jwk =
                    MoveAny::unpack(UnsupportedJWK::MOVE_TYPE_NAME, value.variant).unwrap();
                Ok(Self::Unsupported(unsupported_jwk))
            },
            _ => Err(anyhow!(
                "convertion from jwk move struct to jwk failed with unknown variant"
            )),
        }
    }
}

#[test]
fn convert_jwk_move_struct_to_jwk() {
    let unsupported_jwk = UnsupportedJWK::new_for_testing("id1", "payload1");
    let jwk_move_struct = JWKMoveStruct {
        variant: unsupported_jwk.as_move_any(),
    };
    assert_eq!(
        JWK::Unsupported(unsupported_jwk),
        JWK::try_from(jwk_move_struct).unwrap()
    );

    let rsa_jwk = RSA_JWK::new_for_testing("kid1", "kty1", "alg1", "e1", "n1");
    let jwk_move_struct = JWKMoveStruct {
        variant: rsa_jwk.as_move_any(),
    };
    assert_eq!(JWK::RSA(rsa_jwk), JWK::try_from(jwk_move_struct).unwrap());

    let unknown_jwk_variant = MoveAny {
        type_name: "type1".to_string(),
        data: vec![],
    };
    assert!(JWK::try_from(JWKMoveStruct {
        variant: unknown_jwk_variant
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

    let rsa_jwk = RSA_JWK::new_for_testing("kid1", "kty1", "alg1", "e1", "n1");
    let jwk = JWK::RSA(rsa_jwk.clone());
    let jwk_move_struct = JWKMoveStruct {
        variant: rsa_jwk.as_move_any(),
    };
    assert_eq!(jwk_move_struct, JWKMoveStruct::from(jwk));
}
