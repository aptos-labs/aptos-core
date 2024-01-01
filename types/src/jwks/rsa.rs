// Copyright © Aptos Foundation

use anyhow::{anyhow, bail};
use serde::{Deserialize, Serialize};
use aptos_crypto_derive::{BCSCryptoHash, CryptoHasher};
use move_core_types::value::{MoveStruct, MoveValue};
use crate::move_any::AsMoveAny;
use crate::move_utils::as_move_value::AsMoveValue;

/// Move type `0x1::jwks::RSA_JWK` in rust.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, CryptoHasher, BCSCryptoHash)]
pub struct RSA_JWK {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub e: String,
    pub n: String,
}

impl AsMoveAny for RSA_JWK {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwks::RSA_JWK";
}

impl TryFrom<&serde_json::Value> for RSA_JWK {
    type Error = anyhow::Error;

    fn try_from(json_value: &serde_json::Value) -> Result<Self, Self::Error> {
        let ret = Self {
            kid: json_value.get("kid").ok_or_else(|| anyhow!("Field `kid` not found"))?.as_str().ok_or_else(|| anyhow!("Field `kid` is not a string"))?.to_string(),
            kty: json_value.get("kty").ok_or_else(|| anyhow!("Field `kty` not found"))?.as_str().ok_or_else(|| anyhow!("Field `kty` is not a string"))?.to_string(),
            alg: json_value.get("alg").ok_or_else(|| anyhow!("Field `alg` not found"))?.as_str().ok_or_else(|| anyhow!("Field `alg` is not a string"))?.to_string(),
            e: json_value.get("e").ok_or_else(|| anyhow!("Field `e` not found"))?.as_str().ok_or_else(|| anyhow!("Field `e` is not a string"))?.to_string(),
            n: json_value.get("n").ok_or_else(|| anyhow!("Field `n` not found"))?.as_str().ok_or_else(|| anyhow!("Field `n` is not a string"))?.to_string(),
        };

        if ret.alg.as_str() != "RS256" {
            bail!("field `kid` should be `RS256`");
        }

        if ret.kty.as_str() != "RSA" {
            bail!("field `kty` should be `RSA`");
        }

        Ok(ret)
    }
}

impl AsMoveValue for RSA_JWK {
    fn as_move_value(&self) -> MoveValue {
        MoveValue::Struct(MoveStruct::Runtime(vec![
            self.kid.as_move_value(),
            self.kty.as_move_value(),
            self.alg.as_move_value(),
            self.e.as_move_value(),
            self.n.as_move_value(),
        ]))
    }
}
