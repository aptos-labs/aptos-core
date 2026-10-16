// Adapted from sigstore-rs (Apache 2.0 License)
// https://github.com/sigstore/sigstore-rs/blob/main/src/crypto/certificate.rs
//
// Copyright 2021 The Sigstore Authors.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Certificate validation for Sigstore/Fulcio certificates.
//!
//! Adapted from sigstore-rs to use typed extension parsing via x509-cert
//! instead of manual byte manipulation.

use const_oid::db::rfc5912::ID_KP_CODE_SIGNING;
use x509_cert::{
    Certificate,
    ext::pkix::{ExtendedKeyUsage, KeyUsage},
};

use crate::error::{Error, Result};

/// Verification certificate info extracted from bundle
///
/// # Which fields are load-bearing
///
/// [`Self::issuer`], [`Self::repository`] and [`Self::subject_workflow`] are
/// the identity a caller matches against a *trusted* repository name. The
/// remaining fields come from Fulcio's provider-generic extensions and carry
/// the constraints upstream Tinfoil additionally enforces:
///
/// * [`Self::runner_environment`] must be `github-hosted`, which is asserted by
///   the OIDC token rather than by the workflow, so a self-hosted runner cannot
///   claim it.
/// * [`Self::source_repository_ref`] must be a `refs/tags/...` ref, which is
///   what makes a release tag (rather than a mutable branch) the thing being
///   attested.
/// * [`Self::source_repository_digest`] is the commit the build ran against.
#[derive(Debug, Clone)]
pub struct CertificateInfo {
    pub issuer: String,
    pub subject_workflow: String,
    pub repository: String,
    /// Runner environment (`github-hosted` or `self-hosted`), OID .1.11.
    pub runner_environment: String,
    /// Git ref the build ran against, e.g. `refs/tags/v1.2.3`, OID .1.14.
    pub source_repository_ref: String,
    /// Commit SHA the build ran against, OID .1.13.
    pub source_repository_digest: String,
    /// Digest of the signer workflow file, OID .1.10.
    pub build_signer_digest: String,
}

/// Fulcio OIDC extension OIDs
///
/// See: https://github.com/sigstore/fulcio/blob/main/docs/oid-info.md
///
/// # Encoding note
///
/// `.1.1` through `.1.6` are the deprecated GitHub-specific extensions and
/// store their values as raw strings. `.1.8` through `.1.24` are the
/// provider-generic replacements and are **DER-encoded** (ASN.1 `UTF8String`,
/// tag `0x0C`). [`decode_asn1_string`] handles both, which is why it tries the
/// DER decodings before falling back to raw UTF-8.
mod fulcio_oids {
    /// OIDC Issuer V1 (1.3.6.1.4.1.57264.1.1)
    pub const OIDC_ISSUER_V1: &str = "1.3.6.1.4.1.57264.1.1";
    /// OIDC Issuer V2 (1.3.6.1.4.1.57264.1.8)
    pub const OIDC_ISSUER_V2: &str = "1.3.6.1.4.1.57264.1.8";
    /// GitHub Workflow Repository V1 (1.3.6.1.4.1.57264.1.5)
    pub const GITHUB_WORKFLOW_REPOSITORY: &str = "1.3.6.1.4.1.57264.1.5";
    /// Build Signer URI (1.3.6.1.4.1.57264.1.9)
    pub const BUILD_SIGNER_URI: &str = "1.3.6.1.4.1.57264.1.9";
    /// Build Signer Digest (1.3.6.1.4.1.57264.1.10)
    pub const BUILD_SIGNER_DIGEST: &str = "1.3.6.1.4.1.57264.1.10";
    /// Runner Environment (1.3.6.1.4.1.57264.1.11)
    pub const RUNNER_ENVIRONMENT: &str = "1.3.6.1.4.1.57264.1.11";
    /// Source Repository URI (1.3.6.1.4.1.57264.1.12)
    pub const SOURCE_REPOSITORY_URI: &str = "1.3.6.1.4.1.57264.1.12";
    /// Source Repository Digest (1.3.6.1.4.1.57264.1.13)
    pub const SOURCE_REPOSITORY_DIGEST: &str = "1.3.6.1.4.1.57264.1.13";
    /// Source Repository Ref (1.3.6.1.4.1.57264.1.14)
    pub const SOURCE_REPOSITORY_REF: &str = "1.3.6.1.4.1.57264.1.14";
}

/// The runner environment a GitHub-hosted build reports.
pub const RUNNER_ENVIRONMENT_GITHUB_HOSTED: &str = "github-hosted";

/// Prefix every attested build's ref must carry.
pub const TAG_REF_PREFIX: &str = "refs/tags/";

/// Validate certificate extensions for Fulcio code signing requirements.
///
/// Per Sigstore specification, a valid Fulcio certificate must have:
/// 1. KeyUsage extension with digitalSignature bit set
/// 2. ExtendedKeyUsage extension containing codeSigning OID (1.3.6.1.5.5.7.3.3)
///
/// This implementation uses typed extension parsing from x509-cert,
/// following sigstore-rs's approach.
pub fn validate_certificate_extensions(cert: &Certificate) -> Result<()> {
    let tbs = cert.tbs_certificate();

    // Check KeyUsage extension for digitalSignature
    let key_usage = tbs
        .get_extension::<KeyUsage>()
        .map_err(|e| {
            Error::SigstoreVerification(format!("Failed to parse KeyUsage extension: {}", e))
        })?
        .ok_or_else(|| {
            Error::SigstoreVerification("Certificate missing KeyUsage extension".into())
        })?;

    if !key_usage.1.digital_signature() {
        return Err(Error::SigstoreVerification(
            "Certificate KeyUsage does not include digitalSignature".into(),
        ));
    }

    // Check ExtendedKeyUsage extension for codeSigning
    let ext_key_usage = tbs
        .get_extension::<ExtendedKeyUsage>()
        .map_err(|e| {
            Error::SigstoreVerification(format!(
                "Failed to parse ExtendedKeyUsage extension: {}",
                e
            ))
        })?
        .ok_or_else(|| {
            Error::SigstoreVerification("Certificate missing ExtendedKeyUsage extension".into())
        })?;

    if !ext_key_usage.1.0.contains(&ID_KP_CODE_SIGNING) {
        return Err(Error::SigstoreVerification(
            "Certificate ExtendedKeyUsage does not include codeSigning".into(),
        ));
    }

    Ok(())
}

/// Decode an ASN.1 string from extension value bytes.
/// Fulcio uses UTF8String (tag 0x0C) for these extensions, but IA5String
/// and PrintableString are also accepted.
fn decode_asn1_string(bytes: &[u8]) -> Option<String> {
    use der::Decode;
    // Try UTF8String first (most common for Fulcio V2 extensions)
    if let Ok(s) = der::asn1::Utf8StringRef::from_der(bytes) {
        return Some(s.to_string());
    }
    // IA5String (used by some V1 extensions)
    if let Ok(s) = der::asn1::Ia5StringRef::from_der(bytes) {
        return Some(s.to_string());
    }
    // PrintableString (fallback)
    if let Ok(s) = der::asn1::PrintableStringRef::from_der(bytes) {
        return Some(s.to_string());
    }
    // Fallback: older Fulcio V1 certificates stored the OIDC issuer as raw
    // UTF-8 bytes without an ASN.1 tag wrapper.
    std::str::from_utf8(bytes).ok().map(|s| s.to_string())
}

/// Extract certificate info from a Fulcio certificate.
///
/// Pulls the OIDC issuer, workflow and repository identity, plus the
/// provider-generic extensions upstream Tinfoil constrains: runner
/// environment, source repository ref and digest, and the build signer digest.
///
/// Only the first three are *required*: they are what every Fulcio GitHub
/// Actions certificate carries, and a certificate without them cannot be
/// matched to a repository at all. The provider-generic extensions are
/// returned empty when absent, and the caller decides whether their absence is
/// disqualifying — [`check_identity`] refuses a certificate that lacks the
/// runner environment or a tag ref, but a caller verifying a non-GitHub
/// identity would not.
pub fn extract_certificate_info(cert: &Certificate) -> Result<CertificateInfo> {
    let mut issuer = String::new();
    let mut repository = String::new();
    let mut subject_workflow = String::new();
    let mut runner_environment = String::new();
    let mut source_repository_ref = String::new();
    let mut source_repository_digest = String::new();
    let mut build_signer_digest = String::new();

    if let Some(extensions) = cert.tbs_certificate().extensions() {
        for ext in extensions.iter() {
            let oid_str = ext.extn_id.to_string();
            let raw_bytes = ext.extn_value.as_bytes();

            let Some(value) = decode_asn1_string(raw_bytes) else {
                continue;
            };

            match oid_str.as_str() {
                fulcio_oids::OIDC_ISSUER_V2 => {
                    // V2 takes priority over V1
                    issuer = value;
                }
                fulcio_oids::OIDC_ISSUER_V1 if issuer.is_empty() => {
                    // V1 is used only if V2 was not found
                    issuer = value;
                }
                fulcio_oids::BUILD_SIGNER_URI => {
                    subject_workflow = value;
                }
                fulcio_oids::BUILD_SIGNER_DIGEST => {
                    build_signer_digest = value;
                }
                fulcio_oids::RUNNER_ENVIRONMENT => {
                    runner_environment = value;
                }
                fulcio_oids::SOURCE_REPOSITORY_DIGEST => {
                    source_repository_digest = value;
                }
                fulcio_oids::SOURCE_REPOSITORY_REF => {
                    source_repository_ref = value;
                }
                fulcio_oids::GITHUB_WORKFLOW_REPOSITORY => {
                    repository = value;
                }
                fulcio_oids::SOURCE_REPOSITORY_URI if repository.is_empty() => {
                    // V2 fallback: extract repo name from URI
                    // SOURCE_REPOSITORY_URI is "https://github.com/owner/repo"
                    // GITHUB_WORKFLOW_REPOSITORY (V1) is "owner/repo"
                    repository = value
                        .strip_prefix("https://github.com/")
                        .unwrap_or(&value)
                        .to_string();
                }
                _ => {}
            }
        }
    }

    if issuer.is_empty() {
        return Err(Error::SigstoreVerification(
            "Certificate missing required OIDC issuer extension".into(),
        ));
    }
    if repository.is_empty() {
        return Err(Error::SigstoreVerification(
            "Certificate missing required repository extension".into(),
        ));
    }
    if subject_workflow.is_empty() {
        return Err(Error::SigstoreVerification(
            "Certificate missing required workflow extension".into(),
        ));
    }

    Ok(CertificateInfo {
        issuer,
        subject_workflow,
        repository,
        runner_environment,
        source_repository_ref,
        source_repository_digest,
        build_signer_digest,
    })
}

/// What a caller requires of a signing certificate's identity.
///
/// Built by the caller from a **trusted** repository name — never from the
/// bundle. That is the whole point: a repository name read out of the
/// attestation would let an attacker nominate the repository whose signature
/// they can produce.
#[derive(Debug, Clone)]
pub struct ExpectedIdentity {
    /// The trusted repository, `owner/name`.
    pub repository: String,
    /// Regex the workflow file name must match, e.g. `[^/@]+` or `build\.yml`.
    pub workflow_pattern: String,
    /// Regex the git ref must match, e.g. `refs/tags/[^@]+`.
    pub ref_pattern: String,
    /// Exact release tag the ref must name, when the caller knows it.
    pub tag: Option<String>,
}

impl ExpectedIdentity {
    /// The identity Tinfoil requires of a *code* build: any workflow file
    /// directly under `.github/workflows`, built from a tag.
    pub fn code(repository: &str, tag: Option<&str>) -> Self {
        Self {
            repository: repository.to_string(),
            workflow_pattern: r"[^/@]+".to_string(),
            ref_pattern: r"refs/tags/[^@]+".to_string(),
            tag: tag.map(str::to_string),
        }
    }

    /// The identity Tinfoil requires of the *platform endorsements* artifact.
    ///
    /// Far narrower than [`Self::code`]: one specific publisher repository, one
    /// specific workflow file, and a `v`-prefixed release tag. The endorsements
    /// artifact decides which machines may host workloads at all, so the set of
    /// things allowed to sign it is deliberately minimal.
    pub fn platform_endorsements(tag: Option<&str>) -> Self {
        Self {
            repository: crate::policy::PLATFORM_ENDORSEMENTS_REPO.to_string(),
            workflow_pattern: r"build\.yml".to_string(),
            ref_pattern: r"refs/tags/v[0-9][^@]*".to_string(),
            tag: tag.map(str::to_string),
        }
    }

    /// The identity Tinfoil requires of a *freshness witness*.
    ///
    /// Unlike the other two, this is signed from `refs/heads/main` rather than
    /// a tag: a witness attests that an artifact was still current at a point
    /// in time, so it is published continuously rather than per release.
    pub fn freshness_witness() -> Self {
        Self {
            repository: crate::FRESHNESS_WITNESS_REPO.to_string(),
            workflow_pattern: r"freshness\.yml".to_string(),
            ref_pattern: r"refs/heads/main".to_string(),
            tag: None,
        }
    }

    /// Whether this identity expects a tag ref at all.
    ///
    /// The freshness witness is signed from a branch, so the `refs/tags/`
    /// requirement in [`check_identity`] must not apply to it.
    pub fn expects_tag_ref(&self) -> bool {
        self.ref_pattern.starts_with(TAG_REF_PREFIX)
    }

    /// The anchored SAN pattern a conforming certificate's workflow URI must
    /// match, mirroring upstream's `signingIdentity`.
    ///
    /// The repository is `regex`-escaped: it is caller-supplied configuration,
    /// and an unescaped `.` in `owner/repo.io` would otherwise match any
    /// character.
    pub fn workflow_regex(&self) -> String {
        format!(
            r"^https://github\.com/{}/\.github/workflows/{}@{}$",
            regex::escape(&self.repository),
            self.workflow_pattern,
            self.ref_pattern,
        )
    }
}

/// Check a certificate's identity against what the caller requires.
///
/// This is the port of the identity half of upstream's
/// `provenance.AuthenticateCode`. It enforces, in order:
///
/// 1. the OIDC issuer is exactly GitHub Actions;
/// 2. the repository extension equals the **trusted** repository;
/// 3. the workflow URI matches the anchored SAN pattern;
/// 4. the runner was GitHub-hosted;
/// 5. the build ran from a `refs/tags/` ref, and equals `tag` when given;
/// 6. the source repository digest looks like a commit SHA.
///
/// It does **not** verify any signature — [`crate::dsse`], [`crate::fulcio`]
/// and [`crate::rekor`] do that. Identity means nothing until they have run.
pub fn check_identity(info: &CertificateInfo, expected: &ExpectedIdentity) -> Result<()> {
    if info.issuer != crate::GITHUB_OIDC_ISSUER {
        return Err(Error::SigstoreVerification(format!(
            "Certificate not from GitHub Actions. Issuer: {}",
            info.issuer
        )));
    }

    if info.repository != expected.repository {
        return Err(Error::SigstoreVerification(format!(
            "Certificate repository does not match. Expected: {}, Got: {}",
            expected.repository, info.repository
        )));
    }

    let pattern = expected.workflow_regex();
    let re = regex::Regex::new(&pattern)
        .map_err(|e| Error::SigstoreVerification(format!("Invalid identity regex: {e}")))?;
    if !re.is_match(&info.subject_workflow) {
        return Err(Error::SigstoreVerification(format!(
            "Certificate workflow does not match {pattern:?}. Got: {}",
            info.subject_workflow
        )));
    }

    // GitHub asserts this from the OIDC token, not from workflow-controlled
    // input, so a self-hosted runner cannot claim to be GitHub-hosted. An
    // absent extension is refused rather than assumed.
    if info.runner_environment != RUNNER_ENVIRONMENT_GITHUB_HOSTED {
        return Err(Error::SigstoreVerification(format!(
            "Build did not run on a GitHub-hosted runner. Runner environment: {:?}",
            info.runner_environment
        )));
    }

    // The ref is already constrained through the anchored SAN pattern above,
    // but that only proves the *workflow URI* names this ref. Check the
    // dedicated extension (OID .1.14) independently: the two are separate
    // assertions, and requiring both means a certificate cannot satisfy one
    // while carrying something else in the other.
    let ref_re = regex::Regex::new(&format!("^{}$", expected.ref_pattern))
        .map_err(|e| Error::SigstoreVerification(format!("Invalid ref regex: {e}")))?;
    if !ref_re.is_match(&info.source_repository_ref) {
        return Err(Error::SigstoreVerification(format!(
            "Build ref does not match {:?}. Got: {:?}",
            expected.ref_pattern, info.source_repository_ref
        )));
    }

    // A tag is (by convention) immutable; a branch is not. Requiring a tag ref
    // is what stops a build from a moving branch being presented as a release.
    //
    // Freshness witnesses are the deliberate exception: they are published
    // continuously from `main`, so they declare a non-tag ref pattern and are
    // exempted here rather than by weakening the check for everyone.
    if expected.expects_tag_ref() {
        if !info.source_repository_ref.starts_with(TAG_REF_PREFIX) {
            return Err(Error::SigstoreVerification(format!(
                "Build ref is not a tag. Expected a {TAG_REF_PREFIX}... ref, got: {:?}",
                info.source_repository_ref
            )));
        }
        if let Some(tag) = &expected.tag {
            let want = format!("{TAG_REF_PREFIX}{tag}");
            if info.source_repository_ref != want {
                return Err(Error::SigstoreVerification(format!(
                    "Build ref does not match the expected tag. Expected: {want}, Got: {}",
                    info.source_repository_ref
                )));
            }
        }
    }

    if !is_commit_sha(&info.source_repository_digest) {
        return Err(Error::SigstoreVerification(format!(
            "Source repository digest is not a commit SHA: {:?}",
            info.source_repository_digest
        )));
    }

    Ok(())
}

/// Whether `value` is a 40-character lowercase-hex git commit SHA.
fn is_commit_sha(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

#[cfg(test)]
mod tests {
    use super::*;
    use der::Decode;

    #[test]
    fn test_decode_asn1_string_utf8() {
        // UTF8String with "test"
        let bytes = [0x0C, 0x04, b't', b'e', b's', b't'];
        assert_eq!(decode_asn1_string(&bytes), Some("test".to_string()));
    }

    #[test]
    fn test_decode_asn1_string_ia5() {
        // IA5String with "test"
        let bytes = [0x16, 0x04, b't', b'e', b's', b't'];
        assert_eq!(decode_asn1_string(&bytes), Some("test".to_string()));
    }

    #[test]
    fn test_decode_asn1_string_raw_utf8_fallback() {
        // Raw UTF-8 bytes without ASN.1 tag (as used by older Fulcio V1 certs)
        let url = b"https://github.com/login/oauth";
        assert_eq!(
            decode_asn1_string(url),
            Some("https://github.com/login/oauth".to_string())
        );
    }

    #[test]
    fn test_decode_asn1_string_invalid_utf8() {
        // Invalid UTF-8 bytes
        let bytes = [0xFF, 0xFE, 0x00, 0x01];
        assert_eq!(decode_asn1_string(&bytes), None);
    }

    #[test]
    fn test_validate_fulcio_certificate() {
        // Real Fulcio certificate from sigstore test data
        let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIICzDCCAlGgAwIBAgIUF96OLbM9/tDVHKCJliXLTFvnfjAwCgYIKoZIzj0EAwMw
NzEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MR4wHAYDVQQDExVzaWdzdG9yZS1pbnRl
cm1lZGlhdGUwHhcNMjMxMjEzMDU1MDU1WhcNMjMxMjEzMDYwMDU1WjAAMFkwEwYH
KoZIzj0CAQYIKoZIzj0DAQcDQgAEmir+Lah2291zCsLkmREQNLzf99z571BNB+fa
rerSLGzcwLFK7GRLTGYcO0oStxCYavxRQPMo3JvB8vGtZbn/76OCAXAwggFsMA4G
A1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDAzAdBgNVHQ4EFgQU8U9M
t9GMrRm8+gifPtc63nlP3OIwHwYDVR0jBBgwFoAU39Ppz1YkEZb5qNjpKFWixi4Y
ZD8wGwYDVR0RAQH/BBEwD4ENYXNjQHRldHN1by5zaDAsBgorBgEEAYO/MAEBBB5o
dHRwczovL2dpdGh1Yi5jb20vbG9naW4vb2F1dGgwLgYKKwYBBAGDvzABCAQgDB5o
dHRwczovL2dpdGh1Yi5jb20vbG9naW4vb2F1dGgwgYkGCisGAQQB1nkCBAIEewR5
AHcAdQDdPTBqxscRMmMZHhyZZzcCokpeuN48rf+HinKALynujgAAAYxhumYsAAAE
AwBGMEQCIHRRe20lRrNM4xd07mpjTtgaE6FGS3jjF++zW8ZMnth3AiAd6LVAAeVW
hSW4T0XJRw9lGU6/EK9+ELZpEjrY03dJ1zAKBggqhkjOPQQDAwNpADBmAjEAiHqK
W9PQ/5h7VROVIWPaxUo3LhrL2sZanw4bzTDBDY0dRR19ZFzjtAph1RzpQqppAjEA
plAvxwkAIR2jurboJZ4Zm9rNAx8KvA+A5yQFzNkGgKDLjTJrKmSKoIcWV3j7WfdL
-----END CERTIFICATE-----"#;

        // Parse PEM to DER
        let pem = pem::parse(cert_pem).unwrap();
        let cert = Certificate::from_der(pem.contents()).unwrap();

        // Should pass validation
        assert!(validate_certificate_extensions(&cert).is_ok());

        // This test cert has an OIDC issuer but lacks the GitHub-specific
        // repository and workflow extensions, so extraction should fail.
        let err = extract_certificate_info(&cert).unwrap_err();
        assert!(
            err.to_string().contains("missing required"),
            "Expected missing-field error, got: {}",
            err
        );
    }

    fn parse_test_cert() -> Certificate {
        let cert_pem = r#"-----BEGIN CERTIFICATE-----
MIICzDCCAlGgAwIBAgIUF96OLbM9/tDVHKCJliXLTFvnfjAwCgYIKoZIzj0EAwMw
NzEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MR4wHAYDVQQDExVzaWdzdG9yZS1pbnRl
cm1lZGlhdGUwHhcNMjMxMjEzMDU1MDU1WhcNMjMxMjEzMDYwMDU1WjAAMFkwEwYH
KoZIzj0CAQYIKoZIzj0DAQcDQgAEmir+Lah2291zCsLkmREQNLzf99z571BNB+fa
rerSLGzcwLFK7GRLTGYcO0oStxCYavxRQPMo3JvB8vGtZbn/76OCAXAwggFsMA4G
A1UdDwEB/wQEAwIHgDATBgNVHSUEDDAKBggrBgEFBQcDAzAdBgNVHQ4EFgQU8U9M
t9GMrRm8+gifPtc63nlP3OIwHwYDVR0jBBgwFoAU39Ppz1YkEZb5qNjpKFWixi4Y
ZD8wGwYDVR0RAQH/BBEwD4ENYXNjQHRldHN1by5zaDAsBgorBgEEAYO/MAEBBB5o
dHRwczovL2dpdGh1Yi5jb20vbG9naW4vb2F1dGgwLgYKKwYBBAGDvzABCAQgDB5o
dHRwczovL2dpdGh1Yi5jb20vbG9naW4vb2F1dGgwgYkGCisGAQQB1nkCBAIEewR5
AHcAdQDdPTBqxscRMmMZHhyZZzcCokpeuN48rf+HinKALynujgAAAYxhumYsAAAE
AwBGMEQCIHRRe20lRrNM4xd07mpjTtgaE6FGS3jjF++zW8ZMnth3AiAd6LVAAeVW
hSW4T0XJRw9lGU6/EK9+ELZpEjrY03dJ1zAKBggqhkjOPQQDAwNpADBmAjEAiHqK
W9PQ/5h7VROVIWPaxUo3LhrL2sZanw4bzTDBDY0dRR19ZFzjtAph1RzpQqppAjEA
plAvxwkAIR2jurboJZ4Zm9rNAx8KvA+A5yQFzNkGgKDLjTJrKmSKoIcWV3j7WfdL
-----END CERTIFICATE-----"#;
        let pem = pem::parse(cert_pem).unwrap();
        Certificate::from_der(pem.contents()).unwrap()
    }

    /// OID for KeyUsage: 2.5.29.15
    const KEY_USAGE_OID: &str = "2.5.29.15";
    /// OID for ExtendedKeyUsage: 2.5.29.37
    const EXT_KEY_USAGE_OID: &str = "2.5.29.37";

    /// DER mirrors of x509-cert's certificate types. x509-cert 0.3 keeps the
    /// struct fields private, so rebuilding a certificate with edited
    /// extensions round-trips through these copies. They replicate the RFC 5280
    /// schema exactly, so the DER round-trips byte-for-byte apart from the
    /// edited extension list.
    #[derive(Debug, der::Sequence)]
    struct TbsMirror {
        #[asn1(context_specific = "0", default = "Default::default")]
        version: x509_cert::Version,
        serial_number: x509_cert::serial_number::SerialNumber,
        signature: x509_cert::AlgorithmIdentifier,
        issuer: x509_cert::name::Name,
        validity: x509_cert::time::Validity,
        subject: x509_cert::name::Name,
        subject_public_key_info: x509_cert::SubjectPublicKeyInfo,
        #[asn1(context_specific = "1", tag_mode = "IMPLICIT", optional = "true")]
        issuer_unique_id: Option<der::asn1::BitString>,
        #[asn1(context_specific = "2", tag_mode = "IMPLICIT", optional = "true")]
        subject_unique_id: Option<der::asn1::BitString>,
        #[asn1(context_specific = "3", tag_mode = "EXPLICIT", optional = "true")]
        extensions: Option<x509_cert::ext::Extensions>,
    }

    #[derive(Debug, der::Sequence)]
    struct CertificateMirror {
        tbs_certificate: TbsMirror,
        signature_algorithm: x509_cert::AlgorithmIdentifier,
        signature: der::asn1::BitString,
    }

    /// Returns a copy of `cert` with its extension list mapped through `f`,
    /// keeping the remaining TBS fields and signature parts untouched.
    fn map_cert_extensions(
        cert: &Certificate,
        f: impl FnOnce(&x509_cert::ext::Extensions) -> x509_cert::ext::Extensions,
    ) -> Certificate {
        use der::Encode;

        let tbs_der = cert.tbs_certificate().to_der().unwrap();
        let mut tbs = TbsMirror::from_der(&tbs_der).unwrap();
        tbs.extensions = tbs.extensions.as_ref().map(f);
        let cert_der = CertificateMirror {
            tbs_certificate: tbs,
            signature_algorithm: cert.signature_algorithm().clone(),
            signature: cert.signature().clone(),
        }
        .to_der()
        .unwrap();
        Certificate::from_der(&cert_der).unwrap()
    }

    fn cert_without_extension(oid_to_remove: &str) -> Certificate {
        let cert = parse_test_cert();
        map_cert_extensions(&cert, |exts| {
            exts.iter()
                .filter(|ext| ext.extn_id.to_string() != oid_to_remove)
                .cloned()
                .collect()
        })
    }

    #[test]
    fn test_validate_missing_key_usage() {
        let cert = cert_without_extension(KEY_USAGE_OID);
        let err = validate_certificate_extensions(&cert).unwrap_err();
        assert!(
            err.to_string().contains("KeyUsage"),
            "Expected KeyUsage error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_missing_ext_key_usage() {
        let cert = cert_without_extension(EXT_KEY_USAGE_OID);
        let err = validate_certificate_extensions(&cert).unwrap_err();
        assert!(
            err.to_string().contains("ExtendedKeyUsage"),
            "Expected ExtendedKeyUsage error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_no_digital_signature() {
        use der::Encode;

        let cert = parse_test_cert();

        // Replace KeyUsage extension with one that has no digitalSignature bit
        // KeyUsage is a BIT STRING; 0x00 = no bits set
        let ku_oid: der::asn1::ObjectIdentifier = KEY_USAGE_OID.parse().unwrap();
        let empty_ku_value = der::asn1::BitString::from_bytes(&[0x00]).unwrap();
        let empty_ku_der = empty_ku_value.to_der().unwrap();

        let modified = map_cert_extensions(&cert, |exts| {
            exts.iter()
                .map(|ext| {
                    if ext.extn_id == ku_oid {
                        x509_cert::ext::Extension {
                            extn_id: ext.extn_id.clone(),
                            critical: ext.critical,
                            extn_value: der::asn1::OctetString::new(empty_ku_der.clone()).unwrap(),
                        }
                    } else {
                        ext.clone()
                    }
                })
                .collect()
        });

        let err = validate_certificate_extensions(&modified).unwrap_err();
        assert!(
            err.to_string().contains("digitalSignature"),
            "Expected digitalSignature error, got: {}",
            err
        );
    }

    #[test]
    fn test_validate_no_code_signing() {
        use der::Encode;

        let cert = parse_test_cert();

        // Replace ExtendedKeyUsage with one that only has serverAuth (not codeSigning)
        let eku_oid: der::asn1::ObjectIdentifier = EXT_KEY_USAGE_OID.parse().unwrap();
        let server_auth_oid: der::asn1::ObjectIdentifier = "1.3.6.1.5.5.7.3.1".parse().unwrap();
        let eku = ExtendedKeyUsage(vec![server_auth_oid]);
        let eku_der = eku.to_der().unwrap();

        let modified = map_cert_extensions(&cert, |exts| {
            exts.iter()
                .map(|ext| {
                    if ext.extn_id == eku_oid {
                        x509_cert::ext::Extension {
                            extn_id: ext.extn_id.clone(),
                            critical: ext.critical,
                            extn_value: der::asn1::OctetString::new(eku_der.clone()).unwrap(),
                        }
                    } else {
                        ext.clone()
                    }
                })
                .collect()
        });

        let err = validate_certificate_extensions(&modified).unwrap_err();
        assert!(
            err.to_string().contains("codeSigning"),
            "Expected codeSigning error, got: {}",
            err
        );
    }
}

#[cfg(test)]
mod identity_tests {
    use super::*;

    /// A certificate identity that passes every check, as a starting point for
    /// mutation: each test below breaks exactly one thing.
    fn valid_info() -> CertificateInfo {
        CertificateInfo {
            issuer: crate::GITHUB_OIDC_ISSUER.to_string(),
            subject_workflow:
                "https://github.com/owner/repo/.github/workflows/release.yml@refs/tags/v1.2.3"
                    .to_string(),
            repository: "owner/repo".to_string(),
            runner_environment: RUNNER_ENVIRONMENT_GITHUB_HOSTED.to_string(),
            source_repository_ref: "refs/tags/v1.2.3".to_string(),
            source_repository_digest: "a".repeat(40),
            build_signer_digest: "b".repeat(40),
        }
    }

    fn expected() -> ExpectedIdentity {
        ExpectedIdentity::code("owner/repo", Some("v1.2.3"))
    }

    #[test]
    fn a_conforming_identity_is_accepted() {
        check_identity(&valid_info(), &expected()).expect("the baseline must pass");
    }

    #[test]
    fn a_non_github_issuer_is_refused() {
        let mut info = valid_info();
        info.issuer = "https://accounts.google.com".into();
        let err = check_identity(&info, &expected()).unwrap_err();
        assert!(err.to_string().contains("not from GitHub Actions"), "{err}");
    }

    /// The core of the audit's "trusted verifier input" requirement: the
    /// repository is supplied by the caller, and a certificate naming a
    /// different one must be refused no matter how well-formed it is.
    #[test]
    fn a_certificate_for_another_repository_is_refused() {
        let mut info = valid_info();
        info.repository = "attacker/repo".into();
        info.subject_workflow =
            "https://github.com/attacker/repo/.github/workflows/release.yml@refs/tags/v1.2.3"
                .into();
        let err = check_identity(&info, &expected()).unwrap_err();
        assert!(
            err.to_string().contains("repository does not match"),
            "{err}"
        );
    }

    /// A self-hosted runner can be attacker-controlled, so upstream requires
    /// GitHub-hosted. An absent extension must be refused, not assumed.
    #[test]
    fn a_self_hosted_or_absent_runner_is_refused() {
        for runner in ["self-hosted", ""] {
            let mut info = valid_info();
            info.runner_environment = runner.into();
            let err = check_identity(&info, &expected()).unwrap_err();
            assert!(
                err.to_string().contains("GitHub-hosted runner"),
                "{runner:?} must be refused, got {err}"
            );
        }
    }

    /// Branches move; tags do not. A build from `refs/heads/main` must not be
    /// presentable as a release.
    ///
    /// Two independent checks can catch this — the ref-pattern match against
    /// OID .1.14, and the `refs/tags/` prefix requirement — so the assertion is
    /// on the outcome rather than on which one fired first.
    #[test]
    fn a_non_tag_ref_is_refused() {
        for git_ref in ["refs/heads/main", "refs/pull/1/merge", ""] {
            let mut info = valid_info();
            info.source_repository_ref = git_ref.into();
            let err = check_identity(&info, &expected()).unwrap_err();
            let message = err.to_string();
            assert!(
                message.contains("not a tag") || message.contains("Build ref does not match"),
                "{git_ref:?} must be refused for being a non-tag ref, got {err}"
            );
        }
    }

    /// The ref extension (OID .1.14) is checked independently of the workflow
    /// URI. A certificate whose SAN names a tag but whose ref extension says
    /// otherwise must be refused: they are separate assertions, and satisfying
    /// only one of them is not enough.
    #[test]
    fn the_ref_extension_is_checked_independently_of_the_san() {
        let mut info = valid_info();
        // SAN still says refs/tags/v1.2.3 (see `valid_info`), but the
        // dedicated extension disagrees.
        info.source_repository_ref = "refs/heads/main".into();
        let err = check_identity(&info, &ExpectedIdentity::code("owner/repo", None)).unwrap_err();
        assert!(
            err.to_string().contains("Build ref"),
            "a disagreeing ref extension must be refused, got {err}"
        );
    }

    /// A freshness witness is published continuously from `main`, so the
    /// tag requirement must not apply to it — while every other constraint
    /// still does.
    #[test]
    fn a_freshness_identity_accepts_a_branch_ref() {
        let expected = ExpectedIdentity::freshness_witness();
        assert!(!expected.expects_tag_ref());

        let mut info = valid_info();
        info.repository = crate::FRESHNESS_WITNESS_REPO.into();
        info.subject_workflow = format!(
            "https://github.com/{}/.github/workflows/freshness.yml@refs/heads/main",
            crate::FRESHNESS_WITNESS_REPO
        );
        info.source_repository_ref = "refs/heads/main".into();
        check_identity(&info, &expected).expect("a branch-signed witness must be accepted");

        // Still github-hosted-only, and still that repository only.
        let mut self_hosted = info.clone();
        self_hosted.runner_environment = "self-hosted".into();
        assert!(check_identity(&self_hosted, &expected).is_err());

        let mut other_repo = info.clone();
        other_repo.repository = "attacker/repo".into();
        assert!(check_identity(&other_repo, &expected).is_err());
    }

    /// The platform-endorsements identity is deliberately narrow: one
    /// publisher, one workflow file, `v`-prefixed release tags only.
    #[test]
    fn the_platform_endorsements_identity_is_narrow() {
        let expected = ExpectedIdentity::platform_endorsements(None);
        let repo = crate::policy::PLATFORM_ENDORSEMENTS_REPO;

        let mut info = valid_info();
        info.repository = repo.into();
        info.subject_workflow =
            format!("https://github.com/{repo}/.github/workflows/build.yml@refs/tags/v1.0.0");
        info.source_repository_ref = "refs/tags/v1.0.0".into();
        check_identity(&info, &expected).expect("the canonical endorsements identity must pass");

        // A different workflow file in the same repository is refused.
        let mut other_workflow = info.clone();
        other_workflow.subject_workflow =
            format!("https://github.com/{repo}/.github/workflows/other.yml@refs/tags/v1.0.0");
        assert!(check_identity(&other_workflow, &expected).is_err());

        // A tag that is not `v`-prefixed is refused.
        let mut bad_tag = info.clone();
        bad_tag.subject_workflow =
            format!("https://github.com/{repo}/.github/workflows/build.yml@refs/tags/nightly");
        bad_tag.source_repository_ref = "refs/tags/nightly".into();
        assert!(check_identity(&bad_tag, &expected).is_err());

        // And another repository cannot publish endorsements at all.
        let mut other_repo = info.clone();
        other_repo.repository = "attacker/repo".into();
        other_repo.subject_workflow =
            "https://github.com/attacker/repo/.github/workflows/build.yml@refs/tags/v1.0.0".into();
        assert!(check_identity(&other_repo, &expected).is_err());
    }

    #[test]
    fn a_mismatched_tag_is_refused() {
        let mut info = valid_info();
        info.source_repository_ref = "refs/tags/v9.9.9".into();
        let err = check_identity(&info, &expected()).unwrap_err();
        assert!(
            err.to_string().contains("does not match the expected tag"),
            "{err}"
        );
    }

    /// With no tag pinned, any tag ref is acceptable — but it must still be a
    /// tag.
    #[test]
    fn an_unpinned_tag_accepts_any_tag_but_not_a_branch() {
        let unpinned = ExpectedIdentity::code("owner/repo", None);

        let mut info = valid_info();
        info.source_repository_ref = "refs/tags/anything".into();
        info.subject_workflow =
            "https://github.com/owner/repo/.github/workflows/release.yml@refs/tags/anything".into();
        assert!(check_identity(&info, &unpinned).is_ok());

        info.source_repository_ref = "refs/heads/main".into();
        assert!(check_identity(&info, &unpinned).is_err());
    }

    #[test]
    fn a_malformed_commit_digest_is_refused() {
        for digest in ["", "not-hex", &"a".repeat(39), &"A".repeat(40)] {
            let mut info = valid_info();
            info.source_repository_digest = digest.to_string();
            let err = check_identity(&info, &expected()).unwrap_err();
            assert!(
                err.to_string().contains("not a commit SHA"),
                "{digest:?} must be refused, got {err}"
            );
        }
    }

    /// The workflow must live directly under `.github/workflows`. A nested
    /// path would let a build from an unexpected location sign as this repo.
    #[test]
    fn a_workflow_outside_the_expected_path_is_refused() {
        for workflow in [
            // Nested one level deeper than the pattern allows.
            "https://github.com/owner/repo/.github/workflows/nested/evil.yml@refs/tags/v1.2.3",
            // A different repository's workflow.
            "https://github.com/other/repo/.github/workflows/release.yml@refs/tags/v1.2.3",
            // Not a workflow URI at all.
            "https://example.invalid/owner/repo@refs/tags/v1.2.3",
            // Padding must not slip past the anchors.
            "xhttps://github.com/owner/repo/.github/workflows/release.yml@refs/tags/v1.2.3",
            // A second `@` must not be smuggleable: both the workflow class
            // ([^/@]+) and the ref class ([^@]+) exclude it precisely so a
            // trailing `@...` cannot be appended to an otherwise valid URI.
            "https://github.com/owner/repo/.github/workflows/release.yml@refs/tags/v1@refs/tags/v2",
        ] {
            let mut info = valid_info();
            info.subject_workflow = workflow.into();
            assert!(
                check_identity(&info, &ExpectedIdentity::code("owner/repo", None)).is_err(),
                "{workflow:?} must be refused"
            );
        }
    }

    /// A repository name containing regex metacharacters must be escaped, or
    /// `owner/repo.io` would also match `owner/repoXio`.
    #[test]
    fn the_repository_is_regex_escaped_in_the_pattern() {
        let expected = ExpectedIdentity::code("owner/repo.io", None);
        assert!(expected.workflow_regex().contains(r"repo\.io"));

        let mut info = valid_info();
        info.repository = "owner/repo.io".into();
        info.subject_workflow =
            "https://github.com/ownerXrepoXio/.github/workflows/release.yml@refs/tags/v1".into();
        assert!(
            check_identity(&info, &expected).is_err(),
            "an unescaped dot must not match an arbitrary character"
        );
    }

    /// The extraction path and the check path must agree: a DER-encoded
    /// provider-generic extension has to arrive as a usable string.
    #[test]
    fn der_encoded_extension_values_decode() {
        // UTF8String "github-hosted", as Fulcio encodes OID .1.11.
        let mut der = vec![0x0C, 13];
        der.extend_from_slice(b"github-hosted");
        assert_eq!(
            decode_asn1_string(&der).as_deref(),
            Some(RUNNER_ENVIRONMENT_GITHUB_HOSTED)
        );

        // And a tag ref, as encoded for OID .1.14.
        let mut der = vec![0x0C, 16];
        der.extend_from_slice(b"refs/tags/v1.2.3");
        assert_eq!(
            decode_asn1_string(&der).as_deref(),
            Some("refs/tags/v1.2.3")
        );
    }
}
