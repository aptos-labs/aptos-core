//! Offline verification of Atlas's Tinfoil v3 envelope with legacy collateral
//! FormatV2. Config/runtime collateral FormatV3 is explicitly unsupported.
//!
//! This crate contains no service, CLI, configuration-file, or network client
//! code. Evidence and all verification collateral are supplied by the caller.

mod config;

pub mod envelope;
pub mod provenance;
pub mod tdx;
#[cfg(any(test, feature = "testing"))]
pub mod testdata;

pub use aptos_tinfoil_sigstore::provenance::{AuthenticatedArtifact, Shape};
pub use config::AttestationConfig;

/// Challenge nonce length fixed by the attestation envelope protocol.
pub const NONCE_SIZE: usize = 32;

use envelope::{
    Document, KEY_SPKI_FP_SHA256_V1_FORMAT, KEY_X25519_HPKE_V1_FORMAT, TDX_QUOTE_V1_FORMAT,
};
use provenance::{Freshness, PlatformEndorsements};
use sha2::{Digest, Sha256};

/// Only authenticated fields are returned. The caller must still bind the
/// TLS fingerprint to the transport certificate before sending secrets.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct VerifiedV3 {
    pub tls_spki_sha256: [u8; 32],
    pub hpke_key: [u8; 32],
    pub release_digest: [u8; 32],
    pub mrtd: [u8; 48],
    pub rtmrs: [[u8; 48]; 4],
    pub tcb_status: String,
    pub release_tag: String,
    pub release_commit: String,
    pub platform_measurement_id: String,
}

/// Intel-authenticated quote and the key material bound into its REPORTDATA.
/// Move must persist this only after `verify_quote_stage` succeeds.
#[derive(Debug, Clone)]
pub struct QuoteStage {
    pub report_data: [u8; 64],
    pub quote_sha256: [u8; 32],
    pub tls_spki_sha256: [u8; 32],
    pub hpke_key: [u8; 32],
    pub quote: tdx::AuthenticatedQuote,
}

/// Sigstore-authenticated release identity and VM shape whose code registers
/// match the previously authenticated quote.
#[derive(Debug, Clone)]
pub struct CodeStage {
    pub artifact: AuthenticatedArtifact,
    pub shape: Shape,
}

/// Sigstore-authenticated host policy matched to the Intel quote and code
/// shape. Its artifact is used to authenticate platform freshness.
#[derive(Debug, Clone)]
pub struct PlatformStage {
    pub artifact: AuthenticatedArtifact,
    pub measurement_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("the trusted repository must be a nonempty owner/name identifier")]
    InvalidRepository,
    #[error("a required trust root is missing or too large")]
    InvalidTrustRoot,
    #[error("attestation envelope rejected: {0}")]
    Envelope(#[from] envelope::EnvelopeError),
    #[error("unsupported CPU evidence format: {0}")]
    UnsupportedCpuEvidence(String),
    #[error("device evidence appraisal is not implemented for this subformat")]
    UnsupportedDeviceEvidence,
    #[error("a GPU VM shape requires authenticated GPU device evidence")]
    UnsupportedGpuShape,
    #[error("stage fragment is bound to a different attestation REPORTDATA")]
    StageBindingMismatch,
    #[error("TDX evidence rejected: {0}")]
    Tdx(#[from] tdx::TdxError),
    #[error("provenance rejected: {0}")]
    Provenance(#[from] provenance::ProvenanceError),
    #[error("the attested RTMR3 is not zero")]
    Rtmr3NotZero,
    #[error("the attested {0} key is missing or has the wrong format")]
    MissingKey(&'static str),
    #[error("an authenticated key or digest is not 32 bytes of lowercase hex")]
    InvalidHex,
}

/// Verify an Atlas Tinfoil v3 envelope using legacy collateral FormatV2
/// entirely offline against the supplied
/// quorum-observed trust roots and chain time. No wall clock, network source,
/// or bundled fallback root participates in this decision.
///
/// This entry point accepts Intel TDX CPU evidence with no discrete device
/// evidence. GPU/NVSwitch evidence requires a separate cryptographic device
/// appraisal; silently treating a hash binding as appraisal would be unsafe.
pub fn verify_v3(
    document_bytes: &[u8],
    expected_nonce: &[u8],
    now_secs: u64,
    trusted_repo: &str,
    intel_root_der: &[u8],
    sigstore_trusted_root_json: &[u8],
) -> Result<VerifiedV3, VerifyError> {
    if !valid_repo(trusted_repo) {
        return Err(VerifyError::InvalidRepository);
    }
    if intel_root_der.is_empty()
        || intel_root_der.len() > 60 * 1024
        || sigstore_trusted_root_json.is_empty()
        || sigstore_trusted_root_json.len() > 60 * 1024
    {
        return Err(VerifyError::InvalidTrustRoot);
    }
    let sigstore_root = std::str::from_utf8(sigstore_trusted_root_json)
        .map_err(|_| VerifyError::InvalidTrustRoot)?;

    let (document, report_data) = Document::check(document_bytes, expected_nonce)?;
    if document.raw.cpu_evidence.format != TDX_QUOTE_V1_FORMAT {
        return Err(VerifyError::UnsupportedCpuEvidence(
            document.raw.cpu_evidence.format,
        ));
    }
    if !document.device_evidence.items.is_empty() {
        return Err(VerifyError::UnsupportedDeviceEvidence);
    }

    let config = AttestationConfig {
        // Signed platform endorsements constrain MRTD/RTMR0 and the TDX
        // module. Code provenance constrains RTMR1/RTMR2. RTMR3 is checked
        // below, so no unconstrained measurement remains.
        allow_any_measurement: true,
        ..AttestationConfig::default()
    };
    let quote =
        tdx::authenticate_with_root(&document, &report_data, &config, now_secs, intel_root_der)?;
    if quote.rtmrs[3].bytes().any(|b| b != b'0') {
        return Err(VerifyError::Rtmr3NotZero);
    }
    let proven = provenance::verify_with_root(
        &document,
        &quote,
        trusted_repo,
        Freshness::Required,
        PlatformEndorsements::Required,
        now_secs,
        sigstore_root,
    )?;
    if proven.shape.is_some_and(|shape| shape.gpus != 0) {
        return Err(VerifyError::UnsupportedGpuShape);
    }

    Ok(VerifiedV3 {
        tls_spki_sha256: required_key(&document, "tls", KEY_SPKI_FP_SHA256_V1_FORMAT)?,
        hpke_key: required_key(&document, "hpke", KEY_X25519_HPKE_V1_FORMAT)?,
        release_digest: hex32(&proven.digest)?,
        mrtd: hex48(&quote.mrtd)?,
        rtmrs: quote
            .rtmrs
            .iter()
            .map(|value| hex48(value))
            .collect::<Result<Vec<_>, _>>()?
            .try_into()
            .map_err(|_| VerifyError::InvalidHex)?,
        tcb_status: quote.tcb_status,
        release_tag: proven.tag,
        release_commit: proven.commit,
        platform_measurement_id: proven.platform_measurement.ok_or(VerifyError::Provenance(
            provenance::ProvenanceError::MissingPlatformCollateral,
        ))?,
    })
}

/// Verify the quote fragment using the chain-supplied Intel root. This is the
/// only stage that trusts CPU evidence; subsequent stages consume these stored
/// authenticated quote claims and match its REPORTDATA commitment.
pub fn verify_quote_stage(
    fragment: &[u8],
    expected_nonce: &[u8],
    proof_time: u64,
    intel_root_der: &[u8],
) -> Result<QuoteStage, VerifyError> {
    check_root(intel_root_der)?;
    let (document, report_data) = check_fragment(fragment, expected_nonce, None)?;
    let config = AttestationConfig {
        allow_any_measurement: true,
        ..AttestationConfig::default()
    };
    let quote =
        tdx::authenticate_with_root(&document, &report_data, &config, proof_time, intel_root_der)?;
    if quote.rtmrs[3].bytes().any(|b| b != b'0') {
        return Err(VerifyError::Rtmr3NotZero);
    }
    Ok(QuoteStage {
        report_data,
        quote_sha256: Sha256::digest(document.raw.cpu_evidence.report_base64.as_bytes()).into(),
        tls_spki_sha256: required_key(&document, "tls", KEY_SPKI_FP_SHA256_V1_FORMAT)?,
        hpke_key: required_key(&document, "hpke", KEY_X25519_HPKE_V1_FORMAT)?,
        quote,
    })
}

/// Verify one code-provenance fragment using the chain-supplied Sigstore root.
pub fn verify_code_stage(
    fragment: &[u8],
    expected_nonce: &[u8],
    trusted_repo: &str,
    sigstore_trusted_root_json: &[u8],
    quote_stage: &QuoteStage,
) -> Result<CodeStage, VerifyError> {
    if !valid_repo(trusted_repo) {
        return Err(VerifyError::InvalidRepository);
    }
    let root = sigstore_root(sigstore_trusted_root_json)?;
    let (document, _) = check_fragment(fragment, expected_nonce, Some(quote_stage.binding()))?;
    let code = provenance::verify_code_stage(&document, &quote_stage.quote, trusted_repo, root)?;
    let shape = code
        .shape
        .ok_or(provenance::ProvenanceError::MissingShape)?;
    if shape.gpus != 0 {
        return Err(VerifyError::UnsupportedGpuShape);
    }
    Ok(CodeStage {
        artifact: code.artifact,
        shape,
    })
}

/// Authenticate the independent code freshness witness against the release
/// already proven by `verify_code_stage`.
pub fn verify_code_freshness_stage(
    fragment: &[u8],
    expected_nonce: &[u8],
    proof_time: u64,
    sigstore_trusted_root_json: &[u8],
    quote_stage: &QuoteStage,
    code_stage: &CodeStage,
) -> Result<(), VerifyError> {
    let root = sigstore_root(sigstore_trusted_root_json)?;
    let (document, _) = check_fragment(fragment, expected_nonce, Some(quote_stage.binding()))?;
    provenance::verify_freshness_stage(
        &document,
        envelope::FRESHNESS_COLLATERAL_ID_CODE,
        &code_stage.artifact,
        proof_time,
        root,
    )?;
    Ok(())
}

/// Authenticate host policy, then appraise the Intel-authenticated host,
/// TDX module, MRTD, and RTMR0 against the release's signed VM shape.
pub fn verify_platform_stage(
    fragment: &[u8],
    expected_nonce: &[u8],
    _proof_time: u64,
    sigstore_trusted_root_json: &[u8],
    quote_stage: &QuoteStage,
    code_stage: &CodeStage,
) -> Result<PlatformStage, VerifyError> {
    let root = sigstore_root(sigstore_trusted_root_json)?;
    let (document, _) = check_fragment(fragment, expected_nonce, Some(quote_stage.binding()))?;
    let platform = provenance::verify_platform_stage(
        &document,
        &quote_stage.quote,
        Some(code_stage.shape),
        root,
    )?;
    Ok(PlatformStage {
        artifact: platform.artifact,
        measurement_id: platform.measurement_id,
    })
}

/// Authenticate the independent platform freshness witness against the
/// endorsed policy artifact previously proven in the platform stage.
pub fn verify_platform_freshness_stage(
    fragment: &[u8],
    expected_nonce: &[u8],
    proof_time: u64,
    sigstore_trusted_root_json: &[u8],
    quote_stage: &QuoteStage,
    platform_stage: &PlatformStage,
) -> Result<(), VerifyError> {
    let root = sigstore_root(sigstore_trusted_root_json)?;
    let (document, _) = check_fragment(fragment, expected_nonce, Some(quote_stage.binding()))?;
    provenance::verify_freshness_stage(
        &document,
        envelope::FRESHNESS_COLLATERAL_ID_PLATFORM,
        &platform_stage.artifact,
        proof_time,
        root,
    )?;
    Ok(())
}

/// Build five collateral-specific fragments from one structurally valid Atlas
/// v3 envelope with legacy collateral FormatV2
/// document for separate on-chain submissions. Every fragment keeps the exact
/// challenge and base64-encoded committed sections; only its collateral array
/// is reduced. This helper does not authenticate any collateral.
pub fn split_stage_fragments(
    document_bytes: &[u8],
    expected_nonce: &[u8],
) -> Result<[Vec<u8>; 5], VerifyError> {
    let (document, report_data) = check_fragment(document_bytes, expected_nonce, None)?;
    let quote_sha256: [u8; 32] =
        Sha256::digest(document.raw.cpu_evidence.report_base64.as_bytes()).into();
    let entries = [
        document
            .endorsement_collateral(
                envelope::COLLATERAL_INTEL_PCS_V1_FORMAT,
                envelope::SUBJECT_CPU,
            )
            .ok_or(VerifyError::Tdx(tdx::TdxError::MissingCollateral))?,
        document
            .reference_values_collateral(envelope::COLLATERAL_SIGSTORE_CODE_V1_FORMAT)
            .ok_or(VerifyError::Provenance(
                provenance::ProvenanceError::MissingCodeCollateral,
            ))?,
        document
            .freshness_collateral(envelope::FRESHNESS_COLLATERAL_ID_CODE)
            .ok_or(VerifyError::Provenance(
                provenance::ProvenanceError::MissingFreshness("code-freshness"),
            ))?,
        document
            .reference_values_collateral(envelope::COLLATERAL_SIGSTORE_PLATFORM_V1_FORMAT)
            .ok_or(VerifyError::Provenance(
                provenance::ProvenanceError::MissingPlatformCollateral,
            ))?,
        document
            .freshness_collateral(envelope::FRESHNESS_COLLATERAL_ID_PLATFORM)
            .ok_or(VerifyError::Provenance(
                provenance::ProvenanceError::MissingFreshness("platform-freshness"),
            ))?,
    ];
    let mut fragments = Vec::with_capacity(entries.len());
    for entry in entries {
        let mut raw = document.raw.clone();
        raw.collateral = vec![entry.clone()];
        let fragment = serde_json::to_vec(&raw).map_err(|error| {
            VerifyError::Envelope(envelope::EnvelopeError::Parse(error.to_string()))
        })?;
        let (_, fragment_report_data) = check_fragment(
            &fragment,
            expected_nonce,
            Some((&report_data, &quote_sha256)),
        )?;
        debug_assert_eq!(fragment_report_data, report_data);
        fragments.push(fragment);
    }
    fragments
        .try_into()
        .map_err(|_| VerifyError::StageBindingMismatch)
}

fn check_root(root: &[u8]) -> Result<(), VerifyError> {
    if root.is_empty() || root.len() > 60 * 1024 {
        return Err(VerifyError::InvalidTrustRoot);
    }
    Ok(())
}

fn sigstore_root(root: &[u8]) -> Result<&str, VerifyError> {
    check_root(root)?;
    std::str::from_utf8(root).map_err(|_| VerifyError::InvalidTrustRoot)
}

fn check_fragment(
    fragment: &[u8],
    expected_nonce: &[u8],
    quote_binding: Option<(&[u8; 64], &[u8; 32])>,
) -> Result<(Document, [u8; 64]), VerifyError> {
    let (document, report_data) = Document::check(fragment, expected_nonce)?;
    if document.raw.cpu_evidence.format != TDX_QUOTE_V1_FORMAT {
        return Err(VerifyError::UnsupportedCpuEvidence(
            document.raw.cpu_evidence.format,
        ));
    }
    if !document.device_evidence.items.is_empty() {
        return Err(VerifyError::UnsupportedDeviceEvidence);
    }
    if let Some((trusted_report_data, trusted_quote_sha256)) = quote_binding {
        let quote_sha256: [u8; 32] =
            Sha256::digest(document.raw.cpu_evidence.report_base64.as_bytes()).into();
        if *trusted_report_data != report_data || *trusted_quote_sha256 != quote_sha256 {
            return Err(VerifyError::StageBindingMismatch);
        }
    }
    Ok((document, report_data))
}

impl QuoteStage {
    fn binding(&self) -> (&[u8; 64], &[u8; 32]) {
        (&self.report_data, &self.quote_sha256)
    }
}

fn valid_repo(repo: &str) -> bool {
    let mut parts = repo.split('/');
    let owner = parts.next().unwrap_or_default();
    let name = parts.next().unwrap_or_default();
    !owner.is_empty()
        && !name.is_empty()
        && parts.next().is_none()
        && [owner, name].into_iter().all(|part| {
            part.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.'))
        })
}

fn required_key(
    document: &Document,
    id: &'static str,
    format: &str,
) -> Result<[u8; 32], VerifyError> {
    let item = document
        .crypto_material_item(id)
        .filter(|item| item.format == format)
        .ok_or(VerifyError::MissingKey(id))?;
    hex32(&item.data)
}

fn hex32(value: &str) -> Result<[u8; 32], VerifyError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(VerifyError::InvalidHex);
    }
    hex::decode(value)
        .map_err(|_| VerifyError::InvalidHex)?
        .try_into()
        .map_err(|_| VerifyError::InvalidHex)
}

fn hex48(value: &str) -> Result<[u8; 48], VerifyError> {
    if value.len() != 96
        || !value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(VerifyError::InvalidHex);
    }
    hex::decode(value)
        .map_err(|_| VerifyError::InvalidHex)?
        .try_into()
        .map_err(|_| VerifyError::InvalidHex)
}

#[cfg(test)]
mod stage_tests {
    use super::*;
    use crate::testdata::DocumentBuilder;

    #[test]
    fn every_fragment_must_match_the_quote_stage_commitment() {
        let original = DocumentBuilder::new().build();
        let (parsed, authenticated_report_data) =
            check_fragment(original.json.as_bytes(), &original.nonce, None).unwrap();
        let authenticated_quote_hash: [u8; 32] =
            Sha256::digest(parsed.raw.cpu_evidence.report_base64.as_bytes()).into();

        let changed_key = DocumentBuilder::new()
            .tls_fingerprint(&"9".repeat(64))
            .build();
        assert!(matches!(
            check_fragment(
                changed_key.json.as_bytes(),
                &original.nonce,
                Some((&authenticated_report_data, &authenticated_quote_hash)),
            ),
            Err(VerifyError::StageBindingMismatch)
        ));

        let changed_nonce = DocumentBuilder::new().nonce([0x43; 32]).build();
        assert!(matches!(
            check_fragment(
                changed_nonce.json.as_bytes(),
                &original.nonce,
                Some((&authenticated_report_data, &authenticated_quote_hash)),
            ),
            Err(VerifyError::Envelope(
                envelope::EnvelopeError::NonceMismatch
            ))
        ));

        let changed_quote = DocumentBuilder::new()
            .report_base64("AQEBAQEBAQEBAQEBAQEBAQ==".to_string())
            .build();
        assert!(matches!(
            check_fragment(
                changed_quote.json.as_bytes(),
                &original.nonce,
                Some((&authenticated_report_data, &authenticated_quote_hash)),
            ),
            Err(VerifyError::StageBindingMismatch)
        ));
    }

    #[test]
    fn fragment_parser_does_not_require_unrelated_collateral() {
        let fragment = DocumentBuilder::new().build();
        let (parsed, _) = check_fragment(fragment.json.as_bytes(), &fragment.nonce, None).unwrap();
        assert!(parsed.raw.collateral.is_empty());
    }

    #[test]
    fn splitter_preserves_report_data_with_one_collateral_per_fragment() {
        let mut builder =
            DocumentBuilder::new().collateral_entry(crate::testdata::tdx_collateral_entry());
        for (id, format) in [
            ("code", envelope::COLLATERAL_SIGSTORE_CODE_V1_FORMAT),
            (
                "code-freshness",
                envelope::COLLATERAL_SIGSTORE_FRESHNESS_V1_FORMAT,
            ),
            ("platform", envelope::COLLATERAL_SIGSTORE_PLATFORM_V1_FORMAT),
            (
                "platform-freshness",
                envelope::COLLATERAL_SIGSTORE_FRESHNESS_V1_FORMAT,
            ),
        ] {
            builder = builder.collateral_entry(serde_json::json!({
                "id": id,
                "role": envelope::ROLE_REFERENCE_VALUES,
                "format": format,
                "subjects": [],
                "data": {},
            }));
        }
        let full = builder.build();
        let (full_parsed, full_report_data) =
            check_fragment(full.json.as_bytes(), &full.nonce, None).unwrap();
        let full_quote_hash: [u8; 32] =
            Sha256::digest(full_parsed.raw.cpu_evidence.report_base64.as_bytes()).into();
        let fragments = split_stage_fragments(full.json.as_bytes(), &full.nonce).unwrap();
        for fragment in fragments {
            let (parsed, report_data) = check_fragment(
                &fragment,
                &full.nonce,
                Some((&full_report_data, &full_quote_hash)),
            )
            .unwrap();
            assert_eq!(report_data, full_report_data);
            assert_eq!(parsed.raw.collateral.len(), 1);
        }
    }
}
