//! Offline verification of NVIDIA NRAS detached EAT bundles.
//!
//! NVIDIA publishes the EAT signing keys at
//! `https://nras.attestation.nvidia.com/.well-known/jwks.json`. The caller
//! supplies a trusted snapshot of that JWKS; this module never fetches keys or
//! reads a wall clock. The format and validation rules follow NVIDIA's
//! `nvat_verify_attestation_result` API and the Atlas `[tee]` harness.
//!
//! An EAT with the same nonce as a TDX quote is fresh for that challenge. It
//! does not, on its own, prove that NRAS evaluated the exact raw GPU reports in
//! the quote-bound Tinfoil device section. The producer must establish that
//! relationship, or a verifier must additionally match device identities.

use crate::{envelope::reject_duplicate_members, NONCE_SIZE};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ring::signature::{UnparsedPublicKey, ECDSA_P384_SHA384_FIXED};
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const MAX_EAT_BYTES: usize = 4 << 20;
const MAX_JWKS_BYTES: usize = 60 << 10;
const MAX_GPUS: u32 = 32;
const ISSUER: &str = "https://nras.attestation.nvidia.com";
const SUBJECT: &str = "NVIDIA-PLATFORM-ATTESTATION";

/// NVIDIA-authenticated GPU details. The count is checked against the signed
/// VM shape supplied by the caller, not merely copied from the EAT.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGpu {
    pub gpu_count: u32,
    pub devices: Vec<VerifiedGpuDevice>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerifiedGpuDevice {
    /// The `GPU-N` index in the NRAS detached EAT.
    pub index: u32,
    pub ueid: String,
    pub hwmodel: String,
}

#[derive(Debug, thiserror::Error, Eq, PartialEq)]
pub enum NrasError {
    #[error("NRAS EAT or JWKS is malformed: {0}")]
    Malformed(String),
    #[error("NRAS signing key is missing or invalid")]
    InvalidKey,
    #[error("NRAS JWT signature is invalid")]
    InvalidSignature,
    #[error("NRAS JWT issuer or time is invalid")]
    InvalidClaims,
    #[error("NRAS EAT nonce does not match the expected challenge")]
    NonceMismatch,
    #[error("NRAS EAT submodule digest or GPU set does not match")]
    SubmoduleMismatch,
    #[error("NRAS GPU appraisal policy failed")]
    Policy,
}

struct Jwk {
    kid: Option<String>,
    public_key: [u8; 97],
}

/// Verify a production NRAS detached EAT against a caller-authenticated JWKS.
///
/// `expected_nonce` is the 32 raw challenge bytes whose lowercase hex encoding
/// was passed to `nvattest --nonce`; it must also be bound to authenticated CPU
/// evidence by the calling protocol. `now_secs` must come from chain time.
/// The accepted bundle is `[ ["JWT", overall], {"GPU-0": jwt, ...} ]` or Atlas's
/// legacy SDK wrapper with this bundle under `REMOTE_GPU_CLAIMS`. Only the
/// inner NVIDIA-signed JWTs are trusted in the wrapped case.
pub fn verify_detached_eat(
    eat_json: &[u8],
    jwks_json: &[u8],
    expected_nonce: &[u8],
    now_secs: u64,
    expected_gpus: u32,
) -> Result<VerifiedGpu, NrasError> {
    if expected_nonce.len() != NONCE_SIZE || expected_gpus == 0 || expected_gpus > MAX_GPUS {
        return malformed("invalid nonce length or GPU count");
    }
    if eat_json.is_empty()
        || eat_json.len() > MAX_EAT_BYTES
        || jwks_json.is_empty()
        || jwks_json.len() > MAX_JWKS_BYTES
    {
        return malformed("EAT or JWKS is empty or too large");
    }
    let eat = parse_json(eat_json)?;
    let (overall_jwt, detached) = select_bundle(&eat, false)?;
    let keys = parse_jwks(jwks_json)?;
    let expected_nonce_hex = hex::encode(expected_nonce);

    let overall = verify_jwt(overall_jwt, &keys, now_secs)?;
    if claim_str(&overall, "sub") != Some(SUBJECT)
        || !matches!(
            claim_str(&overall, "x-nvidia-ver"),
            Some("2.0" | "3.0" | "4.0")
        )
        || overall.get("x-nvidia-overall-att-result") != Some(&Value::Bool(true))
    {
        return Err(NrasError::Policy);
    }
    check_nonce(&overall, &expected_nonce_hex)?;
    let submods = overall
        .get("submods")
        .and_then(Value::as_object)
        .ok_or(NrasError::SubmoduleMismatch)?;
    if detached.len() != expected_gpus as usize || submods.len() != detached.len() {
        return Err(NrasError::SubmoduleMismatch);
    }

    let mut devices = Vec::with_capacity(detached.len());
    let mut seen_ueids = BTreeSet::new();
    for index in 0..expected_gpus {
        let name = format!("GPU-{index}");
        let token = detached
            .get(&name)
            .and_then(Value::as_str)
            .ok_or(NrasError::SubmoduleMismatch)?;
        let digest_claim = submods.get(&name).ok_or(NrasError::SubmoduleMismatch)?;
        check_submodule_digest(digest_claim, token)?;

        let claims = verify_jwt(token, &keys, now_secs)?;
        check_nonce(&claims, &expected_nonce_hex)?;
        check_device_policy(&claims)?;
        let ueid = claim_str(&claims, "ueid")
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or(NrasError::Policy)?;
        let hwmodel = claim_str(&claims, "hwmodel")
            .filter(|s| !s.is_empty() && s.len() <= 256)
            .ok_or(NrasError::Policy)?;
        if !seen_ueids.insert(ueid.to_owned()) {
            return Err(NrasError::SubmoduleMismatch);
        }
        devices.push(VerifiedGpuDevice {
            index,
            ueid: ueid.to_owned(),
            hwmodel: hwmodel.to_owned(),
        });
    }
    Ok(VerifiedGpu {
        gpu_count: expected_gpus,
        devices,
    })
}

fn parse_json(bytes: &[u8]) -> Result<Value, NrasError> {
    reject_duplicate_members(bytes).map_err(|e| NrasError::Malformed(e.to_string()))?;
    serde_json::from_slice(bytes).map_err(|e| NrasError::Malformed(e.to_string()))
}

fn select_bundle<'a>(
    value: &'a Value,
    already_wrapped: bool,
) -> Result<(&'a str, &'a Map<String, Value>), NrasError> {
    let parts = value
        .as_array()
        .filter(|v| v.len() == 2)
        .ok_or_else(|| NrasError::Malformed("expected a two-element detached EAT".into()))?;
    let head = parts[0]
        .as_array()
        .filter(|v| v.len() == 2 && v[0].as_str() == Some("JWT"))
        .ok_or_else(|| NrasError::Malformed("missing overall JWT".into()))?;
    let jwt = head[1]
        .as_str()
        .ok_or_else(|| NrasError::Malformed("overall JWT is not a string".into()))?;
    let tail = parts[1]
        .as_object()
        .ok_or_else(|| NrasError::Malformed("detached claims are not an object".into()))?;
    if let Some(inner) = tail.get("REMOTE_GPU_CLAIMS") {
        if already_wrapped || tail.len() != 1 {
            return malformed("invalid SDK wrapper");
        }
        return select_bundle(inner, true);
    }
    if tail.is_empty() {
        return malformed("no detached GPU JWTs");
    }
    Ok((jwt, tail))
}

fn parse_jwks(bytes: &[u8]) -> Result<Vec<Jwk>, NrasError> {
    let document = parse_json(bytes)?;
    let keys = document
        .get("keys")
        .and_then(Value::as_array)
        .ok_or(NrasError::InvalidKey)?;
    if keys.is_empty() || keys.len() > 32 {
        return Err(NrasError::InvalidKey);
    }
    let mut out = Vec::new();
    let mut seen_kids = BTreeSet::new();
    for value in keys {
        let object = value.as_object().ok_or(NrasError::InvalidKey)?;
        if object.get("kty").and_then(Value::as_str) != Some("EC")
            || object.get("crv").and_then(Value::as_str) != Some("P-384")
        {
            continue;
        }
        if object
            .get("alg")
            .is_some_and(|v| v.as_str() != Some("ES384"))
            || object.get("use").is_some_and(|v| v.as_str() != Some("sig"))
        {
            continue;
        }
        if let Some(operations) = object.get("key_ops") {
            let operations = operations.as_array().ok_or(NrasError::InvalidKey)?;
            if !operations.iter().any(|v| v.as_str() == Some("verify")) {
                continue;
            }
        }
        let kid = match object.get("kid") {
            None => None,
            Some(Value::String(kid)) if !kid.is_empty() => Some(kid.clone()),
            Some(_) => return Err(NrasError::InvalidKey),
        };
        if let Some(kid) = &kid
            && !seen_kids.insert(kid.clone())
        {
            return Err(NrasError::InvalidKey);
        }
        let x = jwk_coordinate(object, "x")?;
        let y = jwk_coordinate(object, "y")?;
        let mut public_key = [0u8; 97];
        public_key[0] = 4;
        public_key[1..49].copy_from_slice(&x);
        public_key[49..].copy_from_slice(&y);
        out.push(Jwk { kid, public_key });
    }
    if out.is_empty() {
        return Err(NrasError::InvalidKey);
    }
    Ok(out)
}

fn jwk_coordinate(object: &Map<String, Value>, name: &str) -> Result<[u8; 48], NrasError> {
    let encoded = object
        .get(name)
        .and_then(Value::as_str)
        .ok_or(NrasError::InvalidKey)?;
    let decoded = decode_base64url(encoded).map_err(|_| NrasError::InvalidKey)?;
    decoded.try_into().map_err(|_| NrasError::InvalidKey)
}

fn verify_jwt(jwt: &str, keys: &[Jwk], now_secs: u64) -> Result<Map<String, Value>, NrasError> {
    let mut parts = jwt.split('.');
    let header = parts.next().ok_or(NrasError::InvalidSignature)?;
    let payload = parts.next().ok_or(NrasError::InvalidSignature)?;
    let signature = parts.next().ok_or(NrasError::InvalidSignature)?;
    if parts.next().is_some() || header.is_empty() || payload.is_empty() {
        return Err(NrasError::InvalidSignature);
    }
    let header_json = decode_base64url(header).map_err(|_| NrasError::InvalidSignature)?;
    let header = parse_json(&header_json)?;
    if header.get("alg").and_then(Value::as_str) != Some("ES384")
        || header.get("crit").is_some()
        || header.get("b64") == Some(&Value::Bool(false))
    {
        return Err(NrasError::InvalidSignature);
    }
    let kid = match header.get("kid") {
        None => None,
        Some(Value::String(kid)) if !kid.is_empty() => Some(kid.as_str()),
        Some(_) => return Err(NrasError::InvalidSignature),
    };
    let signature = decode_base64url(signature).map_err(|_| NrasError::InvalidSignature)?;
    if signature.len() != 96 {
        return Err(NrasError::InvalidSignature);
    }
    let (signing_input, _) = jwt.rsplit_once('.').ok_or(NrasError::InvalidSignature)?;
    if !keys
        .iter()
        .filter(|key| kid.is_none_or(|id| key.kid.as_deref() == Some(id)))
        .any(|key| {
            UnparsedPublicKey::new(&ECDSA_P384_SHA384_FIXED, &key.public_key)
                .verify(signing_input.as_bytes(), &signature)
                .is_ok()
        })
    {
        return Err(NrasError::InvalidSignature);
    }
    let payload_json = decode_base64url(payload).map_err(|_| NrasError::InvalidClaims)?;
    let claims = parse_json(&payload_json)?;
    let claims = claims.as_object().ok_or(NrasError::InvalidClaims)?.clone();
    check_claims(&claims, now_secs)?;
    Ok(claims)
}

fn decode_base64url(value: &str) -> Result<Vec<u8>, NrasError> {
    let decoded = URL_SAFE_NO_PAD
        .decode(value)
        .map_err(|_| NrasError::Malformed("invalid base64url".into()))?;
    if URL_SAFE_NO_PAD.encode(&decoded) != value {
        return malformed("noncanonical base64url");
    }
    Ok(decoded)
}

fn check_claims(claims: &Map<String, Value>, now_secs: u64) -> Result<(), NrasError> {
    if claim_str(claims, "iss") != Some(ISSUER) {
        return Err(NrasError::InvalidClaims);
    }
    let iat = claims
        .get("iat")
        .and_then(Value::as_u64)
        .ok_or(NrasError::InvalidClaims)?;
    let nbf = claims
        .get("nbf")
        .and_then(Value::as_u64)
        .ok_or(NrasError::InvalidClaims)?;
    let exp = claims
        .get("exp")
        .and_then(Value::as_u64)
        .ok_or(NrasError::InvalidClaims)?;
    if iat > now_secs || nbf > now_secs || exp <= now_secs || iat >= exp || nbf >= exp {
        return Err(NrasError::InvalidClaims);
    }
    Ok(())
}

fn check_nonce(claims: &Map<String, Value>, expected_hex: &str) -> Result<(), NrasError> {
    if claim_str(claims, "eat_nonce") != Some(expected_hex) {
        return Err(NrasError::NonceMismatch);
    }
    Ok(())
}

fn check_submodule_digest(value: &Value, jwt: &str) -> Result<(), NrasError> {
    let digest = value
        .as_array()
        .filter(|parts| parts.len() == 2 && parts[0].as_str() == Some("DIGEST"))
        .and_then(|parts| parts[1].as_array())
        .filter(|parts| parts.len() == 2 && parts[0].as_str() == Some("SHA-256"))
        .and_then(|parts| parts[1].as_str())
        .ok_or(NrasError::SubmoduleMismatch)?;
    if digest != hex::encode(Sha256::digest(jwt.as_bytes())) {
        return Err(NrasError::SubmoduleMismatch);
    }
    Ok(())
}

fn check_device_policy(claims: &Map<String, Value>) -> Result<(), NrasError> {
    const REQUIRED_TRUE: &[&str] = &[
        "secboot",
        "x-nvidia-gpu-attestation-report-nonce-match",
        "x-nvidia-gpu-attestation-report-signature-verified",
    ];
    if claim_str(claims, "measres") != Some("success")
        || claim_str(claims, "dbgstat") != Some("disabled")
        || REQUIRED_TRUE
            .iter()
            .any(|name| claims.get(*name) != Some(&Value::Bool(true)))
        || claims
            .get("x-nvidia-attestation-warning")
            .is_some_and(|warning| !warning.is_null() && warning != &Value::Bool(false))
    {
        return Err(NrasError::Policy);
    }
    Ok(())
}

fn claim_str<'a>(claims: &'a Map<String, Value>, name: &str) -> Option<&'a str> {
    claims.get(name).and_then(Value::as_str)
}

fn malformed<T>(message: &str) -> Result<T, NrasError> {
    Err(NrasError::Malformed(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use ring::{
        rand::SystemRandom,
        signature::{EcdsaKeyPair, KeyPair, ECDSA_P384_SHA384_FIXED_SIGNING},
    };
    use serde_json::json;

    const NOW: u64 = 1_800_000_000;
    const NONCE: [u8; 32] = [0x42; 32];

    struct Fixture {
        key: EcdsaKeyPair,
        jwks: Vec<u8>,
    }

    impl Fixture {
        fn new() -> Self {
            let random = SystemRandom::new();
            let private = EcdsaKeyPair::generate_pkcs8(&ECDSA_P384_SHA384_FIXED_SIGNING, &random)
                .expect("generate test P-384 key");
            let key = EcdsaKeyPair::from_pkcs8(
                &ECDSA_P384_SHA384_FIXED_SIGNING,
                private.as_ref(),
                &random,
            )
            .expect("parse test P-384 key");
            let point = key.public_key().as_ref();
            assert_eq!(point.len(), 97);
            let jwks = serde_json::to_vec(&json!({
                "keys": [{
                    "kty": "EC", "crv": "P-384", "kid": "test-nras-key",
                    "alg": "ES384", "use": "sig",
                    "x": URL_SAFE_NO_PAD.encode(&point[1..49]),
                    "y": URL_SAFE_NO_PAD.encode(&point[49..]),
                }]
            }))
            .expect("serialize test JWKS");
            Self { key, jwks }
        }

        fn sign(&self, claims: &Value) -> String {
            let header = URL_SAFE_NO_PAD.encode(
                serde_json::to_vec(&json!({"alg":"ES384","kid":"test-nras-key"}))
                    .expect("serialize header"),
            );
            let payload =
                URL_SAFE_NO_PAD.encode(serde_json::to_vec(claims).expect("serialize claims"));
            let input = format!("{header}.{payload}");
            let signature = self
                .key
                .sign(&SystemRandom::new(), input.as_bytes())
                .expect("sign test JWT");
            format!("{input}.{}", URL_SAFE_NO_PAD.encode(signature.as_ref()))
        }

        fn device(&self, index: u32) -> Value {
            json!({
                "iss": ISSUER, "iat": NOW-10, "nbf": NOW-10, "exp": NOW+3600,
                "eat_nonce": hex::encode(NONCE),
                "ueid": format!("nvidia-device-{index}"), "hwmodel": "GH100",
                "measres": "success", "secboot": true, "dbgstat": "disabled",
                "x-nvidia-gpu-attestation-report-nonce-match": true,
                "x-nvidia-gpu-attestation-report-signature-verified": true,
                "x-nvidia-attestation-warning": null,
            })
        }

        fn overall(&self, submods: Value) -> Value {
            json!({
                "iss": ISSUER, "sub": SUBJECT, "x-nvidia-ver": "2.0",
                "iat": NOW-10, "nbf": NOW-10, "exp": NOW+3600,
                "eat_nonce": hex::encode(NONCE),
                "x-nvidia-overall-att-result": true,
                "submods": submods,
            })
        }

        fn bundle(&self, device_claims: &[Value]) -> Value {
            let mut detached = Map::new();
            let mut submods = Map::new();
            for (index, claims) in device_claims.iter().enumerate() {
                let name = format!("GPU-{index}");
                let jwt = self.sign(claims);
                submods.insert(
                    name.clone(),
                    json!(["DIGEST", [
                        "SHA-256",
                        hex::encode(Sha256::digest(jwt.as_bytes()))
                    ]]),
                );
                detached.insert(name, Value::String(jwt));
            }
            json!([
                ["JWT", self.sign(&self.overall(Value::Object(submods)))],
                detached
            ])
        }

        fn verify(&self, bundle: &Value, expected_gpus: u32) -> Result<VerifiedGpu, NrasError> {
            verify_detached_eat(
                &serde_json::to_vec(bundle).expect("serialize EAT"),
                &self.jwks,
                &NONCE,
                NOW,
                expected_gpus,
            )
        }
    }

    #[test]
    fn verifies_flat_and_atlas_sdk_wrapped_eat() {
        let fixture = Fixture::new();
        let bundle = fixture.bundle(&[fixture.device(0), fixture.device(1)]);
        let verified = fixture.verify(&bundle, 2).expect("valid NRAS EAT");
        assert_eq!(verified.gpu_count, 2);
        assert_eq!(verified.devices[1].ueid, "nvidia-device-1");

        // Atlas's pre-nvattest report nests the NVIDIA bundle below an
        // unauthenticated local HS256 SDK wrapper. Only inner JWTs are used.
        let wrapped = json!([["JWT", "a.b.c"], {"REMOTE_GPU_CLAIMS": bundle}]);
        assert_eq!(fixture.verify(&wrapped, 2).expect("wrapped EAT"), verified);
    }

    #[test]
    fn rejects_changed_signature_and_untrusted_jwks() {
        let fixture = Fixture::new();
        let mut bundle = fixture.bundle(&[fixture.device(0)]);
        let token = bundle[1]["GPU-0"].as_str().expect("GPU JWT");
        let mut changed = token.as_bytes().to_vec();
        let last = changed.len() - 1;
        changed[last] = if changed[last] == b'A' { b'B' } else { b'A' };
        bundle[1]["GPU-0"] = Value::String(String::from_utf8(changed).expect("ASCII JWT"));
        assert_eq!(
            fixture.verify(&bundle, 1),
            Err(NrasError::SubmoduleMismatch)
        );

        let other = Fixture::new();
        let bundle = fixture.bundle(&[fixture.device(0)]);
        assert_eq!(other.verify(&bundle, 1), Err(NrasError::InvalidSignature));
    }

    #[test]
    fn rejects_rehashed_but_bad_device_signature() {
        let fixture = Fixture::new();
        let mut bundle = fixture.bundle(&[fixture.device(0)]);
        let token = bundle[1]["GPU-0"].as_str().expect("GPU JWT");
        let mut changed = token.as_bytes().to_vec();
        let last = changed.len() - 1;
        changed[last] = if changed[last] == b'A' { b'B' } else { b'A' };
        let changed = String::from_utf8(changed).expect("ASCII JWT");
        bundle[1]["GPU-0"] = Value::String(changed.clone());
        bundle[0][1] = Value::String(fixture.sign(&fixture.overall(json!({
            "GPU-0": ["DIGEST", ["SHA-256", hex::encode(Sha256::digest(changed.as_bytes()))]]
        }))));
        assert_eq!(fixture.verify(&bundle, 1), Err(NrasError::InvalidSignature));
    }

    #[test]
    fn rejects_wrong_nonce_issuer_and_time() {
        let fixture = Fixture::new();
        let mut device = fixture.device(0);
        device["eat_nonce"] = json!(hex::encode([0x43; 32]));
        assert_eq!(
            fixture.verify(&fixture.bundle(&[device]), 1),
            Err(NrasError::NonceMismatch)
        );

        let mut device = fixture.device(0);
        device["iss"] = json!("https://attacker.example");
        assert_eq!(
            fixture.verify(&fixture.bundle(&[device]), 1),
            Err(NrasError::InvalidClaims)
        );

        for claim in ["iat", "nbf", "exp"] {
            let mut device = fixture.device(0);
            device[claim] = json!(if claim == "exp" { NOW } else { NOW + 1 });
            assert_eq!(
                fixture.verify(&fixture.bundle(&[device]), 1),
                Err(NrasError::InvalidClaims)
            );
        }
    }

    #[test]
    fn rejects_failed_gpu_policy_even_with_true_overall_result() {
        let fixture = Fixture::new();
        for (claim, failed) in [
            ("measres", json!("fail")),
            ("secboot", json!(false)),
            ("dbgstat", json!("enabled")),
            ("x-nvidia-gpu-attestation-report-nonce-match", json!(false)),
            (
                "x-nvidia-gpu-attestation-report-signature-verified",
                json!(false),
            ),
            ("x-nvidia-attestation-warning", json!(true)),
        ] {
            let mut device = fixture.device(0);
            device[claim] = failed;
            assert_eq!(
                fixture.verify(&fixture.bundle(&[device]), 1),
                Err(NrasError::Policy),
                "{claim}"
            );
        }
    }

    #[test]
    fn rejects_missing_extra_or_duplicate_gpu_identity() {
        let fixture = Fixture::new();
        let one = fixture.bundle(&[fixture.device(0)]);
        assert_eq!(fixture.verify(&one, 2), Err(NrasError::SubmoduleMismatch));
        assert_eq!(
            fixture.verify(&one, 0),
            Err(NrasError::Malformed(
                "invalid nonce length or GPU count".into()
            ))
        );

        let mut duplicate = fixture.device(1);
        duplicate["ueid"] = json!("nvidia-device-0");
        let bundle = fixture.bundle(&[fixture.device(0), duplicate]);
        assert_eq!(
            fixture.verify(&bundle, 2),
            Err(NrasError::SubmoduleMismatch)
        );

        let mut extra = fixture.bundle(&[fixture.device(0)]);
        extra[1]["GPU-7"] = extra[1]["GPU-0"].clone();
        assert_eq!(fixture.verify(&extra, 1), Err(NrasError::SubmoduleMismatch));
    }

    #[test]
    fn rejects_duplicate_json_claims_and_unsupported_algorithm() {
        let fixture = Fixture::new();
        let bundle = fixture.bundle(&[fixture.device(0)]);
        let eat = serde_json::to_vec(&bundle).expect("serialize EAT");
        let jwks = br#"{"keys":[],"keys":[]}"#;
        assert!(matches!(
            verify_detached_eat(&eat, jwks, &NONCE, NOW, 1),
            Err(NrasError::Malformed(_))
        ));

        let bad_header = URL_SAFE_NO_PAD.encode(br#"{"alg":"none","kid":"test-nras-key"}"#);
        let overall = bundle[0][1].as_str().expect("overall JWT");
        let (_, rest) = overall.split_once('.').expect("JWT dot");
        let rest = rest.to_owned();
        let mut bad = bundle;
        bad[0][1] = json!(format!("{bad_header}.{rest}"));
        assert_eq!(fixture.verify(&bad, 1), Err(NrasError::InvalidSignature));
    }
}
