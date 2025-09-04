// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    jwks::rsa::RSA_JWK,
    keyless::{
        base64url_encode_bytes, base64url_encode_str, seconds_from_epoch, Configuration,
        IdCommitment, KeylessPublicKey, Pepper,
    },
    transaction::authenticator::EphemeralPublicKey,
};
use anyhow::{ensure, Context};
use velor_crypto::{poseidon_bn254, CryptoMaterialError};
use ark_bn254::Fr;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::skip_serializing_none;
use std::collections::BTreeMap;

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Hash, Serialize)]
#[cfg_attr(feature = "fuzzing", derive(arbitrary::Arbitrary))]
pub struct OpenIdSig {
    /// The decoded bytes of the JWS signature in the JWT (<https://datatracker.ietf.org/doc/html/rfc7515#section-3>)
    #[serde(with = "serde_bytes")]
    pub jwt_sig: Vec<u8>,
    /// The decoded/plaintext JSON payload of the JWT (<https://datatracker.ietf.org/doc/html/rfc7519#section-3>)
    pub jwt_payload_json: String,
    /// The name of the key in the claim that maps to the user identifier; e.g., "sub" or "email"
    pub uid_key: String,
    /// The random value used to obfuscate the EPK from OIDC providers in the nonce field
    #[serde(with = "serde_bytes")]
    pub epk_blinder: Vec<u8>,
    /// The privacy-preserving value used to calculate the identity commitment. It is typically uniquely derived from `(iss, client_id, uid_key, uid_val)`.
    pub pepper: Pepper,
    /// When an override aud_val is used, the signature needs to contain the aud_val committed in the
    /// IDC, since the JWT will contain the override.
    pub idc_aud_val: Option<String>,
}

impl OpenIdSig {
    /// The size of the blinding factor used to compute the nonce commitment to the EPK and expiration
    /// date. This can be upgraded, if the OAuth nonce reconstruction is upgraded carefully.
    pub const EPK_BLINDER_NUM_BYTES: usize = poseidon_bn254::keyless::BYTES_PACKED_PER_SCALAR;

    /// Verifies an `OpenIdSig` by doing the following checks:
    ///  1. Check that the ephemeral public key lifespan is under MAX_EXPIRY_HORIZON_SECS
    ///  2. Check that the `iss` claim in the `PublicKey` matches the one in the `jwt_payload`
    ///  3. Check that the identity commitment in the `PublicKey` matches the one constructed from
    ///     the `jwt_payload`
    ///  4. Check that the nonce constructed from the ephemeral public key, blinder, and
    ///     `exp_timestamp_secs` matches the one in the jwt_payload
    ///
    /// TODO(keyless): Refactor to return a `Result<(), VMStatus>` because (1) this is now called in the
    ///  VM and (2) is_override_aud_allowed does.
    pub fn verify_jwt_claims(
        &self,
        exp_timestamp_secs: u64,
        epk: &EphemeralPublicKey,
        pk: &KeylessPublicKey,
        config: &Configuration,
    ) -> anyhow::Result<()> {
        let claims: Claims = serde_json::from_str(&self.jwt_payload_json)?;

        let max_expiration_date = seconds_from_epoch(
            claims
                .oidc_claims
                .iat
                .checked_add(config.max_exp_horizon_secs)
                .ok_or_else(|| {
                    anyhow::anyhow!("Overflow when adding iat and max_exp_horizon_secs")
                })?,
        )?;
        let expiration_date = seconds_from_epoch(exp_timestamp_secs)?;

        ensure!(
            expiration_date < max_expiration_date,
            "The ephemeral public key's expiration date is too far into the future"
        );

        ensure!(
            claims.oidc_claims.iss.eq(&pk.iss_val),
            "'iss' claim was supposed to match \"{}\"",
            pk.iss_val
        );

        // When an aud_val override is set, the IDC-committed `aud` is included next to the
        // OpenID signature.
        let idc_aud_val = match self.idc_aud_val.as_ref() {
            None => &claims.oidc_claims.aud,
            Some(idc_aud_val) => {
                // If there's an override, check that the override `aud` from the JWT, is allow-listed
                ensure!(
                    config
                        .is_allowed_override_aud(&claims.oidc_claims.aud)
                        .is_ok(),
                    "{} is not an allow-listed override aud",
                    &claims.oidc_claims.aud
                );
                idc_aud_val
            },
        };
        let uid_val = claims.get_uid_val(&self.uid_key)?;
        ensure!(
            IdCommitment::new_from_preimage(&self.pepper, idc_aud_val, &self.uid_key, &uid_val)?
                .eq(&pk.idc),
            "Address IDC verification failed"
        );

        let actual_nonce = OpenIdSig::reconstruct_oauth_nonce(
            &self.epk_blinder[..],
            exp_timestamp_secs,
            epk,
            config,
        )?;
        ensure!(
            actual_nonce.eq(&claims.oidc_claims.nonce),
            "'nonce' claim did not match: JWT contained {} but recomputed {}",
            claims.oidc_claims.nonce,
            actual_nonce
        );

        Ok(())
    }

    /// `jwt_header_json` is the *decoded* JWT header (i.e., *not* base64url-encoded)
    pub fn verify_jwt_signature(
        &self,
        rsa_jwk: &RSA_JWK,
        jwt_header_json: &str,
    ) -> anyhow::Result<()> {
        let jwt_b64 = format!(
            "{}.{}.{}",
            base64url_encode_str(jwt_header_json),
            base64url_encode_str(&self.jwt_payload_json),
            base64url_encode_bytes(&self.jwt_sig)
        );
        rsa_jwk.verify_signature_without_exp_check(&jwt_b64)?;
        Ok(())
    }

    pub fn reconstruct_oauth_nonce(
        epk_blinder: &[u8],
        exp_timestamp_secs: u64,
        epk: &EphemeralPublicKey,
        config: &Configuration,
    ) -> anyhow::Result<String> {
        let mut frs = poseidon_bn254::keyless::pad_and_pack_bytes_to_scalars_with_len(
            epk.to_bytes().as_slice(),
            config.max_commited_epk_bytes as usize,
        )?;

        frs.push(Fr::from(exp_timestamp_secs));
        frs.push(poseidon_bn254::keyless::pack_bytes_to_one_scalar(
            epk_blinder,
        )?);

        let nonce_fr = poseidon_bn254::hash_scalars(frs)?;
        Ok(nonce_fr.to_string())
    }
}

impl TryFrom<&[u8]> for OpenIdSig {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, CryptoMaterialError> {
        bcs::from_bytes::<OpenIdSig>(bytes).map_err(|_e| CryptoMaterialError::DeserializationError)
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcClaims {
    pub iss: String,
    pub aud: String,
    pub sub: String,
    pub nonce: String,
    pub iat: u64,
    pub exp: u64,
    pub email: Option<String>,
    pub email_verified: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    #[serde(flatten)]
    pub oidc_claims: OidcClaims,
    #[serde(default)]
    #[serde(flatten)]
    pub additional_claims: BTreeMap<String, Value>,
}

impl Claims {
    pub fn get_uid_val(&self, uid_key: &String) -> anyhow::Result<String> {
        match uid_key.as_str() {
            "email" => {
                let email_verified = self
                    .oidc_claims
                    .email_verified
                    .clone()
                    .context("'email_verified' claim is missing")?;
                // the 'email_verified' claim may be a boolean or a boolean-as-a-string.
                let email_verified_as_bool = email_verified.as_bool().unwrap_or(false);
                let email_verified_as_str = email_verified.as_str().unwrap_or("false");
                ensure!(
                    email_verified_as_bool || email_verified_as_str.eq("true"),
                    "'email_verified' claim was not \"true\""
                );
                self.oidc_claims
                    .email
                    .clone()
                    .context("email claim missing on jwt")
            },
            "sub" => Ok(self.oidc_claims.sub.clone()),
            _ => {
                let uid_val = self
                    .additional_claims
                    .get(uid_key)
                    .context(format!("{} claim missing on jwt", uid_key))?
                    .as_str()
                    .context(format!("{} value is not a string", uid_key))?;
                Ok(uid_val.to_string())
            },
        }
    }
}
