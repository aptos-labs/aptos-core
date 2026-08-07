// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Native implementation of AWS Nitro Enclave attestation verification.
//!
//! Verifies attestation documents produced by AWS Nitro Enclaves (COSE Sign1 format)
//! by validating the certificate chain and signature against AWS root of trust.

use anyhow::{anyhow, bail, Result};
use aptos_gas_schedule::gas_params::natives::aptos_framework::*;
use aptos_native_interface::{
    safely_assert_eq, safely_pop_arg, RawSafeNative, SafeNativeBuilder, SafeNativeContext,
    SafeNativeResult,
};
use aws_nitro_enclaves_cose::{
    crypto::{Hash, MessageDigest, SignatureAlgorithm, SigningPublicKey},
    error::CoseError,
    CoseSign1,
};
use aws_nitro_enclaves_nsm_api::api::{AttestationDoc, Digest};
use ecdsa::signature::hazmat::PrehashVerifier;
use move_core_types::{
    gas_algebra::NumBytes,
    language_storage::{OPTION_NONE_TAG, OPTION_SOME_TAG},
};
use move_vm_runtime::native_functions::NativeFunction;
use move_vm_types::{
    loaded_data::runtime_types::Type,
    values::{Struct, Value},
};
use sha2_0_10_6::{Digest as Sha2Digest, Sha256, Sha384, Sha512};
use smallvec::{smallvec, SmallVec};
use std::collections::VecDeque;
use webpki::{EndEntityCert, TrustAnchor};
use x509_parser::x509::SubjectPublicKeyInfo;

pub(crate) static SUPPORTED_SIG_ALGS: &[&webpki::SignatureAlgorithm] = &[
    &webpki::ECDSA_P256_SHA256,
    &webpki::ECDSA_P256_SHA384,
    &webpki::ECDSA_P384_SHA256,
    &webpki::ECDSA_P384_SHA384,
    &webpki::ED25519,
    &webpki::RSA_PSS_2048_8192_SHA256_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA384_LEGACY_KEY,
    &webpki::RSA_PSS_2048_8192_SHA512_LEGACY_KEY,
    &webpki::RSA_PKCS1_2048_8192_SHA256,
    &webpki::RSA_PKCS1_2048_8192_SHA384,
    &webpki::RSA_PKCS1_2048_8192_SHA512,
    &webpki::RSA_PKCS1_3072_8192_SHA384,
];

fn option_some(context: &SafeNativeContext, value: Value) -> SafeNativeResult<Value> {
    let enum_option = context.get_feature_flags().is_enum_option_enabled();
    Ok(if enum_option {
        Value::struct_(Struct::pack_variant(OPTION_SOME_TAG, vec![value]))
    } else {
        Value::struct_(Struct::pack(vec![Value::vector_unchecked(vec![value])?]))
    })
}

fn option_none(context: &SafeNativeContext) -> SafeNativeResult<Value> {
    let enum_option = context.get_feature_flags().is_enum_option_enabled();
    Ok(if enum_option {
        Value::struct_(Struct::pack_variant(OPTION_NONE_TAG, vec![]))
    } else {
        Value::struct_(Struct::pack(vec![Value::vector_unchecked(vec![])?]))
    })
}

fn digest_name(digest: Digest) -> &'static [u8] {
    match digest {
        Digest::SHA256 => b"SHA256",
        Digest::SHA384 => b"SHA384",
        Digest::SHA512 => b"SHA512",
    }
}

struct NitroCryptoClient;

impl Hash for NitroCryptoClient {
    fn hash(digest: MessageDigest, data: &[u8]) -> std::result::Result<Vec<u8>, CoseError> {
        Ok(match digest {
            MessageDigest::Sha256 => Sha256::digest(data).to_vec(),
            MessageDigest::Sha384 => Sha384::digest(data).to_vec(),
            MessageDigest::Sha512 => Sha512::digest(data).to_vec(),
        })
    }
}

macro_rules! verify_cose {
    ($op:expr) => {
        $op.map_err(|_| CoseError::UnverifiedSignature)
    };
}

macro_rules! verify_signature {
    ($self:ident, $curve:ident, $digest:ident, $signature:ident) => {{
        let encoded_point: $curve::EncodedPoint =
            verify_cose!($curve::EncodedPoint::from_bytes($self.public_key()))?;
        let verifying_key: $curve::ecdsa::VerifyingKey = verify_cose!(
            $curve::ecdsa::VerifyingKey::from_encoded_point(&encoded_point,)
        )?;
        let sig = verify_cose!($curve::ecdsa::Signature::from_slice($signature))?;
        verify_cose!(verifying_key.verify_prehash($digest, &sig))
    }};
}

pub(crate) struct NitroPublicKey<'a> {
    spki: &'a SubjectPublicKeyInfo<'a>,
}

impl<'a> NitroPublicKey<'a> {
    pub(crate) fn new(spki: &'a SubjectPublicKeyInfo<'a>) -> Result<Self> {
        if spki.algorithm.algorithm.to_id_string() != "1.2.840.10045.2.1" {
            bail!("attestation signing certificate does not contain an EC public key");
        }
        Ok(Self { spki })
    }

    fn public_key(&self) -> &[u8] {
        self.spki.subject_public_key.as_ref()
    }

    fn verify_p256_signature(&self, digest: &[u8], signature: &[u8]) -> Result<(), CoseError> {
        verify_signature!(self, p256, digest, signature)
    }

    fn verify_p384_signature(&self, digest: &[u8], signature: &[u8]) -> Result<(), CoseError> {
        verify_signature!(self, p384, digest, signature)
    }
}

impl SigningPublicKey for NitroPublicKey<'_> {
    fn get_parameters(
        &self,
    ) -> std::result::Result<(SignatureAlgorithm, MessageDigest), CoseError> {
        let parameters = self.spki.algorithm.parameters.as_ref().ok_or_else(|| {
            CoseError::UnsupportedError("EC public key parameters are missing".to_string())
        })?;
        let curve_oid = parameters.as_oid().map_err(|_| {
            CoseError::UnsupportedError(
                "EC public key parameters must be a named curve".to_string(),
            )
        })?;

        match curve_oid.to_id_string().as_str() {
            "1.2.840.10045.3.1.7" => Ok((SignatureAlgorithm::ES256, MessageDigest::Sha256)),
            "1.3.132.0.34" => Ok((SignatureAlgorithm::ES384, MessageDigest::Sha384)),
            "1.3.132.0.35" => Ok((SignatureAlgorithm::ES512, MessageDigest::Sha512)),
            oid => Err(CoseError::UnsupportedError(format!(
                "unsupported EC curve: {oid}"
            ))),
        }
    }

    fn verify(&self, digest: &[u8], signature: &[u8]) -> std::result::Result<bool, CoseError> {
        let (sig_alg, _) = self.get_parameters()?;
        let result = match sig_alg {
            SignatureAlgorithm::ES256 => self.verify_p256_signature(digest, signature),
            SignatureAlgorithm::ES384 => self.verify_p384_signature(digest, signature),
            SignatureAlgorithm::ES512 => Err(CoseError::UnsupportedError(
                "P-521 Nitro attestation signatures are not supported".to_string(),
            )),
        };
        result.map(|()| true).or_else(|_| Ok(false))
    }
}

fn total_attestation_and_root_bytes(attestation_bytes: &[u8], trusted_roots: &[Vec<u8>]) -> u64 {
    trusted_roots
        .iter()
        .fold(attestation_bytes.len() as u64, |total, root| {
            total.saturating_add(root.len().max(1) as u64)
        })
}

pub(crate) fn validate_cert_trust_chain_with_roots(
    target: &[u8],
    intermediates: &[&[u8]],
    trusted_root_certs: &[Vec<u8>],
    unix_time_secs: u64,
) -> Result<()> {
    if trusted_root_certs.is_empty() {
        bail!("no trusted root certificates configured");
    }

    let end_entity_cert =
        EndEntityCert::try_from(target).map_err(|_| anyhow!("invalid end-entity certificate"))?;
    let trust_anchors = trusted_root_certs
        .iter()
        .map(|root| {
            TrustAnchor::try_from_cert_der(root.as_slice())
                .map_err(|_| anyhow!("invalid trusted root certificate"))
        })
        .collect::<Result<Vec<_>>>()?;
    let server_trust_anchors = webpki::TlsServerTrustAnchors(&trust_anchors);
    let time = webpki::Time::from_seconds_since_unix_epoch(unix_time_secs);

    end_entity_cert
        .verify_is_valid_tls_server_cert(
            SUPPORTED_SIG_ALGS,
            &server_trust_anchors,
            intermediates,
            time,
        )
        .map_err(|err| anyhow!("invalid Nitro attestation certificate chain: {err:?}"))?;

    Ok(())
}

pub(crate) fn validate_cose_signature(
    signing_cert_public_key: &dyn SigningPublicKey,
    cose_sign_1_decoded: &CoseSign1,
) -> Result<()> {
    if cose_sign_1_decoded
        .verify_signature::<NitroCryptoClient>(signing_cert_public_key)
        .map_err(|err| anyhow!("invalid Nitro attestation COSE envelope: {err}"))?
    {
        Ok(())
    } else {
        bail!("invalid Nitro attestation COSE signature")
    }
}

pub(crate) fn validate_and_parse_attestation_doc_with_roots(
    attestation_bytes: &[u8],
    trusted_root_certs: &[Vec<u8>],
    unix_time_secs: u64,
) -> Result<AttestationDoc> {
    let (cose_sign_1_decoded, decoded_attestation_doc) =
        attestation_doc_validation::attestation_doc::decode_attestation_document(attestation_bytes)
            .map_err(|err| anyhow!("invalid Nitro attestation document: {err}"))?;

    let intermediate_certs = decoded_attestation_doc
        .cabundle
        .iter()
        .map(|cert| cert.as_slice())
        .collect::<Vec<_>>();
    validate_cert_trust_chain_with_roots(
        &decoded_attestation_doc.certificate,
        &intermediate_certs,
        trusted_root_certs,
        unix_time_secs,
    )?;

    let attestation_doc_signing_cert =
        attestation_doc_validation::parse_cert(&decoded_attestation_doc.certificate)
            .map_err(|err| anyhow!("invalid Nitro attestation signing certificate: {err}"))?;
    let pub_key = NitroPublicKey::new(attestation_doc_signing_cert.public_key())?;
    validate_cose_signature(&pub_key, &cose_sign_1_decoded)?;

    Ok(decoded_attestation_doc)
}

fn attestation_doc_to_move_value(
    context: &SafeNativeContext,
    doc: &AttestationDoc,
) -> SafeNativeResult<Value> {
    let module_id = Value::vector_u8(doc.module_id.as_bytes().to_vec());
    let timestamp = Value::u64(doc.timestamp);
    let digest = Value::vector_u8(digest_name(doc.digest).to_vec());
    let pcrs_vec: Vec<Value> = doc
        .pcrs
        .iter()
        .map(|(idx, val)| {
            Value::struct_(Struct::pack(vec![
                Value::u8((*idx) as u8),
                Value::vector_u8(val.to_vec()),
            ]))
        })
        .collect();
    let pcrs_value = Value::vector_unchecked(pcrs_vec)?;
    let certificate = Value::vector_u8(doc.certificate.to_vec());
    let user_data = match &doc.user_data {
        Some(b) => option_some(context, Value::vector_u8(b.to_vec()))?,
        None => option_none(context)?,
    };
    let nonce = match &doc.nonce {
        Some(b) => option_some(context, Value::vector_u8(b.to_vec()))?,
        None => option_none(context)?,
    };
    let public_key = match &doc.public_key {
        Some(b) => option_some(context, Value::vector_u8(b.to_vec()))?,
        None => option_none(context)?,
    };

    Ok(Value::struct_(Struct::pack(vec![
        module_id,
        timestamp,
        digest,
        pcrs_value,
        certificate,
        user_data,
        nonce,
        public_key,
    ])))
}

/***************************************************************************************************
 * native fun verify_attestation_with_roots
 *
 *   gas cost: base_cost + unit_cost * (attestation_bytes_len + trusted_root_bytes_len)
 *
 *   Same as verify_attestation, but uses caller-provided DER root certificates and an explicit
 *   Unix timestamp for certificate validity checks.
 **************************************************************************************************/
fn native_verify_attestation_with_roots(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);

    let unix_time_secs = safely_pop_arg!(args, u64);
    let mut trusted_root_certs = vec![];
    for root in safely_pop_arg!(args, Vec<Value>) {
        trusted_root_certs.push(root.value_as::<Vec<u8>>()?);
    }
    let attestation_bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = AWS_NITRO_VERIFY_ATTESTATION_BASE
        + AWS_NITRO_VERIFY_ATTESTATION_PER_BYTE
            * NumBytes::new(total_attestation_and_root_bytes(
                &attestation_bytes,
                &trusted_root_certs,
            ));
    context.charge(cost)?;

    let valid = validate_and_parse_attestation_doc_with_roots(
        &attestation_bytes,
        &trusted_root_certs,
        unix_time_secs,
    )
    .is_ok();

    Ok(smallvec![Value::bool(valid)])
}

/***************************************************************************************************
 * native fun verify_and_parse_attestation_with_roots
 *
 *   gas cost: base_cost + unit_cost * (attestation_bytes_len + trusted_root_bytes_len)
 *
 *   Same as verify_and_parse_attestation, but uses caller-provided DER root certificates and an
 *   explicit Unix timestamp for certificate validity checks.
 **************************************************************************************************/
fn native_verify_and_parse_attestation_with_roots(
    context: &mut SafeNativeContext,
    _ty_args: &[Type],
    mut args: VecDeque<Value>,
) -> SafeNativeResult<SmallVec<[Value; 1]>> {
    safely_assert_eq!(_ty_args.len(), 0);
    safely_assert_eq!(args.len(), 3);

    let unix_time_secs = safely_pop_arg!(args, u64);
    let mut trusted_root_certs = vec![];
    for root in safely_pop_arg!(args, Vec<Value>) {
        trusted_root_certs.push(root.value_as::<Vec<u8>>()?);
    }
    let attestation_bytes = safely_pop_arg!(args, Vec<u8>);

    let cost = AWS_NITRO_VERIFY_AND_PARSE_ATTESTATION_BASE
        + AWS_NITRO_VERIFY_AND_PARSE_ATTESTATION_PER_BYTE
            * NumBytes::new(total_attestation_and_root_bytes(
                &attestation_bytes,
                &trusted_root_certs,
            ));
    context.charge(cost)?;

    let opt_doc = match validate_and_parse_attestation_doc_with_roots(
        &attestation_bytes,
        &trusted_root_certs,
        unix_time_secs,
    ) {
        Ok(doc) => option_some(context, attestation_doc_to_move_value(context, &doc)?)?,
        Err(_) => option_none(context)?,
    };

    Ok(smallvec![opt_doc])
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Tests the same root-aware validation path used by the native.
    #[test]
    fn test_empty_attestation_invalid() {
        assert!(validate_and_parse_attestation_doc_with_roots(&[], &[], 0).is_err());
    }

    #[test]
    fn test_garbage_attestation_invalid() {
        let garbage = [0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
        assert!(validate_and_parse_attestation_doc_with_roots(&garbage, &[], 0).is_err());
    }

    /// Happy path: real attestation doc from AWS Nitro Enclave.
    /// Run with: cargo test -p aptos-framework test_real_attestation -- --ignored
    /// (Requires /tmp/attestation-3586.bin and /tmp/aws-nitro-root.der to exist.)
    #[test]
    #[ignore = "requires real attestation doc at /tmp/attestation-3586.bin and Nitro root DER at /tmp/aws-nitro-root.der"]
    fn test_real_attestation_valid() {
        let path = std::path::Path::new("/tmp/attestation-3586.bin");
        let bytes = std::fs::read(path).expect("attestation doc at /tmp/attestation-3586.bin");
        let root = std::fs::read("/tmp/aws-nitro-root.der")
            .expect("Nitro root DER at /tmp/aws-nitro-root.der");
        let (_, decoded_doc) =
            attestation_doc_validation::attestation_doc::decode_attestation_document(&bytes)
                .expect("real attestation doc should decode");
        assert!(
            validate_and_parse_attestation_doc_with_roots(
                &bytes,
                &[root],
                decoded_doc.timestamp / 1000,
            )
            .is_ok(),
            "real attestation doc should validate"
        );
    }

    /// Happy path for verify_and_parse: real attestation doc parses and has expected fields.
    /// Run with: cargo test -p aptos-framework test_real_attestation_parse -- --ignored
    #[test]
    #[ignore = "requires real attestation doc at /tmp/attestation-3586.bin and Nitro root DER at /tmp/aws-nitro-root.der"]
    fn test_real_attestation_parse() {
        let path = std::path::Path::new("/tmp/attestation-3586.bin");
        let bytes = std::fs::read(path).expect("attestation doc at /tmp/attestation-3586.bin");
        let root = std::fs::read("/tmp/aws-nitro-root.der")
            .expect("Nitro root DER at /tmp/aws-nitro-root.der");
        let (_, decoded_doc) =
            attestation_doc_validation::attestation_doc::decode_attestation_document(&bytes)
                .expect("real attestation doc should decode");
        let doc = validate_and_parse_attestation_doc_with_roots(
            &bytes,
            &[root],
            decoded_doc.timestamp / 1000,
        )
        .expect("real attestation doc should parse");
        assert!(!doc.module_id.is_empty(), "module_id non-empty");
        assert!(!doc.certificate.is_empty(), "certificate non-empty");
        assert!(!doc.pcrs.is_empty(), "pcrs non-empty");
    }
}

/***************************************************************************************************
 * module
 *
 **************************************************************************************************/
pub fn make_all(
    builder: &SafeNativeBuilder,
) -> impl Iterator<Item = (String, NativeFunction)> + '_ {
    let natives = [
        (
            "verify_attestation_with_roots",
            native_verify_attestation_with_roots as RawSafeNative,
        ),
        (
            "verify_and_parse_attestation_with_roots",
            native_verify_and_parse_attestation_with_roots as RawSafeNative,
        ),
    ];

    builder.make_named_natives(natives)
}
