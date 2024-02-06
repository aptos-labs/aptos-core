// Copyright © Aptos Foundation

use crate::{move_any::AsMoveAny, move_utils::as_move_value::AsMoveValue, zkid::Claims};
use anyhow::{anyhow, bail, ensure, Result};
use aptos_crypto::poseidon_bn254;
use base64::URL_SAFE_NO_PAD;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use move_core_types::value::{MoveStruct, MoveValue};
use serde::{Deserialize, Serialize};

pub const RSA_MODULUS_BYTES: usize = 256;

/// Move type `0x1::jwks::RSA_JWK` in rust.
/// See its doc in Move for more details.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RSA_JWK {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub e: String,
    pub n: String,
}

impl RSA_JWK {
    /// Make an `RSA_JWK` from `kty="RSA", alg="RS256", e="AQAB"` (a popular setting)
    /// and caller-specified `kid` and `n`.
    pub fn new_256_aqab(kid: &str, n: &str) -> Self {
        Self {
            kid: kid.to_string(),
            kty: "RSA".to_string(),
            alg: "RS256".to_string(),
            e: "AQAB".to_string(),
            n: n.to_string(),
        }
    }

    pub fn new_from_strs(kid: &str, kty: &str, alg: &str, e: &str, n: &str) -> Self {
        Self {
            kid: kid.to_string(),
            kty: kty.to_string(),
            alg: alg.to_string(),
            e: e.to_string(),
            n: n.to_string(),
        }
    }

    pub fn verify_signature(&self, jwt_token: &str) -> Result<TokenData<Claims>> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        let key = &DecodingKey::from_rsa_components(&self.n, &self.e)?;
        let claims = jsonwebtoken::decode::<Claims>(jwt_token, key, &validation)?;
        Ok(claims)
    }

    pub fn id(&self) -> Vec<u8> {
        self.kid.as_bytes().to_vec()
    }

    // TODO(zkid): Move this to aptos-crypto so other services can use this
    pub fn to_poseidon_scalar(&self) -> Result<ark_bn254::Fr> {
        let mut modulus = base64::decode_config(&self.n, URL_SAFE_NO_PAD)?;
        // The circuit only supports RSA256
        if modulus.len() != RSA_MODULUS_BYTES {
            bail!("Wrong modulus size, must be {} bytes", RSA_MODULUS_BYTES);
        }
        modulus.reverse(); // This is done to match the circuit, which requires the modulus in a verify specific format due to how RSA verification is implemented
                           // TODO(zkid): finalize the jwk hashing scheme.
        let mut scalars = modulus
            .chunks(24) // Pack 3 64 bit limbs per scalar, so chunk into 24 bytes per scalar
            .map(|chunk| {
                poseidon_bn254::pack_bytes_to_one_scalar(chunk).expect("chunk converts to scalar")
            })
            .collect::<Vec<ark_bn254::Fr>>();
        scalars.push(ark_bn254::Fr::from(RSA_MODULUS_BYTES as i32));
        poseidon_bn254::hash_scalars(scalars)
    }
}

impl AsMoveAny for RSA_JWK {
    const MOVE_TYPE_NAME: &'static str = "0x1::jwks::RSA_JWK";
}

impl TryFrom<&serde_json::Value> for RSA_JWK {
    type Error = anyhow::Error;

    fn try_from(json_value: &serde_json::Value) -> Result<Self, Self::Error> {
        let kty = json_value
            .get("kty")
            .ok_or_else(|| anyhow!("Field `kty` not found"))?
            .as_str()
            .ok_or_else(|| anyhow!("Field `kty` is not a string"))?
            .to_string();

        ensure!(
            kty.as_str() == "RSA",
            "json to rsa jwk conversion failed with incorrect kty"
        );

        let ret = Self {
            kty,
            kid: json_value
                .get("kid")
                .ok_or_else(|| anyhow!("Field `kid` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `kid` is not a string"))?
                .to_string(),
            alg: json_value
                .get("alg")
                .ok_or_else(|| anyhow!("Field `alg` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `alg` is not a string"))?
                .to_string(),
            e: json_value
                .get("e")
                .ok_or_else(|| anyhow!("Field `e` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `e` is not a string"))?
                .to_string(),
            n: json_value
                .get("n")
                .ok_or_else(|| anyhow!("Field `n` not found"))?
                .as_str()
                .ok_or_else(|| anyhow!("Field `n` is not a string"))?
                .to_string(),
        };

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

#[cfg(test)]
mod tests;
