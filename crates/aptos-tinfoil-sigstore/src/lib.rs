//! Offline Sigstore bundle verification for Atlas's legacy FormatV2 CVM code
//! provenance.
//!
//! Much of this crate is vendored from `tinfoilsh/tinfoil-rs`, while [`policy`]
//! and [`provenance`] are new ports of `tinfoilsh/tinfoil-go` (`feat/v3`), the
//! reference implementation of the v3 verification model. Both are Apache-2.0;
//! see `NOTICE` for full provenance and the list of local modifications, which
//! includes removed and replaced upstream APIs.
//!
//! # Offline by construction
//!
//! Nothing here opens a socket. Production APIs require the caller to supply
//! Fulcio roots, Rekor log keys and CT log keys via a trusted-root JSON blob
//! (see [`trust`]). The embedded JSON is a test fixture only. The bundle
//! itself comes from the attestation document's `sigstore-code` collateral.
//!
//! Upstream's online entry point (`verify_repo`, which fetched releases and
//! bundles from GitHub) is deliberately **not** ported: on-chain verification
//! cannot fetch trust or evidence during execution.
//!
//! # What this crate is not, yet
//!
//! Verifying a bundle proves *who built what*. It does not by itself prove that
//! the code in question is what a particular CVM is running — that requires
//! binding the recovered measurement to a hardware quote. See
//! [`types::Measurement`] for which registers this covers (`RTMR1`/`RTMR2`) and
//! which it does not (`MRTD`, `RTMR0`).

// Style lints are allowed crate-wide because much of `src/` is vendored (see
// NOTICE). Keeping the delta from upstream as small as possible is what makes
// this code auditable against the source it came from, and re-vendoring a
// newer revision cheap. Rewriting validity-window checks or proof-verification
// signatures to satisfy a style lint would risk a semantic change in
// security-critical code for no benefit.
//
// These are deliberately narrow: correctness lints are NOT allowed.
#![allow(clippy::collapsible_if)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::result_unit_err)]

pub mod certificate;
pub mod checkpoint;
pub mod dsse;
pub mod error;
pub mod fulcio;
pub mod keyring;
pub mod merkle;
pub mod policy;
pub mod provenance;
pub mod rekor;
pub mod transparency;
pub mod trust;
pub mod types;
pub mod util;

use der::Decode;
use serde::Deserialize;
use x509_cert::Certificate;

pub use certificate::CertificateInfo;
pub use error::{Error, Result};
pub use types::{Measurement, PredicateType};

/// The OIDC issuer every GitHub Actions signing certificate must carry.
pub const GITHUB_OIDC_ISSUER: &str = "https://token.actions.githubusercontent.com";

/// The repository permitted to publish freshness witnesses.
pub const FRESHNESS_WITNESS_REPO: &str = "tinfoilsh/freshness-witness";

/// The DSSE payload type an in-toto statement travels as.
pub const IN_TOTO_PAYLOAD_TYPE: &str = "application/vnd.in-toto+json";

/// The in-toto Statement type this project's artifacts are produced with.
pub const IN_TOTO_STATEMENT_V1: &str = "https://in-toto.io/Statement/v1";
/// The legacy in-toto Statement type, accepted by [`parse_statement`].
pub const IN_TOTO_STATEMENT_V0_1: &str = "https://in-toto.io/Statement/v0.1";

/// In-toto statement from the decoded payload.
///
/// SPEC §5.4: the statement MUST contain only the recognized top-level fields;
/// `deny_unknown_fields` rejects any extra ones at parse time (matching
/// tinfoil-go's strict protojson parser). Tinfoil produces canonical
/// statements, so an unknown top-level field is non-canonical.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InTotoStatement {
    #[serde(rename = "_type")]
    pub type_: String,
    pub predicate_type: String,
    pub predicate: serde_json::Value,
    pub subject: Vec<Subject>,
}

/// One in-toto subject.
///
/// `deny_unknown_fields` for the same reason the statement has it: Tinfoil
/// produces canonical subjects, so an extra member means the producer is
/// asserting something this verifier does not read.
///
/// `digest` is a `BTreeMap` rather than a `HashMap` only for deterministic
/// iteration; note that duplicate *keys inside* the digest map are still
/// last-wins, because serde resolves map entries that way. That is why
/// [`enforce_subject0_digest`] looks up `sha256` explicitly instead of trusting
/// the map to hold exactly one entry.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Subject {
    pub name: String,
    pub digest: std::collections::BTreeMap<String, String>,
}

/// Extract the signing certificate's DER bytes and validity window.
///
/// The window is returned as Unix seconds and is used to pin the verification
/// clock: Rekor and Fulcio checks are evaluated at the time the certificate was
/// valid, not at "now".
pub fn extract_cert_with_validity(bundle: &serde_json::Value) -> Result<(Vec<u8>, u64, u64)> {
    let cert_der = certificate_der(bundle)?;
    let cert = Certificate::from_der(&cert_der)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to parse certificate: {e}")))?;

    let validity = cert.tbs_certificate().validity();
    Ok((
        cert_der.clone(),
        validity.not_before.to_unix_duration().as_secs(),
        validity.not_after.to_unix_duration().as_secs(),
    ))
}

/// The raw DER bytes of the bundle's signing certificate.
///
/// Only the Sigstore bundle v0.3 single-certificate form is accepted. A bundle
/// carrying `x509CertificateChain` is the legacy (pre-v0.3) shape, which
/// upstream rejects outright rather than trying to interpret.
pub fn certificate_der(bundle: &serde_json::Value) -> Result<Vec<u8>> {
    let material = bundle
        .get("verificationMaterial")
        .ok_or_else(|| Error::SigstoreVerification("No verificationMaterial in bundle".into()))?;

    if material.get("x509CertificateChain").is_some() {
        return Err(Error::SigstoreVerification(
            "Legacy bundle format (x509CertificateChain) is not accepted; \
             requires Sigstore bundle v0.3 single-certificate form"
                .into(),
        ));
    }

    let cert_b64 = material
        .get("certificate")
        .and_then(|c| c.get("rawBytes"))
        .and_then(|rb| rb.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No certificate in bundle".into()))?;

    util::decode_b64(cert_b64)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to decode certificate: {e}")))
}

/// Extract the Fulcio identity extensions from the bundle's certificate.
pub fn extract_certificate_info_from_bundle(bundle: &serde_json::Value) -> Result<CertificateInfo> {
    let cert_der = certificate_der(bundle)?;
    let cert = Certificate::from_der(&cert_der)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to parse certificate: {e}")))?;
    certificate::extract_certificate_info(&cert)
}

/// Decode and structurally validate the bundle's in-toto statement.
///
/// This does **not** authenticate anything — [`dsse::verify_dsse_signature`]
/// must have succeeded first. It only enforces the shape upstream requires:
/// the in-toto payload type, a recognized statement type, and strict parsing.
pub fn parse_statement(bundle: &serde_json::Value) -> Result<InTotoStatement> {
    let envelope = bundle
        .get("dsseEnvelope")
        .ok_or_else(|| Error::SigstoreVerification("No dsseEnvelope in bundle".into()))?;

    let payload_type = envelope
        .get("payloadType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No payloadType in DSSE envelope".into()))?;
    if payload_type != IN_TOTO_PAYLOAD_TYPE {
        return Err(Error::SigstoreVerification(format!(
            "Unsupported DSSE payload type: {payload_type:?}. Expected {IN_TOTO_PAYLOAD_TYPE:?}"
        )));
    }

    let payload_b64 = envelope
        .get("payload")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No payload in DSSE envelope".into()))?;
    let payload_bytes = util::decode_b64(payload_b64)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to decode payload: {e}")))?;

    let statement: InTotoStatement = serde_json::from_slice(&payload_bytes)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to parse statement: {e}")))?;

    if statement.type_ != IN_TOTO_STATEMENT_V0_1 && statement.type_ != IN_TOTO_STATEMENT_V1 {
        return Err(Error::SigstoreVerification(format!(
            "Unsupported in-toto statement type: {:?}",
            statement.type_
        )));
    }
    Ok(statement)
}

/// Require that **`subject[0]`** carries `expected_digest`.
///
/// SPEC §5.4. This is deliberately narrower than sigstore-go's
/// `WithArtifactDigest`, which matches *any* subject: a bundle whose first
/// subject is some other artifact must not be accepted because a later subject
/// happens to match.
pub fn enforce_subject0_digest(statement: &InTotoStatement, expected_digest: &str) -> Result<()> {
    let subject = statement
        .subject
        .first()
        .ok_or_else(|| Error::SigstoreVerification("No subject in statement".into()))?;
    if subject.name.is_empty() {
        return Err(Error::SigstoreVerification(
            "subject[0] has an empty name".into(),
        ));
    }

    let digest = subject
        .digest
        .get("sha256")
        .ok_or_else(|| Error::SigstoreVerification("No sha256 digest in subject[0]".into()))?;

    if !digest.eq_ignore_ascii_case(expected_digest) {
        return Err(Error::SigstoreVerification(format!(
            "subject[0] digest does not match. Expected: {expected_digest}, Got: {digest}"
        )));
    }
    Ok(())
}

/// Recover the code measurement from an authenticated statement.
///
/// Only [`PredicateType::SnpTdxMultiPlatformV1`] is supported, matching
/// upstream: anything else is refused rather than interpreted.
pub fn measurement_from_statement(statement: &InTotoStatement) -> Result<Measurement> {
    if statement.predicate_type != types::SNP_TDX_MULTIPLATFORM_V1 {
        return Err(Error::SigstoreVerification(format!(
            "Unsupported predicate type: {}",
            statement.predicate_type
        )));
    }

    let field = |value: Option<&serde_json::Value>, what: &str| -> Result<String> {
        value
            .and_then(|v| v.as_str())
            .filter(|v| !v.is_empty())
            .map(str::to_string)
            .ok_or_else(|| Error::SigstoreVerification(format!("Missing {what}")))
    };

    let snp = field(
        statement.predicate.get("snp_measurement"),
        "snp_measurement",
    )?;
    let tdx = statement
        .predicate
        .get("tdx_measurement")
        .ok_or_else(|| Error::SigstoreVerification("Missing tdx_measurement".into()))?;
    let rtmr1 = field(tdx.get("rtmr1"), "tdx_measurement.rtmr1")?;
    let rtmr2 = field(tdx.get("rtmr2"), "tdx_measurement.rtmr2")?;

    Ok(Measurement {
        type_: PredicateType::SnpTdxMultiPlatformV1,
        registers: vec![snp, rtmr1, rtmr2],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_legacy_bundle_format_is_rejected() {
        // Pre-v0.3 bundles carry a certificate *chain*. Upstream refuses these
        // outright rather than picking a leaf out of them.
        let bundle = serde_json::json!({
            "verificationMaterial": {
                "x509CertificateChain": { "certificates": [] }
            }
        });
        let err = certificate_der(&bundle).unwrap_err();
        assert!(
            err.to_string().contains("Legacy bundle format"),
            "got {err}"
        );
    }

    fn statement_with(subjects: serde_json::Value) -> InTotoStatement {
        serde_json::from_value(serde_json::json!({
            "_type": "https://in-toto.io/Statement/v1",
            "predicateType": types::SNP_TDX_MULTIPLATFORM_V1,
            "predicate": {},
            "subject": subjects,
        }))
        .expect("statement")
    }

    /// Only subject[0] may satisfy the digest. A bundle whose *second* subject
    /// matches must be refused, or an attacker could append their artifact to
    /// a legitimately signed statement.
    #[test]
    fn only_subject_zero_may_match_the_digest() {
        let statement = statement_with(serde_json::json!([
            { "name": "other", "digest": { "sha256": "aa" } },
            { "name": "wanted", "digest": { "sha256": "bb" } },
        ]));
        assert!(enforce_subject0_digest(&statement, "aa").is_ok());
        assert!(
            enforce_subject0_digest(&statement, "bb").is_err(),
            "a later subject must not satisfy the digest"
        );
    }

    #[test]
    fn subject_zero_digest_is_case_insensitive() {
        let statement = statement_with(serde_json::json!([
            { "name": "artifact", "digest": { "sha256": "AbCd" } },
        ]));
        assert!(enforce_subject0_digest(&statement, "abcd").is_ok());
    }

    #[test]
    fn an_empty_subject_name_is_rejected() {
        let statement = statement_with(serde_json::json!([
            { "name": "", "digest": { "sha256": "aa" } },
        ]));
        assert!(enforce_subject0_digest(&statement, "aa").is_err());
    }

    /// A subject carrying a member this verifier does not read means the
    /// producer is asserting something unrecognized, which for a canonical
    /// producer is a signal rather than a nicety.
    #[test]
    fn an_unknown_subject_member_is_refused() {
        let parsed: Result<InTotoStatement> = serde_json::from_value(serde_json::json!({
            "_type": "https://in-toto.io/Statement/v1",
            "predicateType": types::SNP_TDX_MULTIPLATFORM_V1,
            "predicate": {},
            "subject": [
                { "name": "a", "digest": { "sha256": "aa" }, "annotations": { "x": 1 } },
            ],
        }))
        .map_err(|e| Error::SigstoreVerification(e.to_string()));
        assert!(parsed.is_err(), "an unknown subject member must be refused");
    }

    /// Duplicate *declared* members are rejected by serde itself, so a subject
    /// cannot present one name to one reader and another to the next.
    #[test]
    fn a_duplicate_subject_member_is_refused() {
        let raw = r#"{
            "_type": "https://in-toto.io/Statement/v1",
            "predicateType": "https://tinfoil.sh/predicate/snp-tdx-multiplatform/v1",
            "predicate": {},
            "subject": [{ "name": "a", "digest": { "sha256": "aa" }, "name": "b" }]
        }"#;
        let parsed: std::result::Result<InTotoStatement, _> = serde_json::from_str(raw);
        assert!(
            parsed.is_err(),
            "a duplicated subject member must be refused, not last-wins"
        );
    }

    #[test]
    fn an_unknown_predicate_type_is_refused() {
        let statement: InTotoStatement = serde_json::from_value(serde_json::json!({
            "_type": "https://in-toto.io/Statement/v1",
            "predicateType": "https://example.invalid/predicate/v1",
            "predicate": { "snp_measurement": "aa", "tdx_measurement": {} },
            "subject": [],
        }))
        .expect("statement");
        let err = measurement_from_statement(&statement).unwrap_err();
        assert!(err.to_string().contains("Unsupported predicate type"));
    }

    #[test]
    fn a_measurement_requires_every_register() {
        for predicate in [
            serde_json::json!({ "tdx_measurement": { "rtmr1": "a", "rtmr2": "b" } }),
            serde_json::json!({ "snp_measurement": "s" }),
            serde_json::json!({ "snp_measurement": "s", "tdx_measurement": { "rtmr1": "a" } }),
            // Present but empty must count as missing.
            serde_json::json!({
                "snp_measurement": "",
                "tdx_measurement": { "rtmr1": "a", "rtmr2": "b" }
            }),
        ] {
            let statement: InTotoStatement = serde_json::from_value(serde_json::json!({
                "_type": "https://in-toto.io/Statement/v1",
                "predicateType": types::SNP_TDX_MULTIPLATFORM_V1,
                "predicate": predicate,
                "subject": [],
            }))
            .expect("statement");
            assert!(
                measurement_from_statement(&statement).is_err(),
                "incomplete predicate must be refused: {:?}",
                statement.predicate
            );
        }
    }

    #[test]
    fn a_complete_measurement_orders_registers_as_upstream_does() {
        let statement: InTotoStatement = serde_json::from_value(serde_json::json!({
            "_type": "https://in-toto.io/Statement/v1",
            "predicateType": types::SNP_TDX_MULTIPLATFORM_V1,
            "predicate": {
                "snp_measurement": "snp",
                "tdx_measurement": { "rtmr1": "k", "rtmr2": "i" }
            },
            "subject": [],
        }))
        .expect("statement");

        let measurement = measurement_from_statement(&statement).expect("measurement");
        assert_eq!(measurement.registers, vec!["snp", "k", "i"]);
        assert_eq!(measurement.snp_measurement(), Some("snp"));
        assert_eq!(measurement.rtmr1(), Some("k"));
        assert_eq!(measurement.rtmr2(), Some("i"));
    }

    /// An unknown top-level member must be refused: Tinfoil emits canonical
    /// statements, so anything extra is non-canonical.
    #[test]
    fn statement_parsing_is_strict() {
        let parsed: std::result::Result<InTotoStatement, _> =
            serde_json::from_value(serde_json::json!({
                "_type": "https://in-toto.io/Statement/v1",
                "predicateType": types::SNP_TDX_MULTIPLATFORM_V1,
                "predicate": {},
                "subject": [],
                "extra": 1,
            }));
        assert!(parsed.is_err(), "unknown members must be rejected");
    }
}
