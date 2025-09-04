// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{keyless::Claims, move_any::AsMoveAny, move_utils::as_move_value::AsMoveValue};
use anyhow::{anyhow, bail, ensure, Result};
use velor_crypto::poseidon_bn254;
use base64::URL_SAFE_NO_PAD;
use jsonwebtoken::{Algorithm, DecodingKey, TokenData, Validation};
use move_core_types::value::{MoveStruct, MoveValue};
use once_cell::sync::Lazy;
use poem_openapi_derive::Object;
use ring::signature::RsaKeyPair;
use rsa::{pkcs1::EncodeRsaPrivateKey, pkcs8::DecodePrivateKey};
use serde::{Deserialize, Serialize};
/// Move type `0x1::jwks::RSA_JWK` in rust.
/// See its doc in Move for more details.
#[allow(non_camel_case_types)]
#[derive(Clone, Debug, Hash, PartialEq, Eq, Serialize, Deserialize, Object)]
pub struct RSA_JWK {
    pub kid: String,
    pub kty: String,
    pub alg: String,
    pub e: String,
    pub n: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct RsaJwkSet {
    keys: Vec<RSA_JWK>,
}

pub fn get_jwk_from_str(s: &str) -> RSA_JWK {
    let RsaJwkSet { keys } = serde_json::from_str(s).expect("Unable to parse JSON");
    keys[0].clone()
}

pub static SECURE_TEST_RSA_JWK: Lazy<RSA_JWK> =
    Lazy::new(|| get_jwk_from_str(include_str!("secure_test_jwk.json")));

pub static INSECURE_TEST_RSA_JWK: Lazy<RSA_JWK> =
    Lazy::new(|| get_jwk_from_str(include_str!("insecure_test_jwk.json")));

pub static INSECURE_TEST_RSA_KEY_PAIR: Lazy<RsaKeyPair> = Lazy::new(|| {
    // TODO(keyless): Hacking around the difficulty of parsing PKCS#8-encoded PEM files with the `pem` crate
    let der = rsa::RsaPrivateKey::from_pkcs8_pem(include_str!("insecure_test_jwk_private_key.pem"))
        .unwrap()
        .to_pkcs1_der()
        .unwrap();
    RsaKeyPair::from_der(der.as_bytes()).unwrap()
});

impl RSA_JWK {
    /// The circuit-supported RSA modulus size.
    pub const RSA_MODULUS_BYTES: usize = 256;

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

    // The private key to this JWK is found under INTERNAL_TEST_OIDC_PROVIDER_PRIVATE_KEY in velor-keyless-prod in gcloud secrets
    pub fn secure_test_jwk() -> RSA_JWK {
        RSA_JWK {
            kid:"test-rsa".to_owned(),
            kty:"RSA".to_owned(),
            alg:"RS256".to_owned(),
            e:"AQAB".to_owned(),
            n:"y5Efs1ZzisLLKCARSvTztgWj5JFP3778dZWt-od78fmOZFxem3a_aYbOXSJToRp862do0PxJ4PDMpmqwV5f7KplFI6NswQV-WPufQH8IaHXZtuPdCjPOcHybcDiLkO12d0dG6iZQUzypjAJf63APcadio-4JDNWlGC5_Ow_XQ9lIY71kTMiT9lkCCd0ZxqEifGtnJe5xSoZoaMRKrvlOw-R6iVjLUtPAk5hyUX95LDKxwAR-oshnj7gmATejga2EvH9ozdn3M8Go11PSDa04OQxPcA25OoDTfxLvT28LRpSXrbmUWZ-O_lGtDl3ZAtjIguYGEobTk4N11eRssC95Cw".to_owned()
        }
    }

    pub fn verify_signature_without_exp_check(&self, jwt_token: &str) -> Result<TokenData<Claims>> {
        let mut validation = Validation::new(Algorithm::RS256);
        validation.validate_exp = false;
        let key = &DecodingKey::from_rsa_components(&self.n, &self.e)?;
        let claims = jsonwebtoken::decode::<Claims>(jwt_token, key, &validation)?;
        Ok(claims)
    }

    pub fn id(&self) -> Vec<u8> {
        self.kid.as_bytes().to_vec()
    }

    // TODO(keyless): Move this to velor-crypto so other services can use this
    pub fn to_poseidon_scalar(&self) -> Result<ark_bn254::Fr> {
        let mut modulus = base64::decode_config(&self.n, URL_SAFE_NO_PAD)?;
        // The circuit only supports RSA256
        if modulus.len() != Self::RSA_MODULUS_BYTES {
            bail!(
                "Wrong modulus size, must be {} bytes",
                Self::RSA_MODULUS_BYTES
            );
        }

        // This is done to match the circuit, which requires the modulus in a verify specific format
        // due to how RSA verification is implemented
        modulus.reverse();

        let mut scalars = modulus
            .chunks(24) // Pack 3 64 bit limbs per scalar, so chunk into 24 bytes per scalar
            .map(|chunk| {
                poseidon_bn254::keyless::pack_bytes_to_one_scalar(chunk)
                    .expect("chunk converts to scalar")
            })
            .collect::<Vec<ark_bn254::Fr>>();
        scalars.push(ark_bn254::Fr::from(Self::RSA_MODULUS_BYTES as i32));
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
