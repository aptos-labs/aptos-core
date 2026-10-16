//! Offline Intel TDX quote verification.
//!
//! Verification is *offline*. Nothing here opens a socket: the TD quote carries
//! its own PCK certificate chain (`cert_type` 5), and the remaining Intel
//! collateral — TCB info, QE identity, and the CRLs — travels inside the
//! attestation document as an endorsement collateral entry. That keeps the
//! crate's contract ("must never consult the network")
//! a build-time property: the `dcap-qvl` dependency is compiled *without* its
//! `report` feature, so the PCCS fetcher and its `reqwest` stack are not even
//! linked in.
//!
//! Requiring the collateral to travel in-document is also what Intel asks for:
//! fetching collateral from the PCS on every verification is disallowed by the
//! PCS terms of use, which mandate caching instead.
//!
//! # What this verifies
//!
//! `dcap-qvl` performs the DCAP appraisal — the quote signature, the QE report,
//! the PCK chain up to the **pinned Intel SGX Root CA** compiled into the
//! library, the signed TCB info and QE identity, and the TCB status. On top of
//! that this module enforces the vault's own policy: the challenge binding
//! (`REPORTDATA`), the MRTD/RTMR allowlists, and the acceptable TCB statuses.
//!
//! # What this does NOT verify
//!
//! * **Sigstore code provenance** — `repo` is not established by attestation at
//!   all. It is bound to a measurement by operator configuration instead
//!   (`[[repositories]].measurements`), enforced by the calling service.
//! * **`MRSEAM` / `MRSIGNERSEAM`** — the TDX module identity is authenticated
//!   by Intel but is not appraised against an operator allowlist here.
//! * **GPU / NVSwitch device evidence** — hash-bound into the quote, but its
//!   contents are not appraised.
//! * **`RTMR3`** — unchecked unless the operator pins `allowed_rtmr3`.
//!   Upstream pins it to 48 zero bytes, but that rule does not transfer here:
//!   this codebase uses `RTMR3` as a runtime event mirror (`ccs-node-agent`
//!   extends it on every launch/drain), so it is legitimately non-zero on a
//!   node-agent host. The appraisal that *would* apply is a replay of the
//!   ordered event log against the attested register, and the v3 document
//!   carries no member holding that log — so replay is not implementable at
//!   this layer today, and pinning is the only available control.

use dcap_qvl::quote::Report;
use dcap_qvl::{QuoteCollateralV3, QuotePolicy, TcbStatus};

use super::envelope::{
    Document, IntelPcsCollateral, PcsResponse, COLLATERAL_INTEL_PCS_V1_FORMAT, SUBJECT_CPU,
};
use crate::AttestationConfig;

/// Smallest plausible TD quote. Purely a cheap sanity bound before handing the
/// bytes to the verifier; `dcap-qvl` does the real structural parsing.
const MIN_QUOTE_SIZE: usize = 1020;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TdxError {
    #[error("document carries no intel-pcs endorsement collateral for the cpu")]
    MissingCollateral,
    #[error("parsing intel-pcs collateral: {0}")]
    BadCollateral(String),
    #[error("cpu_evidence.report_base64 is not canonical base64")]
    BadQuoteEncoding,
    #[error("TDX quote is implausibly small ({0} bytes)")]
    BadQuoteSize(usize),
    #[error("quote REPORTDATA does not match the challenge-derived value")]
    ReportDataMismatch,
    #[error("TDX quote rejected by DCAP verification: {0}")]
    Verification(String),
    #[error("evidence is an SGX enclave report, not a TDX quote")]
    NotTdx,
    #[error("attested MRTD {0} is not in the allowed set")]
    MrtdNotAllowed(String),
    #[error("attested RTMR{index} {value} is not in the allowed set")]
    RtmrNotAllowed { index: usize, value: String },
    #[error("quote carries a non-zero {register}: {value}")]
    NonZeroLaunchInput {
        register: &'static str,
        value: String,
    },
}

/// An authenticated TDX quote.
///
/// Constructing one means Intel's chain verified against the pinned Intel root,
/// the TCB status was acceptable, and the quote bound the expected `REPORTDATA`.
#[derive(Debug, Clone)]
pub struct AuthenticatedQuote {
    /// Launch measurement of the TD (`MRTD`), lowercase hex (48 bytes).
    pub mrtd: String,
    /// Runtime measurement registers `RTMR0..3`, lowercase hex (48 bytes each).
    pub rtmrs: [String; 4],
    /// The TCB status Intel's collateral reports for this platform.
    pub tcb_status: String,
    /// Security advisory IDs applicable to this platform, if any.
    pub advisory_ids: Vec<String>,
    /// Quoting Enclave vendor ID from the quote header, lowercase hex
    /// (16 bytes).
    pub qe_vendor_id: String,
    /// TDX module security version numbers (`TEE_TCB_SVN`), lowercase hex
    /// (16 bytes, one SVN component per byte).
    pub tee_tcb_svn: String,
    /// Measurement of the TDX module (`MRSEAM`), lowercase hex (48 bytes).
    pub mr_seam: String,
    /// TD attributes (`TD_ATTRIBUTES`), lowercase hex (8 bytes).
    ///
    /// Carries the flags that decide whether the TD is debuggable, so a policy
    /// that never appraises this cannot claim the TD's contents are sealed.
    pub td_attributes: String,
    /// Extended features allowed mask (`XFAM`), lowercase hex (8 bytes).
    pub xfam: String,
    /// Intel's `tcbEvaluationDataNumber` for this platform: the monotonically
    /// increasing version of the TCB recovery data the verdict was drawn from.
    pub tcb_evaluation_data_number: i64,
    /// Platform Provisioning ID of the *host*, lowercase hex.
    ///
    /// Recovered by `dcap-qvl` from the PCK certificate's SGX extension
    /// (OID `1.2.840.113741.1.13.1.1`), so it is authenticated by Intel's
    /// chain rather than asserted by the TD.
    ///
    /// This is the key into a platform-endorsements `machines` map: it
    /// identifies *which physical machine* produced the quote, which is what
    /// lets an operator endorse a specific fleet rather than every genuine TDX
    /// host in the world.
    pub ppid: String,
}

/// Map a configuration string onto a `dcap-qvl` TCB status.
///
/// Shared with configuration validation so an unknown status is refused at
/// startup rather than silently ignored at verification time.
pub fn parse_tcb_status(value: &str) -> Option<TcbStatus> {
    match value {
        "UpToDate" => Some(TcbStatus::UpToDate),
        "SWHardeningNeeded" => Some(TcbStatus::SWHardeningNeeded),
        "ConfigurationNeeded" => Some(TcbStatus::ConfigurationNeeded),
        "ConfigurationAndSWHardeningNeeded" => Some(TcbStatus::ConfigurationAndSWHardeningNeeded),
        "OutOfDate" => Some(TcbStatus::OutOfDate),
        "OutOfDateConfigurationNeeded" => Some(TcbStatus::OutOfDateConfigurationNeeded),
        // `Revoked` is deliberately not accepted: a revoked platform must never
        // be configurable as acceptable.
        _ => None,
    }
}

/// Every status an operator is permitted to allow, for error messages.
pub const ALLOWABLE_TCB_STATUSES: &[&str] = &[
    "UpToDate",
    "SWHardeningNeeded",
    "ConfigurationNeeded",
    "ConfigurationAndSWHardeningNeeded",
    "OutOfDate",
    "OutOfDateConfigurationNeeded",
];

/// Authenticate the document's TDX evidence against the pinned Intel root.
///
/// `expected_report_data` comes from [`Document::check`]; requiring the quote to
/// carry it is what ties Intel's signature to *our* challenge nonce, *this* TLS
/// key fingerprint, and the presented device evidence.
///
/// `now_secs` is supplied by the caller rather than read from the clock here so
/// that verification stays a pure function of its inputs (and so tests can pin
/// a time inside a fixture's collateral validity window).
#[cfg(test)]
pub fn authenticate(
    document: &Document,
    expected_report_data: &[u8; 64],
    config: &AttestationConfig,
    now_secs: u64,
) -> Result<AuthenticatedQuote, TdxError> {
    authenticate_with_verifier(
        document,
        expected_report_data,
        config,
        now_secs,
        dcap_qvl::verify::QuoteVerifier::new_prod(),
    )
}

/// Authenticate Intel evidence using the quorum-observed DER trust root.
pub fn authenticate_with_root(
    document: &Document,
    expected_report_data: &[u8; 64],
    config: &AttestationConfig,
    now_secs: u64,
    intel_root_der: &[u8],
) -> Result<AuthenticatedQuote, TdxError> {
    authenticate_with_verifier(
        document,
        expected_report_data,
        config,
        now_secs,
        dcap_qvl::verify::QuoteVerifier::new(intel_root_der.to_vec()),
    )
}

fn authenticate_with_verifier(
    document: &Document,
    expected_report_data: &[u8; 64],
    config: &AttestationConfig,
    now_secs: u64,
    verifier: dcap_qvl::verify::QuoteVerifier,
) -> Result<AuthenticatedQuote, TdxError> {
    let quote = decode_quote(&document.raw.cpu_evidence.report_base64)?;
    let collateral = load_collateral(document)?;

    // Intel's appraisal first: everything below reads the *authenticated*
    // report body, never the raw bytes the client supplied.
    //
    // The caller-selected verifier supplies the Intel root. Policy defaults
    // still disallow debug TDs, including on the chain-root path.
    let claims = verifier
        .verify_with_policy(
            &quote,
            collateral,
            now_secs,
            &build_policy(config, now_secs),
        )
        .map_err(|err| TdxError::Verification(err.to_string()))?;

    let td = match claims.report {
        Report::TD10(report) => report,
        Report::TD15(report) => report.base,
        Report::SgxEnclave(_) => return Err(TdxError::NotTdx),
    };

    // Challenge binding, checked against the authenticated body.
    if td.report_data != expected_report_data[..] {
        return Err(TdxError::ReportDataMismatch);
    }

    // VMM-supplied launch inputs must be zero.
    //
    // MR_CONFIG_ID, MR_OWNER and MR_OWNER_CONFIG are measured into the quote
    // body but *not* into MRTD, so nothing else in this verifier constrains
    // them: an endorsed host could populate them arbitrarily and every other
    // check would still pass. They are also host-controlled rather than
    // guest-controlled, which makes them the one input a compromised-but-
    // endorsed host could otherwise use freely.
    //
    // `cvmimage` never sets them, and the reference verifier pins all three to
    // 48 zero bytes unconditionally, so this is a fixed expectation rather than
    // operator policy.
    check_zero("MRCONFIGID", &td.mr_config_id)?;
    check_zero("MROWNER", &td.mr_owner)?;
    check_zero("MROWNERCONFIG", &td.mr_owner_config)?;

    let authenticated = AuthenticatedQuote {
        mrtd: hex::encode(td.mr_td),
        rtmrs: [
            hex::encode(td.rt_mr0),
            hex::encode(td.rt_mr1),
            hex::encode(td.rt_mr2),
            hex::encode(td.rt_mr3),
        ],
        tcb_status: claims.tcb.status.to_string(),
        advisory_ids: claims.tcb.advisory_ids.clone(),
        qe_vendor_id: hex::encode(claims.header.qe_vendor_id),
        tee_tcb_svn: hex::encode(td.tee_tcb_svn),
        mr_seam: hex::encode(td.mr_seam),
        td_attributes: hex::encode(td.td_attributes),
        xfam: hex::encode(td.xfam),
        tcb_evaluation_data_number: i64::from(claims.tcb.eval_data_number),
        ppid: hex::encode(&claims.platform.pck.ppid),
    };
    authenticated.check_measurements(config)?;
    Ok(authenticated)
}

impl AuthenticatedQuote {
    /// Apply the operator's MRTD and RTMR expectations.
    ///
    /// An empty allowlist means "unconstrained" — for MRTD that is only
    /// reachable when the operator set `allow_any_measurement`, which
    /// configuration validation forces them to do explicitly.
    pub fn check_measurements(&self, config: &AttestationConfig) -> Result<(), TdxError> {
        if !config.allowed_mrtd.is_empty() && !config.allowed_mrtd.contains(&self.mrtd) {
            return Err(TdxError::MrtdNotAllowed(self.mrtd.clone()));
        }
        for (index, allowed) in config.allowed_rtmrs().iter().enumerate() {
            let Some(actual) = self.rtmrs.get(index) else {
                continue;
            };
            if !allowed.is_empty() && !allowed.contains(actual) {
                return Err(TdxError::RtmrNotAllowed {
                    index,
                    value: actual.clone(),
                });
            }
        }
        Ok(())
    }
}

/// Build the DCAP policy from operator configuration.
///
/// The baseline is `QuotePolicy::strict`, which accepts only an `UpToDate`
/// platform. Operators widen it explicitly, one status at a time.
/// The three platform flags are *provisioning* characteristics reported by the
/// PCK certificate, not statements about this TD. `strict()` refuses all three,
/// which would reject essentially every commercially available cloud TDX host
/// even while its TCB is `UpToDate`, so they are operator-controlled and
/// default to permitted. See [`AttestationConfig`] for the per-flag reasoning.
fn build_policy(config: &AttestationConfig, now_secs: u64) -> QuotePolicy {
    let mut policy = QuotePolicy::strict(now_secs)
        .allow_dynamic_platform(config.allow_dynamic_platform)
        .allow_cached_keys(config.allow_cached_keys)
        .allow_smt(config.allow_smt);
    for status in &config.allowed_tcb_statuses {
        if let Some(status) = parse_tcb_status(status) {
            policy = policy.allow_status(status);
        }
    }
    policy
}

/// Pull the Intel collateral out of the document and adapt it for `dcap-qvl`.
///
/// The wire format carries Intel PCS responses captured *verbatim* (see
/// [`IntelPcsCollateral`]), because that is what `cvmimage` produces and what
/// upstream's `tinfoil-go` verifier replays. `dcap-qvl` instead wants a
/// pre-assembled [`QuoteCollateralV3`], so this function performs the
/// translation that upstream gets for free from `go-tdx-guest`'s HTTP getter
/// seam: it indexes the captured responses by URL, recovers the issuer chains
/// from the response *headers*, and splits the signed documents out of the
/// bodies.
///
/// Two properties are load-bearing:
///
/// 1. **Byte-exactness.** Intel's signature covers the exact `tcbInfo` and
///    `enclaveIdentity` bytes. They are extracted as `RawValue` and never
///    re-serialized, because re-serializing would reorder members and
///    invalidate the signature.
/// 2. **No network.** Everything is recovered from the document. This function
///    cannot fetch, which keeps the verifier's offline contract intact.
///
/// This is untrusted transport: `dcap-qvl` re-verifies every signature against
/// the pinned Intel root afterwards, so a substituted capture can only cause
/// verification to fail, never to wrongly succeed.
fn load_collateral(document: &Document) -> Result<QuoteCollateralV3, TdxError> {
    let entry = document
        .endorsement_collateral(COLLATERAL_INTEL_PCS_V1_FORMAT, SUBJECT_CPU)
        .ok_or(TdxError::MissingCollateral)?;
    let collateral: IntelPcsCollateral = serde_json::from_value(entry.data.clone())
        .map_err(|err| TdxError::BadCollateral(err.to_string()))?;

    let mut pck_crl = None;
    let mut pck_crl_issuer_chain = None;
    let mut tcb = None;
    let mut qe_identity = None;
    let mut root_ca_crl = None;

    for response in &collateral.responses {
        let body = decode_body(&response.url, &response.body_base64)?;
        match classify(&response.url) {
            PcsEndpoint::PckCrl => {
                pck_crl_issuer_chain = Some(header(
                    response,
                    &["SGX-PCK-CRL-Issuer-Chain"],
                    &response.url,
                )?);
                pck_crl = Some(body);
            },
            PcsEndpoint::TcbInfo => {
                // Intel served this one under two header spellings over time.
                tcb = Some((
                    header(
                        response,
                        &["SGX-TCB-Info-Issuer-Chain", "TCB-Info-Issuer-Chain"],
                        &response.url,
                    )?,
                    body,
                ));
            },
            PcsEndpoint::QeIdentity => {
                qe_identity = Some((
                    header(
                        response,
                        &["SGX-Enclave-Identity-Issuer-Chain"],
                        &response.url,
                    )?,
                    body,
                ));
            },
            PcsEndpoint::RootCaCrl => root_ca_crl = Some(body),
            PcsEndpoint::Unknown => {
                return Err(TdxError::BadCollateral(format!(
                    "captured response for an unrecognized PCS endpoint {:?}",
                    response.url
                )));
            },
        }
    }

    let (tcb_info_issuer_chain, tcb_body) =
        tcb.ok_or_else(|| missing("a TCB info (/tcb) response"))?;
    let (qe_identity_issuer_chain, qe_body) =
        qe_identity.ok_or_else(|| missing("a QE identity (/qe/identity) response"))?;
    let (tcb_info, tcb_info_signature) = split_signed(&tcb_body, "tcbInfo")?;
    let (qe_identity, qe_identity_signature) = split_signed(&qe_body, "enclaveIdentity")?;

    Ok(QuoteCollateralV3 {
        pck_crl_issuer_chain: pck_crl_issuer_chain
            .ok_or_else(|| missing("a PCK CRL (/pckcrl) response"))?,
        root_ca_crl: root_ca_crl.ok_or_else(|| missing("a root CA CRL response"))?,
        pck_crl: pck_crl.ok_or_else(|| missing("a PCK CRL (/pckcrl) response"))?,
        tcb_info_issuer_chain,
        tcb_info,
        tcb_info_signature,
        qe_identity_issuer_chain,
        qe_identity,
        qe_identity_signature,
        // The TD quote carries its own PCK chain (`cert_type` 5), so there is
        // nothing to supply here.
        pck_certificate_chain: None,
    })
}

/// Refuse a launch input the VMM should have left zeroed.
fn check_zero(register: &'static str, value: &[u8; 48]) -> Result<(), TdxError> {
    if value.iter().any(|byte| *byte != 0) {
        return Err(TdxError::NonZeroLaunchInput {
            register,
            value: hex::encode(value),
        });
    }
    Ok(())
}

fn missing(what: &str) -> TdxError {
    TdxError::BadCollateral(format!("intel-pcs collateral carries no {what}"))
}

/// The Intel PCS endpoints whose responses the collateral must carry.
enum PcsEndpoint {
    PckCrl,
    TcbInfo,
    QeIdentity,
    RootCaCrl,
    Unknown,
}

/// Classify a captured response by its URL path.
///
/// Matching is on the path only: the host varies (Intel's PCS, a PCCS mirror)
/// and the query string carries per-platform parameters (`fmspc`, `ca`).
fn classify(url: &str) -> PcsEndpoint {
    let without_scheme = url.split("://").nth(1).unwrap_or(url);
    let path_and_query = without_scheme.split_once('/').map(|(_, rest)| rest);
    let Some(path_and_query) = path_and_query else {
        return PcsEndpoint::Unknown;
    };
    let path = path_and_query
        .split(['?', '#'])
        .next()
        .unwrap_or(path_and_query)
        .trim_end_matches('/');

    if path.ends_with("/pckcrl") {
        PcsEndpoint::PckCrl
    } else if path.ends_with("/tcb") {
        PcsEndpoint::TcbInfo
    } else if path.ends_with("/qe/identity") {
        PcsEndpoint::QeIdentity
    } else if path.ends_with(".der") || path.ends_with("/rootcacrl") {
        // Intel serves the root CA CRL from a `.der` URL taken from the root
        // certificate's CRL distribution point; a PCCS mirror exposes
        // `/rootcacrl`.
        PcsEndpoint::RootCaCrl
    } else {
        PcsEndpoint::Unknown
    }
}

/// Recover a response header, trying each accepted spelling in turn.
///
/// Header names are matched case-insensitively (HTTP header names are
/// case-insensitive, and captures preserve whatever casing the server used),
/// and values are percent-decoded because Intel percent-encodes the PEM issuer
/// chains it returns in headers.
fn header(response: &PcsResponse, names: &[&str], url: &str) -> Result<String, TdxError> {
    for name in names {
        let found = response
            .headers
            .iter()
            .find(|(key, _)| key.eq_ignore_ascii_case(name));
        if let Some((_, values)) = found {
            let value = values.first().ok_or_else(|| {
                TdxError::BadCollateral(format!("captured header {name:?} for {url:?} is empty"))
            })?;
            return percent_decode(value).ok_or_else(|| {
                TdxError::BadCollateral(format!(
                    "captured header {name:?} for {url:?} is not valid percent-encoding"
                ))
            });
        }
    }
    Err(TdxError::BadCollateral(format!(
        "captured response for {url:?} is missing the {} header",
        names[0]
    )))
}

/// Percent-decode a header value, requiring the result to be UTF-8.
///
/// Implemented locally rather than pulling in a dependency: the input is a PEM
/// certificate chain, and the grammar is three characters wide.
fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let hex = bytes.get(index + 1..index + 3)?;
            let hex = std::str::from_utf8(hex).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            index += 3;
        } else {
            out.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Decode a captured body, insisting on canonical base64.
fn decode_body(url: &str, body_base64: &str) -> Result<Vec<u8>, TdxError> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine.decode(body_base64).map_err(|_| {
        TdxError::BadCollateral(format!("captured body for {url:?} is not valid base64"))
    })?;
    if engine.encode(&bytes) != body_base64 {
        return Err(TdxError::BadCollateral(format!(
            "captured body for {url:?} is not canonical base64"
        )));
    }
    Ok(bytes)
}

/// Split an Intel-signed PCS document into its signed payload and signature.
///
/// The payload is returned as the *exact* bytes Intel signed. `serde_json`'s
/// `RawValue` is what makes that possible: parsing into a `Value` and
/// re-serializing would reorder members and break signature verification.
fn split_signed(body: &[u8], member: &str) -> Result<(String, Vec<u8>), TdxError> {
    let parsed: std::collections::BTreeMap<String, Box<serde_json::value::RawValue>> =
        serde_json::from_slice(body).map_err(|err| {
            TdxError::BadCollateral(format!("captured {member} response is not JSON: {err}"))
        })?;

    let payload = parsed
        .get(member)
        .ok_or_else(|| TdxError::BadCollateral(format!("captured response has no {member:?}")))?
        .get()
        .to_owned();

    let signature = parsed
        .get("signature")
        .ok_or_else(|| {
            TdxError::BadCollateral(format!("captured {member} response has no signature"))
        })?
        .get();
    let signature: String = serde_json::from_str(signature)
        .map_err(|_| TdxError::BadCollateral(format!("{member} signature is not a JSON string")))?;
    let signature = hex::decode(&signature)
        .map_err(|_| TdxError::BadCollateral(format!("{member} signature is not valid hex")))?;

    Ok((payload, signature))
}

/// Decode the quote, insisting on canonical base64.
fn decode_quote(report_base64: &str) -> Result<Vec<u8>, TdxError> {
    use base64::Engine;
    let engine = base64::engine::general_purpose::STANDARD;
    let bytes = engine
        .decode(report_base64)
        .map_err(|_| TdxError::BadQuoteEncoding)?;
    if engine.encode(&bytes) != report_base64 {
        return Err(TdxError::BadQuoteEncoding);
    }
    if bytes.len() < MIN_QUOTE_SIZE {
        return Err(TdxError::BadQuoteSize(bytes.len()));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testdata;

    /// Build a parsed document from a fixture builder.
    fn document(fixture: &testdata::Fixture) -> Document {
        let (doc, _) =
            Document::check(fixture.json.as_bytes(), &fixture.nonce).expect("fixture parses");
        doc
    }

    fn permissive() -> AttestationConfig {
        AttestationConfig {
            allow_any_measurement: true,
            ..Default::default()
        }
    }

    /// The anchor test: a genuine Intel-signed quote, verified against the
    /// pinned Intel root with no network access whatsoever.
    #[test]
    fn authenticates_a_genuine_intel_signed_quote() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let quote = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &permissive(),
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .expect("a genuine Intel-signed quote must verify");

        assert_eq!(quote.mrtd, testdata::TDX_QUOTE_MRTD);
        assert_eq!(quote.rtmrs[0], testdata::TDX_QUOTE_RTMR0);
        assert_eq!(quote.tcb_status, "UpToDate");

        // The PPID identifies the *host*, and comes from the PCK certificate's
        // SGX extension rather than from anything the TD asserts. It is the key
        // into a platform-endorsements `machines` map, so a genuine quote must
        // actually yield one: 16 bytes of lowercase hex.
        assert_eq!(
            quote.ppid.len(),
            32,
            "PPID must be 16 bytes of hex, got {:?}",
            quote.ppid
        );
        assert!(
            quote
                .ppid
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)),
            "PPID must be lowercase hex, got {:?}",
            quote.ppid
        );
        // Not all-zeros: that would mean the extension was absent and we
        // silently defaulted, which would key every host to the same entry.
        assert_ne!(
            quote.ppid,
            "0".repeat(32),
            "PPID must not be a zero default"
        );
    }

    #[test]
    fn supplied_intel_root_is_the_only_quote_trust_anchor() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let doc = document(&fixture);
        let genuine_root = include_bytes!("../../../third_party/dcap-qvl/src/TrustedRootCA.der");

        authenticate_with_root(
            &doc,
            &testdata::tdx_quote_report_data(),
            &permissive(),
            testdata::TDX_COLLATERAL_VALID_AT,
            genuine_root,
        )
        .expect("the supplied Intel root accepts the genuine quote");

        let wrong_root = vec![0; genuine_root.len()];
        assert!(matches!(
            authenticate_with_root(
                &doc,
                &testdata::tdx_quote_report_data(),
                &permissive(),
                testdata::TDX_COLLATERAL_VALID_AT,
                &wrong_root,
            ),
            Err(TdxError::Verification(_))
        ));
    }

    /// The binding that makes the whole protocol work: a real quote is still
    /// refused when it does not commit to the challenge we issued.
    #[test]
    fn rejects_a_genuine_quote_bound_to_a_different_challenge() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let err = authenticate(
            &document(&fixture),
            &[0x11; 64],
            &permissive(),
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .unwrap_err();
        assert_eq!(err, TdxError::ReportDataMismatch);
    }

    #[test]
    fn rejects_a_tampered_quote() {
        let fixture = testdata::DocumentBuilder::new()
            .with_tampered_tdx_quote()
            .build();
        let err = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &permissive(),
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .unwrap_err();
        assert!(
            matches!(
                err,
                TdxError::Verification(_) | TdxError::ReportDataMismatch
            ),
            "a quote whose body was altered must be refused, got {err}"
        );
    }

    #[test]
    fn rejects_evidence_without_intel_collateral() {
        let fixture = testdata::DocumentBuilder::new()
            .with_tdx_quote_missing_collateral()
            .build();
        let err = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &permissive(),
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .unwrap_err();
        assert_eq!(err, TdxError::MissingCollateral);
    }

    /// Expired collateral must fail closed rather than be waved through.
    #[test]
    fn rejects_expired_collateral() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        // Well past the fixture collateral's 2025-07-19 nextUpdate.
        let long_after = testdata::TDX_COLLATERAL_VALID_AT + 400 * 24 * 60 * 60;
        let err = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &permissive(),
            long_after,
        )
        .unwrap_err();
        assert!(
            matches!(err, TdxError::Verification(_)),
            "expired collateral must be refused, got {err}"
        );
    }

    /// MR_CONFIG_ID, MR_OWNER and MR_OWNER_CONFIG are VMM-supplied launch
    /// inputs measured into the quote body but *not* into MRTD, so nothing
    /// else here constrains them. They are pinned to zero.
    ///
    /// This exercises the predicate directly rather than through
    /// `authenticate`: the registers live inside Intel's signed report body, so
    /// a fixture with a non-zero value could not also carry a valid signature —
    /// verification would fail earlier, and for the wrong reason.
    #[test]
    fn vmm_supplied_launch_inputs_must_be_zero() {
        assert_eq!(check_zero("MRCONFIGID", &[0u8; 48]), Ok(()));

        // A single non-zero byte anywhere is enough to refuse the quote.
        for index in [0usize, 23, 47] {
            let mut populated = [0u8; 48];
            populated[index] = 1;
            let err = check_zero("MROWNER", &populated).unwrap_err();
            assert_eq!(
                err,
                TdxError::NonZeroLaunchInput {
                    register: "MROWNER",
                    value: hex::encode(populated),
                },
                "a non-zero byte at {index} must be refused"
            );
        }
    }

    /// The genuine Intel-signed quote really does leave all three zeroed, so
    /// the pin above matches what a real `cvmimage` TD produces rather than
    /// only what a synthetic fixture produces.
    #[test]
    fn a_genuine_quote_leaves_the_launch_inputs_zeroed() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &permissive(),
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .expect("a genuine quote must pass the launch-input pin");
    }

    #[test]
    fn enforces_the_mrtd_allowlist() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let config = AttestationConfig {
            allowed_mrtd: vec!["aa".repeat(48)],
            ..Default::default()
        };
        let err = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &config,
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .unwrap_err();
        assert!(matches!(err, TdxError::MrtdNotAllowed(m) if m == testdata::TDX_QUOTE_MRTD));
    }

    #[test]
    fn accepts_an_mrtd_on_the_allowlist() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let config = AttestationConfig {
            allowed_mrtd: vec![testdata::TDX_QUOTE_MRTD.to_string()],
            ..Default::default()
        };
        assert!(authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &config,
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .is_ok());
    }

    #[test]
    fn enforces_the_rtmr_allowlist() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let config = AttestationConfig {
            allow_any_measurement: true,
            allowed_rtmr0: vec!["bb".repeat(48)],
            ..Default::default()
        };
        let err = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &config,
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .unwrap_err();
        assert!(matches!(err, TdxError::RtmrNotAllowed { index: 0, .. }));
    }

    #[test]
    fn rejects_a_quote_that_is_not_canonical_base64() {
        assert_eq!(
            decode_quote("!!!not base64!!!").unwrap_err(),
            TdxError::BadQuoteEncoding
        );
    }

    #[test]
    fn rejects_an_implausibly_small_quote() {
        use base64::Engine;
        let tiny = base64::engine::general_purpose::STANDARD.encode([0u8; 32]);
        assert_eq!(decode_quote(&tiny).unwrap_err(), TdxError::BadQuoteSize(32));
    }

    /// A revoked platform must never be configurable as acceptable.
    #[test]
    fn revoked_is_never_an_allowable_tcb_status() {
        assert!(parse_tcb_status("Revoked").is_none());
        assert!(!ALLOWABLE_TCB_STATUSES.contains(&"Revoked"));
        for status in ALLOWABLE_TCB_STATUSES {
            assert!(parse_tcb_status(status).is_some(), "{status} must parse");
        }
    }

    #[test]
    fn rejects_an_unknown_tcb_status() {
        assert!(parse_tcb_status("NotAStatus").is_none());
    }

    /// The genuine cloud TDX platform reports `dynamic_platform`,
    /// `cached_keys` and `smt_enabled` all true. `QuotePolicy::strict` refuses
    /// all three, so a vault that did not override them would reject every real
    /// cloud host while its TCB was `UpToDate`. This pins that reasoning.
    #[test]
    fn strict_policy_alone_would_reject_a_real_cloud_platform() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let doc = document(&fixture);
        let quote = decode_quote(&doc.raw.cpu_evidence.report_base64).unwrap();
        let collateral = load_collateral(&doc).unwrap();
        let now = testdata::TDX_COLLATERAL_VALID_AT;

        let strict = dcap_qvl::verify::QuoteVerifier::new_prod().verify_with_policy(
            &quote,
            collateral,
            now,
            &QuotePolicy::strict(now),
        );
        assert!(
            strict.is_err(),
            "fixture must exercise the platform-flag policy"
        );
    }

    /// Tightening a platform flag really does refuse the platform, so the
    /// permissive default is a deliberate choice rather than a dead knob.
    #[test]
    fn disallowing_smt_rejects_an_smt_enabled_platform() {
        let fixture = testdata::DocumentBuilder::new()
            .with_real_tdx_quote()
            .build();
        let config = AttestationConfig {
            allow_any_measurement: true,
            allow_smt: false,
            ..Default::default()
        };
        let err = authenticate(
            &document(&fixture),
            &testdata::tdx_quote_report_data(),
            &config,
            testdata::TDX_COLLATERAL_VALID_AT,
        )
        .unwrap_err();
        assert!(
            matches!(err, TdxError::Verification(_)),
            "an SMT-enabled platform must be refused when allow_smt = false, got {err}"
        );
    }
}
