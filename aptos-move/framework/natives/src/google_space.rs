// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Verification of Google Confidential Space OIDC attestation tokens.
//!
//! The JWKS is supplied by `0x1::trust_observation::get_effective_value`, not
//! fetched during VM execution. All time decisions use the chain timestamp
//! supplied by Move. The signed payload is returned verbatim so consumers can
//! inspect claims that are not promoted to fields of `VerifiedGoogleSpaceClaims`.

use anyhow::{bail, ensure, Context, Result};
use aptos_gas_schedule::gas_params::natives::aptos_framework::{
    GOOGLE_SPACE_VERIFY_BASE, GOOGLE_SPACE_VERIFY_PER_BYTE,
};
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeError, SafeNativeResult,
};
use base64::URL_SAFE_NO_PAD;
use jsonwebtoken::{Algorithm, DecodingKey, Validation};
use move_core_types::gas_algebra::NumBytes;
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use serde::Deserialize;
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;

const GOOGLE_ISSUER: &str = "https://confidentialcomputing.googleapis.com";
const MAX_TOKEN_BYTES: usize = 60 * 1024;
const MAX_JWKS_BYTES: usize = 60 * 1024;
const MAX_AUDIENCE_BYTES: usize = 512;
const MAX_NONCE_BYTES: usize = 88;
const EINVALID_ATTESTATION: u64 = 1;

/// Field order matches `0x1::google_space::VerifiedGoogleSpaceClaims`.
#[derive(Debug, Eq, PartialEq)]
pub struct VerifiedGoogleSpaceClaims {
    pub image_digest: Vec<u8>,
    pub image_reference: Vec<u8>,
    pub project_id: Vec<u8>,
    pub instance_id: Vec<u8>,
    pub subject: Vec<u8>,
    pub audience: Vec<u8>,
    pub nonce: Vec<u8>,
    pub hwmodel: Vec<u8>,
    pub issued_at: u64,
    pub not_before: u64,
    pub expires_at: u64,
    pub signed_payload: Vec<u8>,
}

impl VerifiedGoogleSpaceClaims {
    fn into_move_value(self) -> Value {
        Value::struct_(Struct::pack(vec![
            Value::vector_u8(self.image_digest),
            Value::vector_u8(self.image_reference),
            Value::vector_u8(self.project_id),
            Value::vector_u8(self.instance_id),
            Value::vector_u8(self.subject),
            Value::vector_u8(self.audience),
            Value::vector_u8(self.nonce),
            Value::vector_u8(self.hwmodel),
            Value::u64(self.issued_at),
            Value::u64(self.not_before),
            Value::u64(self.expires_at),
            Value::vector_u8(self.signed_payload),
        ]))
    }
}

#[derive(Deserialize)]
struct Jwks {
    keys: Vec<RsaJwk>,
}

#[derive(Deserialize)]
struct RsaJwk {
    kid: String,
    kty: String,
    n: String,
    e: String,
    alg: Option<String>,
    #[serde(rename = "use")]
    key_use: Option<String>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum EatNonce {
    Single(String),
    Multiple(Vec<String>),
}

impl EatNonce {
    fn contains_exactly_once(&self, expected: &str) -> bool {
        match self {
            Self::Single(nonce) => nonce == expected,
            Self::Multiple(nonces) => {
                !nonces.is_empty()
                    && nonces.len() <= 6
                    && nonces
                        .iter()
                        .filter(|nonce| nonce.as_str() == expected)
                        .count()
                        == 1
            },
        }
    }
}

#[derive(Deserialize)]
struct Claims {
    iss: String,
    aud: String,
    sub: String,
    exp: u64,
    iat: u64,
    nbf: u64,
    eat_nonce: EatNonce,
    oemid: u64,
    secboot: bool,
    hwmodel: String,
    swname: String,
    dbgstat: String,
    submods: Submods,
}

#[derive(Deserialize)]
struct Submods {
    container: Container,
    gce: Gce,
}

#[derive(Deserialize)]
struct Container {
    image_digest: String,
    #[serde(default)]
    image_reference: String,
}

#[derive(Deserialize)]
struct Gce {
    project_id: String,
    instance_id: String,
}

/// Verify a Google Cloud Attestation OIDC JWT against validator-observed JWKS.
///
/// `expected_nonce` is the exact UTF-8 string given to the Google token API.
/// The calling Move module constructs and consumes this challenge. It may be
/// derived from a session key or TLS exported key material to bind the token
/// to the worker connection; the token has no separate public-key claim.
pub fn verify_google_space_token(
    token: &[u8],
    trusted_jwks: &[u8],
    expected_audience: &[u8],
    expected_nonce: &[u8],
    now_secs: u64,
) -> Result<VerifiedGoogleSpaceClaims> {
    ensure!(
        !token.is_empty() && token.len() <= MAX_TOKEN_BYTES,
        "Google attestation token has invalid length"
    );
    ensure!(
        !trusted_jwks.is_empty() && trusted_jwks.len() <= MAX_JWKS_BYTES,
        "Google JWKS has invalid length"
    );
    ensure!(
        !expected_audience.is_empty() && expected_audience.len() <= MAX_AUDIENCE_BYTES,
        "Google audience has invalid length"
    );
    ensure!(
        !expected_nonce.is_empty() && expected_nonce.len() <= MAX_NONCE_BYTES,
        "Google nonce has invalid length"
    );
    let token = std::str::from_utf8(token).context("Google token is not UTF-8")?;
    let expected_audience =
        std::str::from_utf8(expected_audience).context("Google audience is not UTF-8")?;
    let expected_nonce =
        std::str::from_utf8(expected_nonce).context("Google nonce is not UTF-8")?;

    let header = jsonwebtoken::decode_header(token).context("invalid Google JWT header")?;
    ensure!(header.alg == Algorithm::RS256, "Google JWT must use RS256");
    ensure!(
        header.x5c.is_none() && header.jku.is_none() && header.jwk.is_none(),
        "Google OIDC JWT must not supply its own verification key"
    );
    let kid = header.kid.context("Google JWT has no key ID")?;
    ensure!(!kid.is_empty() && kid.len() <= 256, "invalid Google key ID");
    let jwks: Jwks = serde_json::from_slice(trusted_jwks).context("invalid Google JWKS")?;
    let mut matching_keys = jwks.keys.iter().filter(|key| key.kid == kid);
    let key = matching_keys
        .next()
        .context("Google JWT key ID is not trusted")?;
    ensure!(
        matching_keys.next().is_none(),
        "duplicate Google JWKS key ID"
    );
    ensure!(key.kty == "RSA", "Google JWT key is not RSA");
    ensure!(
        !key.n.is_empty() && key.n.len() <= 1400 && !key.e.is_empty() && key.e.len() <= 32,
        "Google RSA key has invalid component length"
    );
    ensure!(
        key.alg.as_deref().is_none_or(|alg| alg == "RS256"),
        "Google JWT key algorithm is not RS256"
    );
    ensure!(
        key.key_use
            .as_deref()
            .is_none_or(|key_use| key_use == "sig"),
        "Google JWT key is not a signing key"
    );
    let decoding_key =
        DecodingKey::from_rsa_components(&key.n, &key.e).context("invalid Google RSA key")?;

    // jsonwebtoken's default exp/nbf validation reads the host wall clock.
    // Disable it and validate all time claims against the on-chain timestamp.
    let mut validation = Validation::new(Algorithm::RS256);
    validation.validate_exp = false;
    validation.validate_nbf = false;
    validation.required_spec_claims.clear();
    let claims = jsonwebtoken::decode::<Claims>(token, &decoding_key, &validation)
        .context("Google JWT signature or claims are invalid")?
        .claims;

    ensure!(claims.iss == GOOGLE_ISSUER, "wrong Google JWT issuer");
    ensure!(claims.aud == expected_audience, "wrong Google JWT audience");
    ensure!(
        claims.eat_nonce.contains_exactly_once(expected_nonce),
        "wrong Google JWT nonce"
    );
    ensure!(
        claims.iat <= now_secs && claims.nbf <= now_secs && now_secs < claims.exp,
        "Google JWT is outside its validity window"
    );
    ensure!(
        claims.iat < claims.exp && claims.nbf < claims.exp,
        "inconsistent Google JWT validity window"
    );
    ensure!(
        claims.swname == "CONFIDENTIAL_SPACE",
        "not a Confidential Space image"
    );
    ensure!(
        claims.dbgstat == "disabled-since-boot",
        "debug image is not allowed"
    );
    ensure!(claims.secboot, "Google secure boot is disabled");
    ensure!(claims.oemid == 11129, "wrong Google OEM ID");
    ensure!(
        matches!(
            claims.hwmodel.as_str(),
            "GCP_AMD_SEV" | "GCP_AMD_SEV_ES" | "GCP_INTEL_TDX"
        ),
        "unsupported Google confidential hardware model"
    );
    ensure!(!claims.sub.is_empty(), "Google JWT has no subject");
    ensure!(
        !claims.submods.gce.project_id.is_empty() && !claims.submods.gce.instance_id.is_empty(),
        "Google JWT has no VM identity"
    );
    let digest = claims.submods.container.image_digest.as_bytes();
    ensure!(
        digest.len() == 71
            && digest.starts_with(b"sha256:")
            && digest[7..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte)),
        "Google JWT has no valid container image digest"
    );

    let mut segments = token.split('.');
    let _header = segments.next();
    let payload = segments.next().context("Google JWT has no payload")?;
    if segments.next().is_none() || segments.next().is_some() {
        bail!("Google JWT must have three segments");
    }
    let signed_payload = base64::decode_config(payload, URL_SAFE_NO_PAD)
        .context("invalid Google JWT payload encoding")?;
    ensure!(
        signed_payload.len() <= MAX_TOKEN_BYTES,
        "Google JWT payload is too large"
    );

    Ok(VerifiedGoogleSpaceClaims {
        image_digest: claims.submods.container.image_digest.into_bytes(),
        image_reference: claims.submods.container.image_reference.into_bytes(),
        project_id: claims.submods.gce.project_id.into_bytes(),
        instance_id: claims.submods.gce.instance_id.into_bytes(),
        subject: claims.sub.into_bytes(),
        audience: claims.aud.into_bytes(),
        nonce: expected_nonce.as_bytes().to_vec(),
        hwmodel: claims.hwmodel.into_bytes(),
        issued_at: claims.iat,
        not_before: claims.nbf,
        expires_at: claims.exp,
        signed_payload,
    })
}

fn native_verify_google_space_token(
    context: &mut SafeNativeContext,
    ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(ty_args.len(), 0);
    safely_assert_eq!(args.len(), 5);

    let now_secs = safely_pop_arg!(args, u64);
    let expected_nonce = safely_pop_arg!(args, Vec<u8>);
    let expected_audience = safely_pop_arg!(args, Vec<u8>);
    let trusted_jwks = safely_pop_arg!(args, Vec<u8>);
    let token = safely_pop_arg!(args, Vec<u8>);

    let input_len = token
        .len()
        .saturating_add(trusted_jwks.len())
        .saturating_add(expected_audience.len())
        .saturating_add(expected_nonce.len());
    context.charge(
        GOOGLE_SPACE_VERIFY_BASE + GOOGLE_SPACE_VERIFY_PER_BYTE * NumBytes::new(input_len as u64),
    )?;

    let verified = verify_google_space_token(
        &token,
        &trusted_jwks,
        &expected_audience,
        &expected_nonce,
        now_secs,
    )
    .map_err(|_| SafeNativeError::abort(EINVALID_ATTESTATION))?;
    Ok(smallvec![verified.into_move_value()])
}

pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    builder.make_named_natives([(
        "verify_google_space_token",
        native_verify_google_space_token as RawSafeNative,
    )])
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{EncodingKey, Header};
    use serde_json::{json, Value};

    const PRIVATE_KEY: &[u8] =
        include_bytes!("../../../../types/src/jwks/rsa/insecure_test_jwk_private_key.txt");
    const JWKS: &[u8] = include_bytes!("../../../../types/src/jwks/rsa/insecure_test_jwk.json");
    const AUDIENCE: &[u8] = b"aptos-attestation-test";
    const NONCE: &[u8] = b"0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    const NOW: u64 = 1_800_000_000;

    fn claims() -> Value {
        json!({
            "iss": GOOGLE_ISSUER,
            "aud": "aptos-attestation-test",
            "sub": "https://www.googleapis.com/compute/v1/projects/example/zones/us-central1-a/instances/123",
            "exp": NOW + 3600,
            "iat": NOW - 10,
            "nbf": NOW - 10,
            "eat_nonce": [std::str::from_utf8(NONCE).unwrap()],
            "oemid": 11129,
            "secboot": true,
            "hwmodel": "GCP_INTEL_TDX",
            "swname": "CONFIDENTIAL_SPACE",
            "dbgstat": "disabled-since-boot",
            "submods": {
                "container": {
                    "image_digest": format!("sha256:{}", "a".repeat(64)),
                    "image_reference": "example.com/workload:latest"
                },
                "gce": {"project_id": "example", "instance_id": "123"}
            }
        })
    }

    fn sign(claims: &Value) -> Vec<u8> {
        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some("test-rsa".to_owned());
        jsonwebtoken::encode(
            &header,
            claims,
            &EncodingKey::from_rsa_pem(PRIVATE_KEY).unwrap(),
        )
        .unwrap()
        .into_bytes()
    }

    fn verify(token: &[u8]) -> Result<VerifiedGoogleSpaceClaims> {
        verify_google_space_token(token, JWKS, AUDIENCE, NONCE, NOW)
    }

    #[test]
    fn valid_token_returns_signed_identity_and_payload() {
        let token = sign(&claims());
        let verified = verify(&token).unwrap();
        assert_eq!(verified.project_id, b"example");
        assert_eq!(verified.instance_id, b"123");
        assert_eq!(verified.hwmodel, b"GCP_INTEL_TDX");
        assert_eq!(verified.nonce, NONCE);
        assert_eq!(
            verified.image_digest,
            format!("sha256:{}", "a".repeat(64)).as_bytes()
        );
        let signed: Value = serde_json::from_slice(&verified.signed_payload).unwrap();
        assert_eq!(signed["submods"]["gce"]["project_id"], "example");
    }

    #[test]
    fn rejects_changed_signature_and_untrusted_key() {
        let mut token = sign(&claims());
        let payload_start = token.iter().position(|byte| *byte == b'.').unwrap() + 1;
        token[payload_start] = if token[payload_start] == b'A' {
            b'B'
        } else {
            b'A'
        };
        assert!(verify(&token).is_err());

        let token = sign(&claims());
        assert!(
            verify_google_space_token(&token, br#"{"keys":[]}"#, AUDIENCE, NONCE, NOW).is_err()
        );
    }

    #[test]
    fn rejects_wrong_audience_nonce_and_issuer() {
        let token = sign(&claims());
        assert!(verify_google_space_token(&token, JWKS, b"other", NONCE, NOW).is_err());
        assert!(verify_google_space_token(&token, JWKS, AUDIENCE, b"other", NOW).is_err());
        let mut claims = claims();
        claims["iss"] = json!("https://accounts.google.com");
        assert!(verify(&sign(&claims)).is_err());
    }

    #[test]
    fn rejects_expired_future_and_insecure_tokens() {
        let mut claims = claims();
        claims["exp"] = json!(NOW);
        assert!(verify(&sign(&claims)).is_err());
        claims["exp"] = json!(NOW + 3600);
        claims["iat"] = json!(NOW + 1);
        assert!(verify(&sign(&claims)).is_err());
        claims["iat"] = json!(NOW - 10);
        claims["dbgstat"] = json!("enabled");
        assert!(verify(&sign(&claims)).is_err());
        claims["dbgstat"] = json!("disabled-since-boot");
        claims["secboot"] = json!(false);
        assert!(verify(&sign(&claims)).is_err());
        claims["secboot"] = json!(true);
        claims["hwmodel"] = json!("GCP_SHIELDED_VM");
        assert!(verify(&sign(&claims)).is_err());
    }
}
