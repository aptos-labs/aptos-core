use core::time::Duration;

use anyhow::{bail, ensure, Context, Result};
use rustls_pki_types::UnixTime;
use scale::Decode;

#[cfg(feature = "default-x509")]
use crate::policy::{PckIdentity, PlatformInfo, Policy, QeInfo, QuoteClaims, TcbVerdict};
use {
    crate::constants::*,
    crate::policy::PckCertFlag,
    crate::qe_identity::{QeIdentity, QeTcbLevel},
    crate::tcb_info::{TcbInfo, TcbLevel, TcbStatus, TcbStatusWithAdvisory, TdxModuleTcbLevel},
    alloc::string::String,
    alloc::vec::Vec,
};

pub use crate::quote::{AuthData, EnclaveReport, Quote};

use crate::{
    config::{Config, CryptoProvider},
    quote::{Report, TDAttributes},
    utils::{
        encode_as_der_with, extract_certs, parse_crls, parse_rfc3339_unix_secs,
        verify_certificate_chain,
    },
};
use crate::{
    quote::{TDReport10, TDReport15},
    QuoteCollateralV3,
};

use rustls_pki_types::CertificateDer;
use serde::{Deserialize, Serialize};

/// Crypto backend configuration for quote verification.
///
/// Holds the signature verification algorithm and SHA-256 implementation
/// needed by the verification logic. Use [`ring::backend()`] or
/// [`rustcrypto::backend()`] to obtain a pre-configured instance.
pub struct CryptoBackend {
    /// ECDSA P-256 SHA-256 algorithm for certificate and raw signature verification
    pub sig_algo: &'static dyn rustls_pki_types::SignatureVerificationAlgorithm,
    /// SHA-256 hash function
    pub sha256: fn(&[u8]) -> [u8; 32],
    /// SHA-384 hash function (used for root_key_id computation)
    pub sha384: fn(&[u8]) -> [u8; 48],
    /// Raw ECDSA `r || s` to DER encoder.
    pub encode_ecdsa: fn(&[u8]) -> Result<Vec<u8>>,
    /// Parse Intel PCK extensions with the configured X.509 backend.
    pub parse_pck_extension: fn(&[u8]) -> Result<crate::intel::PckExtension>,
}

fn backend_for<C: Config>() -> CryptoBackend {
    CryptoBackend {
        sig_algo: C::Crypto::sig_algo(),
        sha256: C::Crypto::sha256,
        sha384: C::Crypto::sha384,
        encode_ecdsa: encode_as_der_with::<C>,
        parse_pck_extension: crate::intel::parse_pck_extension_with::<C>,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TeeType {
    Sgx,
    Tdx,
}

impl TeeType {
    fn from_u32(value: u32) -> Result<Self> {
        match value {
            TEE_TYPE_SGX => Ok(TeeType::Sgx),
            TEE_TYPE_TDX => Ok(TeeType::Tdx),
            _ => bail!("Unsupported TEE type: {value}"),
        }
    }

    fn is_tdx(&self) -> bool {
        matches!(self, TeeType::Tdx)
    }
}

#[cfg(feature = "js")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "js")]
fn format_error_chain(e: &anyhow::Error) -> String {
    use alloc::format;
    let mut msg = format!("{}", e);
    let mut source = e.source();
    while let Some(err) = source {
        msg.push_str(&format!("\n  Caused by: {}", err));
        source = err.source();
    }
    msg
}

#[cfg(feature = "borsh_schema")]
use borsh::BorshSchema;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};
use core::marker::PhantomData;

/// Result of cryptographic quote verification, before policy validation.
///
/// The enclave report is private — it can only be obtained by passing a [`Policy`]
/// via [`validate()`](Self::validate).
///
/// [`QuoteClaims`] is built lazily via [`claims()`](Self::claims) —
/// the `verify()` call itself does the minimum work (crypto only).
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
struct QuoteVerificationResult {
    header: crate::quote::Header,
    report: Report,
    collateral: QuoteCollateralV3,
    #[serde(with = "crate::utils::serde_vec_bytes")]
    pck_cert_chain_der: Vec<Vec<u8>>,
    // -- core verification results (always computed) --
    tee_type: u32,
    tcb_status: TcbStatus,
    advisory_ids: Vec<String>,
    platform_tcb_level: TcbLevel,
    qe_tcb_level: QeTcbLevel,
    pck_ext: PckCertChainResult,
    qe_report: EnclaveReport,
    tcb_eval_data_number: u32,
    qe_tcb_eval_data_number: u32,
    #[serde(with = "serde_bytes")]
    root_key_id: [u8; 48],
}

impl QuoteVerificationResult {
    /// Build the full [`QuoteClaims`] from verification intermediates.
    ///
    /// Computes the collateral time window from all 8 sources (TCBInfo, QEIdentity,
    /// 2 CRLs, 4 certificate chains), root_key_id SHA-384, CRL numbers, and tcb_date_tag.
    #[cfg(feature = "default-x509")]
    pub fn claims(&self) -> Result<QuoteClaims> {
        // Parse collateral JSON for time window computation
        let tcb_info: TcbInfo = serde_json::from_str(&self.collateral.tcb_info)
            .context("Failed to parse TcbInfo for claims")?;
        let qe_identity: QeIdentity = serde_json::from_str(&self.collateral.qe_identity)
            .context("Failed to parse QeIdentity for claims")?;
        let pck_certs: Vec<CertificateDer<'_>> = self
            .pck_cert_chain_der
            .iter()
            .map(|cert| CertificateDer::from(cert.as_slice()))
            .collect();

        let collateral_dates =
            compute_collateral_time_window(&self.collateral, &pck_certs, &tcb_info, &qe_identity)?;

        // root_key_id: SHA-384 of root CA's raw public key bytes
        let root_key_id = self.root_key_id;

        // CRL numbers
        let root_ca_crl_num = crate::utils::extract_crl_number(&self.collateral.root_ca_crl)
            .context("Failed to extract root CA CRL number")?;
        let pck_crl_num = crate::utils::extract_crl_number(&self.collateral.pck_crl)
            .context("Failed to extract PCK CRL number")?;

        // tcb_date_tag
        let tcb_date_tag = parse_rfc3339_unix_secs(&self.platform_tcb_level.tcb_date)
            .context("Failed to parse platform TCB date")?;

        Ok(QuoteClaims {
            claims_version: 1,
            header: self.header,
            tee_type: self.tee_type,
            tcb: TcbVerdict {
                status: self.tcb_status,
                advisory_ids: self.advisory_ids.clone(),
                eval_data_number: self.tcb_eval_data_number,
            },
            platform: PlatformInfo {
                tcb_level: self.platform_tcb_level.clone(),
                tcb_date_tag,
                pck: PckIdentity {
                    ppid: self.pck_ext.ppid.clone(),
                    cpu_svn: self.pck_ext.cpu_svn,
                    pce_svn: self.pck_ext.pce_svn,
                    pce_id: self.pck_ext.pce_id.clone(),
                    fmspc: self.pck_ext.fmspc,
                    sgx_type: self.pck_ext.sgx_type,
                    platform_instance_id: self.pck_ext.platform_instance_id,
                    dynamic_platform: self.pck_ext.dynamic_platform,
                    cached_keys: self.pck_ext.cached_keys,
                    smt_enabled: self.pck_ext.smt_enabled,
                    // Intel's upstream DCAP Rego policy checks
                    // `platform_provider_id`, but the upstream QvE producer
                    // currently leaves it as a TODO when building the platform
                    // measurement JSON:
                    // https://github.com/intel/confidential-computing.tee.dcap/blob/main/ae/QvE/qve/qve.cpp
                    platform_provider_id: None,
                },
                root_key_id: root_key_id.to_vec(),
                pck_crl_num,
                root_ca_crl_num,
            },
            qe: QeInfo {
                tcb_level: self.qe_tcb_level.clone(),
                report: self.qe_report,
                tcb_eval_data_number: self.qe_tcb_eval_data_number,
            },
            report: self.report.clone(),
            earliest_issue_date: collateral_dates.earliest_issue,
            latest_issue_date: collateral_dates.latest_issue,
            earliest_expiration_date: collateral_dates.earliest_expiration,
            qe_iden_earliest_issue_date: collateral_dates.qe_iden_earliest_issue,
            qe_iden_latest_issue_date: collateral_dates.qe_iden_latest_issue,
            qe_iden_earliest_expiration_date: collateral_dates.qe_iden_earliest_expiration,
        })
    }

    /// Convert directly into [`VerifiedReport`] **without applying any policy**.
    ///
    /// # Warning
    /// This skips all policy checks (TCB status, advisory IDs, collateral
    /// freshness, platform flags). Use only when you handle validation
    /// externally or intentionally accept any verification result.
    pub fn into_report_unchecked(self) -> VerifiedReport {
        let platform_status = TcbStatusWithAdvisory::new(
            self.platform_tcb_level.tcb_status,
            self.platform_tcb_level.advisory_ids.clone(),
        );
        let qe_status = TcbStatusWithAdvisory::new(
            self.qe_tcb_level.tcb_status,
            self.qe_tcb_level.advisory_ids.clone(),
        );
        VerifiedReport {
            status: self.tcb_status.to_string(),
            advisory_ids: self.advisory_ids,
            report: self.report,
            ppid: self.pck_ext.ppid,
            platform_status,
            qe_status,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct VerifiedReport {
    pub status: String,
    pub advisory_ids: Vec<String>,
    pub report: Report,
    #[serde(with = "serde_bytes")]
    pub ppid: Vec<u8>,
    pub qe_status: TcbStatusWithAdvisory,
    pub platform_status: TcbStatusWithAdvisory,
}

/// Quote verifier with configurable root certificate and crypto backend.
///
/// Provides both the backwards-compatible report API and the detailed claims API.
pub struct QuoteVerifier<C: Config = crate::configs::DefaultConfig> {
    root_ca_der: Vec<u8>,
    allow_service_td: bool,
    allow_debug: bool,
    config: PhantomData<C>,
}

#[cfg(feature = "default-x509")]
impl QuoteVerifier<crate::configs::DefaultConfig> {
    /// Create a new verifier with a custom root certificate.
    pub fn new(root_ca_der: Vec<u8>) -> Self {
        Self {
            root_ca_der,
            allow_service_td: false,
            allow_debug: false,
            config: PhantomData,
        }
    }

    /// Create a new verifier using Intel's production root certificate.
    pub fn new_prod() -> Self {
        Self::new(TRUSTED_ROOT_CA_DER.to_vec())
    }
}

impl<C: Config> QuoteVerifier<C> {
    /// Create a verifier for `C` with a custom root certificate.
    pub fn new_with_config(root_ca_der: Vec<u8>) -> Self {
        Self {
            root_ca_der,
            allow_service_td: false,
            allow_debug: false,
            config: PhantomData,
        }
    }

    /// Select a different compile-time verification backend.
    pub fn with_config<D: Config>(self) -> QuoteVerifier<D> {
        QuoteVerifier {
            root_ca_der: self.root_ca_der,
            allow_service_td: self.allow_service_td,
            allow_debug: self.allow_debug,
            config: PhantomData,
        }
    }

    pub fn allow_service_td(mut self, allow: bool) -> Self {
        self.allow_service_td = allow;
        self
    }

    pub fn allow_debug(mut self, allow: bool) -> Self {
        self.allow_debug = allow;
        self
    }

    /// Verify a quote, apply a policy, and return detailed serializable claims.
    #[cfg(feature = "default-x509")]
    pub fn verify_with_policy<P: Policy + ?Sized>(
        &self,
        raw_quote: &[u8],
        collateral: impl Into<QuoteCollateralV3>,
        now_secs: u64,
        policy: &P,
    ) -> Result<QuoteClaims> {
        let claims = self
            .verify_result(raw_quote, collateral, now_secs)?
            .claims()?;
        policy.validate(&claims)?;
        Ok(claims)
    }

    fn verify_result(
        &self,
        raw_quote: &[u8],
        collateral: impl Into<QuoteCollateralV3>,
        now_secs: u64,
    ) -> Result<QuoteVerificationResult> {
        let backend = backend_for::<C>();
        verify_impl(
            raw_quote,
            collateral.into(),
            now_secs,
            &self.root_ca_der,
            &backend,
            self.allow_service_td,
            self.allow_debug,
            #[cfg(feature = "danger-allow-tcb-override")]
            None::<fn(TcbInfo) -> TcbInfo>,
        )
    }

    /// Verify with the one-shot API using this verifier's [`Config`].
    pub fn verify(
        &self,
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        now_secs: u64,
    ) -> Result<VerifiedReport> {
        self.verify_result(raw_quote, collateral, now_secs)
            .map(QuoteVerificationResult::into_report_unchecked)
    }

    /// Verify a quote with the configured root certificate, passing a TCB info override.
    ///
    /// The override function receives `TcbInfo` after signature verification and can
    /// modify it before TCB level matching. Use with extreme caution.
    #[cfg(all(feature = "danger-allow-tcb-override", feature = "default-x509"))]
    pub fn dangerous_verify_claims_with_tcb_override(
        &self,
        raw_quote: &[u8],
        collateral: impl Into<QuoteCollateralV3>,
        now_secs: u64,
        override_tcb_info: impl FnOnce(TcbInfo) -> TcbInfo,
    ) -> Result<QuoteClaims> {
        verify_impl(
            raw_quote,
            collateral.into(),
            now_secs,
            &self.root_ca_der,
            &backend_for::<C>(),
            self.allow_service_td,
            self.allow_debug,
            Some(override_tcb_info),
        )?
        .claims()
    }

    #[cfg(feature = "danger-allow-tcb-override")]
    fn dangerous_verify_result_with_tcb_override<F: FnOnce(TcbInfo) -> TcbInfo>(
        &self,
        raw_quote: &[u8],
        collateral: impl Into<QuoteCollateralV3>,
        now_secs: u64,
        override_tcb_info: F,
    ) -> Result<QuoteVerificationResult> {
        let backend = backend_for::<C>();
        verify_impl(
            raw_quote,
            collateral.into(),
            now_secs,
            &self.root_ca_der,
            &backend,
            self.allow_service_td,
            self.allow_debug,
            Some(override_tcb_info),
        )
    }

    #[cfg(all(feature = "danger-allow-tcb-override", feature = "default-x509"))]
    pub fn dangerous_verify_with_tcb_override(
        &self,
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        now_secs: u64,
        override_tcb_info: impl FnOnce(TcbInfo) -> TcbInfo,
    ) -> Result<VerifiedReport> {
        self.dangerous_verify_result_with_tcb_override(
            raw_quote,
            collateral,
            now_secs,
            override_tcb_info,
        )
        .map(QuoteVerificationResult::into_report_unchecked)
    }
}

/// Backwards-compatible one-shot verification using [`DefaultConfig`].
#[cfg(feature = "default-x509")]
pub fn verify(
    raw_quote: &[u8],
    collateral: &QuoteCollateralV3,
    now_secs: u64,
) -> Result<VerifiedReport> {
    QuoteVerifier::<crate::configs::DefaultConfig>::new_prod()
        .verify(raw_quote, collateral, now_secs)
}

#[cfg(all(feature = "default-x509", feature = "danger-allow-tcb-override"))]
pub fn dangerous_verify_with_tcb_override(
    raw_quote: &[u8],
    collateral: &QuoteCollateralV3,
    now_secs: u64,
    override_tcb_info: impl FnOnce(TcbInfo) -> TcbInfo,
) -> Result<VerifiedReport> {
    QuoteVerifier::<crate::configs::DefaultConfig>::new_prod().dangerous_verify_with_tcb_override(
        raw_quote,
        collateral,
        now_secs,
        override_tcb_info,
    )
}

/// Verification policy builder for JS/WASM.
///
/// ```js
/// const policy = new QuotePolicy(now)
///     .allow_status("OutOfDate")
///     .platform_grace_period(7n * 86400n)
///     .allow_smt(true);
/// ```
#[cfg(feature = "js")]
#[wasm_bindgen(js_name = "QuotePolicy")]
pub struct JsQuotePolicy {
    inner: crate::policy::QuotePolicy,
}

#[cfg(feature = "js")]
fn js_parse_tcb_status(s: &str) -> Result<TcbStatus, JsValue> {
    match s {
        "UpToDate" => Ok(TcbStatus::UpToDate),
        "SWHardeningNeeded" => Ok(TcbStatus::SWHardeningNeeded),
        "ConfigurationNeeded" => Ok(TcbStatus::ConfigurationNeeded),
        "ConfigurationAndSWHardeningNeeded" => Ok(TcbStatus::ConfigurationAndSWHardeningNeeded),
        "OutOfDate" => Ok(TcbStatus::OutOfDate),
        "OutOfDateConfigurationNeeded" => Ok(TcbStatus::OutOfDateConfigurationNeeded),
        "Revoked" => Ok(TcbStatus::Revoked),
        _ => Err(JsValue::from_str(&alloc::format!(
            "Unknown TCB status: {s}"
        ))),
    }
}

#[cfg(feature = "js")]
#[wasm_bindgen(js_class = "QuotePolicy")]
impl JsQuotePolicy {
    /// Create a strict policy: only `UpToDate`, no grace period, no advisory blacklist.
    #[wasm_bindgen(constructor)]
    pub fn strict(now_secs: u64) -> Self {
        Self {
            inner: crate::policy::QuotePolicy::strict(now_secs),
        }
    }

    /// Create a pass-through policy for downstream appraisal.
    #[wasm_bindgen(js_name = "claimsOnly")]
    pub fn claims_only(now_secs: u64) -> Self {
        Self {
            inner: crate::policy::QuotePolicy::claims_only(now_secs),
        }
    }

    /// Allow an additional TCB status (e.g. "OutOfDate", "SWHardeningNeeded").
    pub fn allow_status(self, status: &str) -> Result<JsQuotePolicy, JsValue> {
        let s = js_parse_tcb_status(status)?;
        Ok(Self {
            inner: self.inner.allow_status(s),
        })
    }

    /// Reject a specific advisory ID (e.g. "INTEL-SA-00334").
    pub fn reject_advisory(self, id: &str) -> Self {
        Self {
            inner: self.inner.reject_advisory(id),
        }
    }

    /// Reject multiple advisory IDs at once.
    pub fn reject_advisories(self, ids: Vec<String>) -> Self {
        Self {
            inner: self.inner.reject_advisories(&ids),
        }
    }

    /// Set platform grace period in seconds.
    pub fn platform_grace_period(self, secs: u64) -> Self {
        Self {
            inner: self.inner.platform_grace_period(Duration::from_secs(secs)),
        }
    }

    /// Set QE grace period in seconds.
    pub fn qe_grace_period(self, secs: u64) -> Self {
        Self {
            inner: self.inner.qe_grace_period(Duration::from_secs(secs)),
        }
    }

    /// Set minimum TCB evaluation data number.
    pub fn min_tcb_eval_data_number(self, min: u32) -> Self {
        Self {
            inner: self.inner.min_tcb_eval_data_number(min),
        }
    }

    /// Set whether dynamic platforms are allowed.
    pub fn allow_dynamic_platform(self, allow: bool) -> Self {
        Self {
            inner: self.inner.allow_dynamic_platform(allow),
        }
    }

    /// Set whether cached keys are allowed.
    pub fn allow_cached_keys(self, allow: bool) -> Self {
        Self {
            inner: self.inner.allow_cached_keys(allow),
        }
    }

    /// Set whether SMT (hyperthreading) is allowed.
    pub fn allow_smt(self, allow: bool) -> Self {
        Self {
            inner: self.inner.allow_smt(allow),
        }
    }

    /// Set accepted SGX types (e.g. [0, 1, 2]).
    pub fn accepted_sgx_types(self, types: Vec<u8>) -> Self {
        Self {
            inner: self.inner.accepted_sgx_types(&types),
        }
    }
}

/// Quote verifier for JS/WASM.
///
/// ```js
/// const verifier = new QuoteVerifier();          // Intel production root CA
/// const verifier = new QuoteVerifier(rootCaDer);  // custom root CA
/// const result = verifier.verify(quote, collateral, now);
/// ```
#[cfg(feature = "js")]
#[wasm_bindgen(js_name = "QuoteVerifier")]
pub struct JsQuoteVerifier {
    inner: QuoteVerifier,
}

#[cfg(feature = "js")]
#[wasm_bindgen(js_class = "QuoteVerifier")]
impl JsQuoteVerifier {
    /// Create a verifier. No argument = Intel production root CA; pass `rootCaDer` for custom.
    #[wasm_bindgen(constructor)]
    pub fn new(root_ca_der: Option<Vec<u8>>) -> Self {
        let inner = match root_ca_der {
            Some(der) => QuoteVerifier::new(der),
            None => QuoteVerifier::new_prod(),
        };
        Self { inner }
    }

    /// Backwards-compatible one-shot verification returning `VerifiedReport`.
    pub fn verify(
        &self,
        raw_quote: JsValue,
        quote_collateral: JsValue,
        now: u64,
    ) -> Result<JsValue, JsValue> {
        let raw_quote: Vec<u8> = serde_wasm_bindgen::from_value(raw_quote)
            .map_err(|_| JsValue::from_str("Failed to decode raw_quote"))?;
        let quote_collateral =
            serde_wasm_bindgen::from_value::<QuoteCollateralV3>(quote_collateral)?;
        let report = self
            .inner
            .verify(&raw_quote, &quote_collateral, now)
            .map_err(|e| JsValue::from_str(&format_error_chain(&e)))?;
        serde_wasm_bindgen::to_value(&report)
            .map_err(|_| JsValue::from_str("Failed to encode verified report"))
    }

    /// Verify the quote, apply the built-in policy, and return claims.
    pub fn verify_with_policy(
        &self,
        raw_quote: JsValue,
        quote_collateral: JsValue,
        now: u64,
        policy: &JsQuotePolicy,
    ) -> Result<JsValue, JsValue> {
        let raw_quote: Vec<u8> = serde_wasm_bindgen::from_value(raw_quote)
            .map_err(|_| JsValue::from_str("Failed to decode raw_quote"))?;
        let quote_collateral =
            serde_wasm_bindgen::from_value::<QuoteCollateralV3>(quote_collateral)?;
        let claims = self
            .inner
            .verify_with_policy(&raw_quote, quote_collateral, now, &policy.inner)
            .map_err(|e| JsValue::from_str(&format_error_chain(&e)))?;
        serde_wasm_bindgen::to_value(&claims)
            .map_err(|_| JsValue::from_str("Failed to encode quote claims"))
    }

    /// Fetch collateral from a PCCS server.
    pub async fn get_collateral(pccs_url: &str, raw_quote: JsValue) -> Result<JsValue, JsValue> {
        let raw_quote: Vec<u8> = serde_wasm_bindgen::from_value(raw_quote)
            .map_err(|_| JsValue::from_str("Failed to decode raw_quote"))?;

        let collateral: QuoteCollateralV3 =
            crate::collateral::CollateralClient::with_default_http(pccs_url)
                .map_err(|e| JsValue::from_str(&format_error_chain(&e)))?
                .fetch(&raw_quote)
                .await
                .map_err(|e| JsValue::from_str(&format_error_chain(&e)))?;
        serde_wasm_bindgen::to_value(&collateral)
            .map_err(|_| JsValue::from_str("Failed to encode collateral"))
    }
}

// =============================================================================
// Step 1: Verify TCB Info signature (Intel Root -> TCB Signing Cert -> TCB Info JSON)
// =============================================================================

/// Verify TCB Info collateral: certificate chain, signature, parsing, and expiration check
fn verify_tcb_info_signature(
    collateral: &QuoteCollateralV3,
    now: UnixTime,
    crls: &[webpki::CertRevocationList<'_>],
    trust_anchor: rustls_pki_types::TrustAnchor,
    backend: &CryptoBackend,
) -> Result<TcbInfo> {
    // Parse TCB Info
    let tcb_info = serde_json::from_str::<TcbInfo>(&collateral.tcb_info)
        .context("Failed to decode TcbInfo")?;

    // Check validity window
    let issue_date = parse_rfc3339_unix_secs(&tcb_info.issue_date)
        .context("Failed to parse TCB Info issue date")?;
    let next_update = parse_rfc3339_unix_secs(&tcb_info.next_update)
        .context("Failed to parse TCB Info next update")?;
    if now.as_secs() < issue_date {
        bail!("TCBInfo issue date is in the future");
    }
    if now.as_secs() > next_update {
        bail!("TCBInfo expired");
    }

    // Verify certificate chain
    let tcb_certs = extract_certs(collateral.tcb_info_issuer_chain.as_bytes())?;
    let [tcb_leaf, tcb_chain @ ..] = &tcb_certs[..] else {
        bail!("Certificate chain is too short for TCB Info");
    };
    let tcb_leaf_cert = webpki::EndEntityCert::try_from(tcb_leaf)
        .context("Failed to parse TCB Info leaf certificate")?;
    verify_certificate_chain(&tcb_leaf_cert, tcb_chain, now, crls, trust_anchor)?;

    // Verify signature
    let asn1_signature = (backend.encode_ecdsa)(&collateral.tcb_info_signature)?;
    if tcb_leaf_cert
        .verify_signature(
            backend.sig_algo,
            collateral.tcb_info.as_bytes(),
            &asn1_signature,
        )
        .is_err()
    {
        bail!("Signature is invalid for tcb_info in quote_collateral");
    }

    Ok(tcb_info)
}

// =============================================================================
// Step 2: Verify QE Identity signature (Intel Root -> QE Identity Signing Cert -> QE Identity JSON)
// =============================================================================

/// Verify QE Identity collateral: certificate chain, signature, parsing, and expiration check
fn verify_qe_identity_signature(
    collateral: &QuoteCollateralV3,
    now: UnixTime,
    crls: &[webpki::CertRevocationList<'_>],
    trust_anchor: rustls_pki_types::TrustAnchor,
    backend: &CryptoBackend,
) -> Result<QeIdentity> {
    // Parse QE Identity
    let qe_identity = serde_json::from_str::<QeIdentity>(&collateral.qe_identity)
        .context("Failed to decode QeIdentity")?;

    // Check validity window
    let issue_date = parse_rfc3339_unix_secs(&qe_identity.issue_date)
        .context("Failed to parse QE Identity issue date")?;
    let next_update = parse_rfc3339_unix_secs(&qe_identity.next_update)
        .context("Failed to parse QE Identity next update")?;
    if now.as_secs() < issue_date {
        bail!("QE Identity issue date is in the future");
    }
    if now.as_secs() > next_update {
        bail!("QE Identity expired");
    }

    // Verify certificate chain
    let qe_id_certs = extract_certs(collateral.qe_identity_issuer_chain.as_bytes())?;
    let [qe_id_leaf, qe_id_chain @ ..] = &qe_id_certs[..] else {
        bail!("Certificate chain is too short for QE Identity");
    };
    let qe_id_leaf_cert = webpki::EndEntityCert::try_from(qe_id_leaf)
        .context("Failed to parse QE Identity leaf certificate")?;
    verify_certificate_chain(&qe_id_leaf_cert, qe_id_chain, now, crls, trust_anchor)?;

    // Verify signature
    let qe_id_asn1_signature = (backend.encode_ecdsa)(&collateral.qe_identity_signature)?;
    if qe_id_leaf_cert
        .verify_signature(
            backend.sig_algo,
            collateral.qe_identity.as_bytes(),
            &qe_id_asn1_signature,
        )
        .is_err()
    {
        bail!("Signature is invalid for qe_identity in quote_collateral");
    }

    Ok(qe_identity)
}

// =============================================================================
// Step 3: Verify PCK certificate chain (Intel Root -> PCK CA -> PCK Cert)
// =============================================================================

/// Verify PCK certificate chain and extract platform data
///
/// Verifies the PCK certificate chain against the trusted root and CRLs.
/// Extracts cpu_svn, pce_svn, fmspc, and ppid from the certificate.
fn verify_pck_cert_chain(
    collateral: &QuoteCollateralV3,
    certification_data: &crate::quote::CertificationData,
    now: UnixTime,
    crls: &[webpki::CertRevocationList<'_>],
    trust_anchor: rustls_pki_types::TrustAnchor,
    backend: &CryptoBackend,
) -> Result<PckCertChainResult> {
    // Extract PCK certificate chain - prefer collateral, fall back to quote
    let certification_certs = if let Some(pem_chain) = &collateral.pck_certificate_chain {
        extract_certs(pem_chain.as_bytes())
            .context("Failed to extract PCK certificates from collateral")?
    } else {
        if certification_data.cert_type != PCK_CERT_CHAIN {
            bail!("Unsupported DCAP PCK cert format: {}. Use get_collateral() to fetch PCK certificate.", certification_data.cert_type);
        }
        extract_certs(&certification_data.body.data)
            .context("Failed to extract PCK certificates from quote")?
    };

    let [pck_leaf, pck_chain @ ..] = &certification_certs[..] else {
        bail!("Certificate chain is too short in quote");
    };

    // Verify PCK certificate chain
    let pck_leaf_cert =
        webpki::EndEntityCert::try_from(pck_leaf).context("Failed to parse PCK certificate")?;
    verify_certificate_chain(&pck_leaf_cert, pck_chain, now, crls, trust_anchor)?;

    // Extract Intel extension data from PCK cert (parsed once)
    let pck_ext = (backend.parse_pck_extension)(pck_leaf)?;

    // Preserve pce_id as the raw value from the PCK cert SGX extension.
    let pce_id = pck_ext.pce_id.clone();

    // Convert platform_instance_id to fixed-size array
    let platform_instance_id = pck_ext.platform_instance_id.as_ref().and_then(|v| {
        let arr: [u8; 16] = v.as_slice().try_into().ok()?;
        Some(arr)
    });

    Ok(PckCertChainResult {
        pck_cert_chain_der: certification_certs
            .iter()
            .map(|cert| cert.as_ref().to_vec())
            .collect(),
        pck_leaf_der: pck_leaf.as_ref().to_vec(),
        ppid: pck_ext.ppid,
        cpu_svn: pck_ext.cpu_svn,
        pce_svn: pck_ext.pce_svn,
        fmspc: pck_ext.fmspc,
        pce_id,
        sgx_type: pck_ext.sgx_type as u8,
        platform_instance_id,
        dynamic_platform: pck_ext.dynamic_platform.into(),
        cached_keys: pck_ext.cached_keys.into(),
        smt_enabled: pck_ext.smt_enabled.into(),
    })
}

/// Result from PCK certificate chain verification
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
struct PckCertChainResult {
    #[serde(with = "crate::utils::serde_vec_bytes")]
    pck_cert_chain_der: Vec<Vec<u8>>,
    #[serde(with = "serde_bytes")]
    pck_leaf_der: Vec<u8>,
    #[serde(with = "serde_bytes")]
    ppid: Vec<u8>,
    #[serde(with = "serde_bytes")]
    cpu_svn: [u8; 16],
    pce_svn: u16,
    #[serde(with = "serde_bytes")]
    fmspc: [u8; 6],
    #[serde(with = "serde_bytes")]
    pce_id: Vec<u8>,
    sgx_type: u8,
    platform_instance_id: Option<[u8; 16]>,
    dynamic_platform: PckCertFlag,
    cached_keys: PckCertFlag,
    smt_enabled: PckCertFlag,
}

// =============================================================================
// Step 4: Verify QE Report signature (PCK Cert signs QE Report)
// =============================================================================

/// Verify QE report signature using PCK certificate
fn verify_qe_report_signature(
    pck_leaf: &CertificateDer,
    auth_data: &crate::quote::AuthDataV3,
    backend: &CryptoBackend,
) -> Result<EnclaveReport> {
    let pck_leaf_cert =
        webpki::EndEntityCert::try_from(pck_leaf).context("Failed to parse PCK certificate")?;

    // Verify QE report signature (signed by PCK)
    let qe_report_signature = (backend.encode_ecdsa)(&auth_data.qe_report_signature)?;
    if pck_leaf_cert
        .verify_signature(backend.sig_algo, &auth_data.qe_report, &qe_report_signature)
        .is_err()
    {
        bail!("Signature is invalid for qe_report in quote");
    }

    // Decode QE report
    let mut qe_report_slice = auth_data.qe_report.as_slice();
    let qe_report =
        EnclaveReport::decode(&mut qe_report_slice).context("Failed to decode QE report")?;

    Ok(qe_report)
}

// =============================================================================
// Step 5: Verify QE Report content (QE Hash = hash(attestation_key + auth_data))
// =============================================================================

/// Verify QE report hash matches attestation key and auth data (panic-free)
fn verify_qe_report_data(
    qe_report: &EnclaveReport,
    auth_data: &crate::quote::AuthDataV3,
    backend: &CryptoBackend,
) -> Result<()> {
    use crate::constants::{ATTESTATION_KEY_LEN, AUTHENTICATION_DATA_LEN};

    ensure!(
        auth_data.qe_auth_data.data.len() == AUTHENTICATION_DATA_LEN,
        "Invalid QE auth data length"
    );
    // Build hash data: attestation_key || qe_auth_data
    let mut qe_hash_data = [0u8; ATTESTATION_KEY_LEN + AUTHENTICATION_DATA_LEN];
    qe_hash_data[..ATTESTATION_KEY_LEN].copy_from_slice(&auth_data.ecdsa_attestation_key);
    qe_hash_data[ATTESTATION_KEY_LEN..].copy_from_slice(&auth_data.qe_auth_data.data);
    let qe_hash = (backend.sha256)(&qe_hash_data);
    if qe_hash[..] != qe_report.report_data[..32] {
        bail!("QE report hash mismatch");
    }
    Ok(())
}

// =============================================================================
// Step 6: Verify QE Report policy (QE Report fields match QE Identity policy)
// =============================================================================

// verify_qe_identity_policy is defined below (after verify_impl)

// =============================================================================
// Step 7: Verify ISV Report signature (Attestation Key signs ISV Report)
// =============================================================================

/// Verify ISV enclave report signature using attestation key
fn verify_isv_report_signature(
    raw_quote: &[u8],
    quote: &Quote,
    auth_data: &crate::quote::AuthDataV3,
    backend: &CryptoBackend,
) -> Result<()> {
    // Prepend 0x04 to raw public key for SEC1 uncompressed format
    let mut pub_key = [0x04u8; 65];
    pub_key[1..].copy_from_slice(&auth_data.ecdsa_attestation_key);

    // DER-encode the raw r||s signature for SignatureVerificationAlgorithm
    let der_sig = (backend.encode_ecdsa)(&auth_data.ecdsa_signature)?;

    let signed_data = raw_quote
        .get(..quote.signed_length())
        .context("Failed to get signed quote scope")?;

    backend
        .sig_algo
        .verify_signature(&pub_key, signed_data, &der_sig)
        .map_err(|_| anyhow::anyhow!("ISV enclave report signature is invalid"))
}

// =============================================================================
// Step 8: Match Platform TCB (PCK Cert's CPU_SVN/PCE_SVN/FMSPC vs TCB Info)
// =============================================================================

/// Match platform TCB level and return the matched TcbLevel
fn match_platform_tcb(
    tcb_info: &TcbInfo,
    quote: &Quote,
    tee_type: TeeType,
    cpu_svn: &[u8],
    pce_svn: u16,
    fmspc: &[u8],
) -> Result<TcbLevel> {
    // Verify FMSPC matches
    let tcb_fmspc = hex::decode(&tcb_info.fmspc)
        .ok()
        .context("Failed to decode TCB FMSPC")?;
    if fmspc[..] != tcb_fmspc[..] {
        bail!("Fmspc mismatch");
    }

    // Verify TCB Info type matches quote TEE type
    match tee_type {
        TeeType::Tdx => {
            if tcb_info.version < 3 || tcb_info.id != "TDX" {
                bail!("TDX quote with non-TDX TCB info in the collateral");
            }
        }
        TeeType::Sgx => {
            if tcb_info.version < 2 || tcb_info.id != "SGX" {
                bail!("SGX quote with non-SGX TCB info in the collateral");
            }
        }
    }

    // Find matching TCB level
    for tcb_level in &tcb_info.tcb_levels {
        if pce_svn < tcb_level.tcb.pce_svn {
            continue;
        }

        let sgx_components: Vec<u8> = tcb_level.tcb.sgx_components.iter().map(|c| c.svn).collect();
        if sgx_components.len() != cpu_svn.len() {
            bail!(
                "SGX component count mismatch: expected {}, got {}",
                cpu_svn.len(),
                sgx_components.len()
            );
        }

        // Component-wise comparison: every cpu_svn[i] must be >= sgx_components[i]
        if cpu_svn.iter().zip(&sgx_components).any(|(a, b)| a < b) {
            continue;
        }

        // For TDX, also check TDX components
        if tee_type.is_tdx() {
            let td_report = quote
                .report
                .as_td10()
                .context("Failed to get TD10 report")?;
            let tdx_components: Vec<u8> =
                tcb_level.tcb.tdx_components.iter().map(|c| c.svn).collect();
            if tdx_components.len() != td_report.tee_tcb_svn.len() {
                bail!(
                    "TDX component count mismatch: expected {}, got {}",
                    td_report.tee_tcb_svn.len(),
                    tdx_components.len()
                );
            }
            // Component-wise comparison: every tee_tcb_svn[i] must be >= tdx_components[i]
            if td_report
                .tee_tcb_svn
                .iter()
                .zip(&tdx_components)
                .any(|(a, b)| a < b)
            {
                continue;
            }
        }

        let mut matched = tcb_level.clone();
        if tee_type.is_tdx() {
            if let Some(module_status) =
                match_tdx_module_identity(tcb_info, quote).context("TDX module identity check")?
            {
                matched.tcb_status = matched
                    .tcb_status
                    .converge_with_component(module_status.status);
                for advisory in module_status.advisory_ids {
                    if !matched.advisory_ids.contains(&advisory) {
                        matched.advisory_ids.push(advisory);
                    }
                }
            }
        }
        return Ok(matched);
    }

    bail!("No matching TCB level found");
}

fn match_tdx_module_identity(
    tcb_info: &TcbInfo,
    quote: &Quote,
) -> Result<Option<TcbStatusWithAdvisory>> {
    if tcb_info.id != "TDX" || tcb_info.version < 3 {
        return Ok(None);
    }

    let td_report = quote
        .report
        .as_td10()
        .context("Failed to get TD10 report for TDX module identity")?;

    let module_isvsvn = td_report.tee_tcb_svn[0];
    let module_version = td_report.tee_tcb_svn[1];

    let base_module = match &tcb_info.tdx_module {
        Some(m) => m,
        None => {
            bail!("TDX TCB Info is missing tdxModule field");
        }
    };

    // Helper to decode a hex string into a fixed-size array
    fn decode_hex_array<const N: usize>(hex_str: &str, field: &str) -> Result<[u8; N]> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| anyhow::anyhow!("Failed to decode {field} as hex: {e}"))?;
        ensure!(
            bytes.len() == N,
            "{field} has invalid length {}, expected {N}",
            bytes.len()
        );
        let mut arr = [0u8; N];
        arr.copy_from_slice(&bytes);
        Ok(arr)
    }

    // Start from the base module values
    let mut expected_mrsigner =
        decode_hex_array::<48>(&base_module.mrsigner, "tdxModule.mrsigner")?;
    let mut expected_attributes =
        decode_hex_array::<8>(&base_module.attributes, "tdxModule.attributes")?;
    let mut attributes_mask =
        decode_hex_array::<8>(&base_module.attributes_mask, "tdxModule.attributesMask")?;

    // If a specific module version is indicated and identities are present,
    // override expectations from the matching identity entry.
    let mut identity_tcb_levels: Option<&[TdxModuleTcbLevel]> = None;
    if module_version > 0 && !tcb_info.tdx_module_identities.is_empty() {
        let wanted_id = format!("TDX_{:02X}", module_version);
        let identity = tcb_info
            .tdx_module_identities
            .iter()
            .find(|id| id.id.eq_ignore_ascii_case(&wanted_id))
            .with_context(|| {
                format!(
                    "No TDX module identity with id {} found in TCB Info",
                    wanted_id
                )
            })?;

        expected_mrsigner =
            decode_hex_array::<48>(&identity.mrsigner, "tdxModuleIdentity.mrsigner")?;
        expected_attributes =
            decode_hex_array::<8>(&identity.attributes, "tdxModuleIdentity.attributes")?;
        attributes_mask = decode_hex_array::<8>(
            &identity.attributes_mask,
            "tdxModuleIdentity.attributesMask",
        )?;
        identity_tcb_levels = Some(&identity.tcb_levels);
    }

    // Verify MRSEAM signer (MR_SIGNER_SEAM) matches expected module MRSIGNER.
    if td_report.mr_signer_seam != expected_mrsigner {
        bail!(
            "TDX module MRSIGNER mismatch: expected {}, got {}",
            hex::encode_upper(expected_mrsigner),
            hex::encode_upper(td_report.mr_signer_seam)
        );
    }

    // Verify SEAMATTRIBUTES with mask: masked bits must match, and bits
    // outside the mask must be zero (defense in depth).
    for (i, ((expected, mask), actual)) in expected_attributes
        .iter()
        .zip(attributes_mask.iter())
        .zip(td_report.seam_attributes.iter())
        .enumerate()
    {
        let expected_masked = expected & mask;
        let actual_masked = actual & mask;
        if expected_masked != actual_masked {
            bail!(
                "TDX module SEAMATTRIBUTES mismatch at byte {}: expected {:02X} (masked), got {:02X} (masked)",
                i,
                expected_masked,
                actual_masked
            );
        }
        if actual & !mask != 0 {
            bail!(
                "TDX module SEAMATTRIBUTES has bits set outside mask at byte {}",
                i
            );
        }
    }

    // If we have module identity TCB levels, derive module status from them.
    if let Some(levels) = identity_tcb_levels {
        let mut matched: Option<&TdxModuleTcbLevel> = None;
        for level in levels {
            if module_isvsvn >= level.tcb.isvsvn {
                matched = Some(level);
                break;
            }
        }

        let module_level = matched.with_context(|| {
            format!(
                "TDX module ISVSVN {} is below minimum required from TDX module TCB levels",
                module_isvsvn
            )
        })?;

        return Ok(Some(TcbStatusWithAdvisory::new(
            module_level.tcb_status,
            module_level.advisory_ids.clone(),
        )));
    }

    // No identity-specific TCB levels: we've still corroborated module identity,
    // but there is no additional status/advisory to merge.
    Ok(None)
}

// =============================================================================
// Main verification flow following the trust chain
// =============================================================================

/// Cryptographic verification of a quote. Returns [`QuoteClaims`] without
/// applying any policy — the caller decides acceptance via [`QuoteClaims::validate()`].
///
/// Trust chain verification order:
/// 1. Verify TCB Info signature (Intel Root -> TCB Signing Cert -> TCB Info JSON)
/// 2. Verify QE Identity signature (Intel Root -> QE Identity Signing Cert -> QE Identity JSON)
/// 3. Verify PCK certificate chain (Intel Root -> PCK CA -> PCK Cert)
/// 4. Verify QE Report signature (PCK Cert signs QE Report)
/// 5. Verify QE Report content (QE Hash = hash(attestation_key + auth_data))
/// 6. Verify QE Report policy (QE Report fields match QE Identity policy)
/// 7. Verify ISV Report signature (Attestation Key signs ISV Report)
/// 8. Match Platform TCB (PCK Cert's CPU_SVN/PCE_SVN/FMSPC vs TCB Info)
/// 9. Match QE TCB (QE Report's ISVSVN vs QE Identity tcb_levels)
/// 10. Merge TCB statuses
#[allow(clippy::too_many_arguments)]
fn verify_impl(
    raw_quote: &[u8],
    collateral: QuoteCollateralV3,
    now_secs: u64,
    root_ca_der: &[u8],
    backend: &CryptoBackend,
    allow_service_td: bool,
    allow_debug: bool,
    #[cfg(feature = "danger-allow-tcb-override")] override_tcb_info: Option<
        impl FnOnce(TcbInfo) -> TcbInfo,
    >,
) -> Result<QuoteVerificationResult> {
    // Setup trust anchor and time
    let root_ca = CertificateDer::from_slice(root_ca_der);
    let trust_anchor =
        webpki::anchor_from_trusted_cert(&root_ca).context("Failed to load root ca")?;
    let now = UnixTime::since_unix_epoch(Duration::from_secs(now_secs));
    let raw_crls = [&collateral.root_ca_crl[..], &collateral.pck_crl];

    // Check root CA against CRL
    webpki::check_single_cert_crl(root_ca_der, &raw_crls, now)?;

    // Parse CRLs once for reuse across all certificate chain verifications
    let crls = parse_crls(&raw_crls)?;

    // Parse quote and validate header
    let mut quote_slice = raw_quote;
    let quote = Quote::decode(&mut quote_slice).context("Failed to decode quote")?;
    if !ALLOWED_QUOTE_VERSIONS.contains(&quote.header.version) {
        bail!("Unsupported DCAP quote version");
    }
    if quote.header.qe_vendor_id != INTEL_QE_VENDOR_ID {
        bail!("Unknown QE vendor ID");
    }
    let tee_type = TeeType::from_u32(quote.header.tee_type)?;
    match tee_type {
        TeeType::Sgx => {
            if quote.header.version != 3 {
                bail!("SGX TEE quote must have version 3");
            }
        }
        TeeType::Tdx => {
            if ![4, 5].contains(&quote.header.version) {
                bail!("TDX TEE quote must have version 4 or 5");
            }
        }
    }
    if quote.header.attestation_key_type != ATTESTATION_KEY_TYPE_ECDSA256_WITH_P256_CURVE {
        bail!("Unsupported DCAP attestation key type");
    }
    let auth_data = quote.auth_data.clone().into_v3();

    // Step 1: Verify TCB Info signature
    let mut tcb_info =
        verify_tcb_info_signature(&collateral, now, &crls, trust_anchor.clone(), backend)?;

    #[cfg(feature = "danger-allow-tcb-override")]
    if let Some(override_tcb_info) = override_tcb_info {
        tcb_info = override_tcb_info(tcb_info);
    }
    tcb_info.canonicalize_tcb_levels();

    // Step 2: Verify QE Identity signature
    let qe_identity =
        verify_qe_identity_signature(&collateral, now, &crls, trust_anchor.clone(), backend)?;
    let (expected_qe_id, allowed_qe_versions): (&str, &[u8]) = match tee_type {
        TeeType::Sgx => ("QE", &[2]),
        TeeType::Tdx => ("TD_QE", &[2, 3]),
    };
    if qe_identity.id != expected_qe_id || !allowed_qe_versions.contains(&qe_identity.version) {
        bail!(
            "Unsupported QE Identity id/version for the quote TEE type: {} version {} (expected {} version {:?})",
            qe_identity.id,
            qe_identity.version,
            expected_qe_id,
            allowed_qe_versions
        );
    }

    // Step 3: Verify PCK certificate chain
    let pck_result = verify_pck_cert_chain(
        &collateral,
        &auth_data.certification_data,
        now,
        &crls,
        trust_anchor,
        backend,
    )?;
    let pck_leaf = CertificateDer::from(pck_result.pck_leaf_der.as_slice());

    // Step 4: Verify QE Report signature
    let qe_report = verify_qe_report_signature(&pck_leaf, &auth_data, backend)?;

    // Step 5: Verify QE Report content (hash check)
    verify_qe_report_data(&qe_report, &auth_data, backend)?;

    // Step 6: Verify QE Report policy (returns matched QeTcbLevel)
    let qe_tcb_level = verify_qe_identity_policy(&qe_report, &qe_identity)?;

    // Step 7: Verify ISV Report signature
    verify_isv_report_signature(raw_quote, &quote, &auth_data, backend)?;

    // Step 8: Match Platform TCB (returns matched TcbLevel)
    let platform_tcb_level = match_platform_tcb(
        &tcb_info,
        &quote,
        tee_type,
        &pck_result.cpu_svn,
        pck_result.pce_svn,
        &pck_result.fmspc,
    )?;

    // Step 9 & 10: Merge statuses (take worst)
    let platform_status = TcbStatusWithAdvisory::new(
        platform_tcb_level.tcb_status,
        platform_tcb_level.advisory_ids.clone(),
    );
    let qe_status =
        TcbStatusWithAdvisory::new(qe_tcb_level.tcb_status, qe_tcb_level.advisory_ids.clone());
    let final_status = platform_status.merge(&qe_status);

    // Revoked means the platform's keys are compromised — reject unconditionally,
    // regardless of policy. This is a security invariant, not a policy decision.
    if final_status.status == TcbStatus::Revoked {
        bail!("TCB status is invalid: Revoked");
    }

    #[cfg(feature = "default-x509")]
    let root_key_id = {
        let root_cert: x509_cert::Certificate =
            der::Decode::from_der(root_ca_der).context("Failed to parse root CA certificate")?;
        let raw_key = root_cert
            .tbs_certificate()
            .subject_public_key_info()
            .subject_public_key
            .raw_bytes();
        (backend.sha384)(raw_key)
    };
    #[cfg(not(feature = "default-x509"))]
    let root_key_id = [0u8; 48];

    // Validate report attributes (debug mode check, etc.)
    validate_attrs(&quote.report, allow_service_td, allow_debug)?;

    Ok(QuoteVerificationResult {
        header: quote.header,
        report: quote.report,
        collateral,
        pck_cert_chain_der: pck_result.pck_cert_chain_der.clone(),
        tee_type: quote.header.tee_type,
        tcb_status: final_status.status,
        advisory_ids: final_status.advisory_ids,
        platform_tcb_level,
        qe_tcb_level,
        pck_ext: pck_result,
        qe_report,
        tcb_eval_data_number: tcb_info
            .tcb_evaluation_data_number
            .min(qe_identity.tcb_evaluation_data_number),
        qe_tcb_eval_data_number: qe_identity.tcb_evaluation_data_number,
        root_key_id,
    })
}

/// Collateral time window dates (8 sources + QE Identity subset).
#[cfg(feature = "default-x509")]
struct CollateralDates {
    earliest_issue: u64,
    latest_issue: u64,
    earliest_expiration: u64,
    /// QE Identity-specific dates (sources \[5\] + \[7\] only).
    qe_iden_earliest_issue: u64,
    qe_iden_latest_issue: u64,
    qe_iden_earliest_expiration: u64,
}

/// Compute the collateral time window: earliest issue, latest issue, earliest expiration.
///
/// Matches Intel QVL's `qve_get_collateral_dates()` which considers **8 date sources**:
///
/// 1. Root CA CRL thisUpdate/nextUpdate
/// 2. PCK CRL thisUpdate/nextUpdate
/// 3. PCK CRL issuer certificate chain notBefore/notAfter
/// 4. PCK certificate chain notBefore/notAfter
/// 5. TCBInfo issuer certificate chain notBefore/notAfter
/// 6. QEIdentity issuer certificate chain notBefore/notAfter
/// 7. TCBInfo JSON issueDate/nextUpdate
/// 8. QEIdentity JSON issueDate/nextUpdate
#[cfg(feature = "default-x509")]
fn compute_collateral_time_window(
    collateral: &QuoteCollateralV3,
    pck_cert_chain: &[CertificateDer<'_>],
    tcb_info: &TcbInfo,
    qe_identity: &QeIdentity,
) -> Result<CollateralDates> {
    fn parse_crl_dates(crl_der: &[u8]) -> Result<(u64, Option<u64>)> {
        use der::Decode as _;
        let crl: x509_cert::crl::CertificateList<x509_cert::certificate::Rfc5280> =
            x509_cert::crl::CertificateList::from_der(crl_der)
                .context("Failed to parse CRL for time window")?;
        let this_update = crl.tbs_cert_list.this_update.to_unix_duration().as_secs();
        let next_update = crl
            .tbs_cert_list
            .next_update
            .map(|t| t.to_unix_duration().as_secs());
        Ok((this_update, next_update))
    }

    /// Extract notBefore/notAfter from a PEM certificate chain and fold into min/max accumulators.
    fn fold_cert_chain_dates(
        pem_chain: &[u8],
        earliest_issue: &mut u64,
        latest_issue: &mut u64,
        earliest_expiration: &mut u64,
    ) -> Result<()> {
        let certs = extract_certs(pem_chain)?;
        fold_der_cert_dates(&certs, earliest_issue, latest_issue, earliest_expiration)
    }

    fn fold_der_cert_dates(
        certs: &[CertificateDer<'_>],
        earliest_issue: &mut u64,
        latest_issue: &mut u64,
        earliest_expiration: &mut u64,
    ) -> Result<()> {
        use der::Decode as _;
        for cert_der in certs {
            let cert = x509_cert::Certificate::from_der(cert_der)
                .context("Failed to parse certificate for time window")?;
            let not_before = cert
                .tbs_certificate()
                .validity()
                .not_before
                .to_unix_duration()
                .as_secs();
            let not_after = cert
                .tbs_certificate()
                .validity()
                .not_after
                .to_unix_duration()
                .as_secs();
            *earliest_issue = (*earliest_issue).min(not_before);
            *latest_issue = (*latest_issue).max(not_before);
            *earliest_expiration = (*earliest_expiration).min(not_after);
        }
        Ok(())
    }

    // TCBInfo dates (already parsed upstream)
    let tcb_issue = parse_rfc3339_unix_secs(&tcb_info.issue_date).context("TCBInfo issueDate")?;
    let tcb_next = parse_rfc3339_unix_secs(&tcb_info.next_update).context("TCBInfo nextUpdate")?;

    // QEIdentity dates (already parsed upstream)
    let qe_issue =
        parse_rfc3339_unix_secs(&qe_identity.issue_date).context("QEIdentity issueDate")?;
    let qe_next =
        parse_rfc3339_unix_secs(&qe_identity.next_update).context("QEIdentity nextUpdate")?;

    let mut earliest_issue = tcb_issue.min(qe_issue);
    let mut latest_issue = tcb_issue.max(qe_issue);
    let mut earliest_expiration = tcb_next.min(qe_next);

    // Include CRL dates (sources 1 & 2)
    for crl_der in [&collateral.root_ca_crl[..], &collateral.pck_crl[..]] {
        let (this_update, next_update) = parse_crl_dates(crl_der)?;
        earliest_issue = earliest_issue.min(this_update);
        latest_issue = latest_issue.max(this_update);
        if let Some(next) = next_update {
            earliest_expiration = earliest_expiration.min(next);
        }
    }

    // Include certificate chain dates (sources 3-6)
    // PCK CRL issuer chain (same PEM as pck_crl_issuer_chain)
    fold_cert_chain_dates(
        collateral.pck_crl_issuer_chain.as_bytes(),
        &mut earliest_issue,
        &mut latest_issue,
        &mut earliest_expiration,
    )?;
    // PCK certificate chain
    fold_der_cert_dates(
        pck_cert_chain,
        &mut earliest_issue,
        &mut latest_issue,
        &mut earliest_expiration,
    )?;
    // TCBInfo issuer chain
    fold_cert_chain_dates(
        collateral.tcb_info_issuer_chain.as_bytes(),
        &mut earliest_issue,
        &mut latest_issue,
        &mut earliest_expiration,
    )?;
    // QEIdentity issuer chain (source [5]) — also track QE-specific dates
    let mut qe_chain_earliest_issue = u64::MAX;
    let mut qe_chain_latest_issue = 0u64;
    let mut qe_chain_earliest_expiration = u64::MAX;
    fold_cert_chain_dates(
        collateral.qe_identity_issuer_chain.as_bytes(),
        &mut qe_chain_earliest_issue,
        &mut qe_chain_latest_issue,
        &mut qe_chain_earliest_expiration,
    )?;
    // Fold into global window
    earliest_issue = earliest_issue.min(qe_chain_earliest_issue);
    latest_issue = latest_issue.max(qe_chain_latest_issue);
    earliest_expiration = earliest_expiration.min(qe_chain_earliest_expiration);

    // QE Identity-specific window: min/max of source [5] (issuer chain) + source [7] (JSON)
    let qe_iden_earliest_issue = qe_chain_earliest_issue.min(qe_issue);
    let qe_iden_latest_issue = qe_chain_latest_issue.max(qe_issue);
    let qe_iden_earliest_expiration = qe_chain_earliest_expiration.min(qe_next);

    Ok(CollateralDates {
        earliest_issue,
        latest_issue,
        earliest_expiration,
        qe_iden_earliest_issue,
        qe_iden_latest_issue,
        qe_iden_earliest_expiration,
    })
}

fn validate_sgx_attrs(report: &EnclaveReport, allow_debug: bool) -> Result<()> {
    let is_debug = report.attributes[0] & 0x02 != 0;
    if is_debug && !allow_debug {
        bail!("Debug mode is enabled");
    }
    Ok(())
}

fn validate_attrs(report: &Report, allow_service_td: bool, allow_debug: bool) -> Result<()> {
    fn validate_td10(report: &TDReport10, allow_debug: bool) -> Result<()> {
        let td_attrs =
            TDAttributes::parse(report.td_attributes).context("Failed to parse TD attributes")?;
        if td_attrs.tud & !0x01 != 0 {
            bail!("Reserved bits in TD attributes are set");
        }
        if td_attrs.tud & 0x01 != 0 && !allow_debug {
            bail!("Debug mode is enabled");
        }
        if td_attrs.sec.reserved_lower != 0
            || td_attrs.sec.reserved_bit29
            || td_attrs.other.reserved != 0
        {
            bail!("Reserved bits in TD attributes are set");
        }
        if !td_attrs.sec.sept_ve_disable {
            bail!("SEPT_VE_DISABLE is not enabled");
        }
        Ok(())
    }
    fn validate_td15(report: &TDReport15, allow_service_td: bool, allow_debug: bool) -> Result<()> {
        if !allow_service_td && report.mr_service_td != [0u8; 48] {
            bail!("Invalid MR service TD");
        }
        validate_td10(&report.base, allow_debug)
    }
    match &report {
        Report::TD15(report) => validate_td15(report, allow_service_td, allow_debug),
        Report::TD10(report) => validate_td10(report, allow_debug),
        Report::SgxEnclave(report) => validate_sgx_attrs(report, allow_debug),
    }
}

/// Ring crypto backend module.
///
/// Provides a pre-configured [`CryptoBackend`] using ring for ECDSA P-256 and SHA-256.
#[cfg(all(feature = "ring", feature = "default-x509"))]
pub mod ring {
    use super::*;

    fn ring_sha256(data: &[u8]) -> [u8; 32] {
        let digest = ::ring::digest::digest(&::ring::digest::SHA256, data);
        let mut out = [0u8; 32];
        out.copy_from_slice(digest.as_ref());
        out
    }

    fn ring_sha384(data: &[u8]) -> [u8; 48] {
        let digest = ::ring::digest::digest(&::ring::digest::SHA384, data);
        let mut out = [0u8; 48];
        out.copy_from_slice(digest.as_ref());
        out
    }

    /// Returns a [`CryptoBackend`] backed by ring.
    pub fn backend() -> CryptoBackend {
        CryptoBackend {
            sig_algo: webpki::ring::ECDSA_P256_SHA256,
            sha256: ring_sha256,
            sha384: ring_sha384,
            encode_ecdsa: encode_as_der_with::<crate::configs::RingConfig>,
            parse_pck_extension: crate::intel::parse_pck_extension_with::<crate::configs::RingConfig>,
        }
    }

    pub fn verify(
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        now_secs: u64,
    ) -> Result<VerifiedReport> {
        QuoteVerifier::new_prod()
            .with_config::<crate::configs::RingConfig>()
            .verify(raw_quote, collateral, now_secs)
    }

    #[cfg(feature = "danger-allow-tcb-override")]
    pub fn dangerous_verify_with_tcb_override(
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        now_secs: u64,
        override_tcb_info: impl FnOnce(TcbInfo) -> TcbInfo,
    ) -> Result<VerifiedReport> {
        QuoteVerifier::new_prod()
            .with_config::<crate::configs::RingConfig>()
            .dangerous_verify_with_tcb_override(raw_quote, collateral, now_secs, override_tcb_info)
    }
}

/// RustCrypto backend module.
///
/// Provides a pre-configured [`CryptoBackend`] using RustCrypto (sha2 + p256) for ECDSA P-256 and SHA-256.
#[cfg(all(feature = "rustcrypto", feature = "default-x509"))]
pub mod rustcrypto {
    use super::*;

    fn rustcrypto_sha256(data: &[u8]) -> [u8; 32] {
        use sha2::Digest;
        sha2::Sha256::digest(data).into()
    }

    fn rustcrypto_sha384(data: &[u8]) -> [u8; 48] {
        use sha2::Digest;
        sha2::Sha384::digest(data).into()
    }

    /// Returns a [`CryptoBackend`] backed by RustCrypto.
    pub fn backend() -> CryptoBackend {
        CryptoBackend {
            sig_algo: webpki::rustcrypto::ECDSA_P256_SHA256,
            sha256: rustcrypto_sha256,
            sha384: rustcrypto_sha384,
            encode_ecdsa: encode_as_der_with::<crate::configs::RustCryptoConfig>,
            parse_pck_extension: crate::intel::parse_pck_extension_with::<
                crate::configs::RustCryptoConfig,
            >,
        }
    }

    pub fn verify(
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        now_secs: u64,
    ) -> Result<VerifiedReport> {
        QuoteVerifier::new_prod()
            .with_config::<crate::configs::RustCryptoConfig>()
            .verify(raw_quote, collateral, now_secs)
    }

    #[cfg(feature = "danger-allow-tcb-override")]
    pub fn dangerous_verify_with_tcb_override(
        raw_quote: &[u8],
        collateral: &QuoteCollateralV3,
        now_secs: u64,
        override_tcb_info: impl FnOnce(TcbInfo) -> TcbInfo,
    ) -> Result<VerifiedReport> {
        QuoteVerifier::new_prod()
            .with_config::<crate::configs::RustCryptoConfig>()
            .dangerous_verify_with_tcb_override(raw_quote, collateral, now_secs, override_tcb_info)
    }
}

// =============================================================================
// Step 6 & 9: Verify QE Report policy and match QE TCB
// =============================================================================

/// Verify QE report fields against QE Identity policy constraints.
///
/// This enforces Intel's QE Identity policy by checking:
/// - MRSIGNER matches the expected value from QE Identity
/// - ISVPRODID matches the expected value
/// - MISCSELECT matches after applying the mask
/// - ATTRIBUTES match after applying the mask
/// - ISVSVN meets minimum requirement from QE Identity TCB levels (Step 9)
///
/// Returns the matched QeTcbLevel based on the QE's ISVSVN.
fn verify_qe_identity_policy(
    qe_report: &EnclaveReport,
    qe_identity: &QeIdentity,
) -> Result<QeTcbLevel> {
    // Verify MRSIGNER
    if qe_report.mr_signer != qe_identity.mrsigner {
        bail!(
            "QE MRSIGNER mismatch: expected {}, got {}",
            hex::encode_upper(qe_identity.mrsigner),
            hex::encode_upper(qe_report.mr_signer)
        );
    }

    validate_sgx_attrs(qe_report, false).context("QE report validation failed")?;

    // Verify ISVPRODID
    if qe_report.isv_prod_id != qe_identity.isvprodid {
        bail!(
            "QE ISVPRODID mismatch: expected {}, got {}",
            qe_identity.isvprodid,
            qe_report.isv_prod_id
        );
    }

    // Verify MISCSELECT with mask
    let expected_miscselect_u32 = u32::from_le_bytes(qe_identity.miscselect);
    let miscselect_mask_u32 = u32::from_le_bytes(qe_identity.miscselect_mask);
    let qe_miscselect_masked = qe_report.misc_select & miscselect_mask_u32;
    let expected_miscselect_masked = expected_miscselect_u32 & miscselect_mask_u32;

    if qe_miscselect_masked != expected_miscselect_masked {
        bail!(
            "QE MISCSELECT mismatch: expected {:08X} (masked), got {:08X} (masked)",
            expected_miscselect_masked,
            qe_miscselect_masked
        );
    }

    // Verify ATTRIBUTES with mask
    // Apply mask and compare byte-by-byte using iterators
    for (i, ((expected, mask), qe_attr)) in qe_identity
        .attributes
        .iter()
        .zip(qe_identity.attributes_mask.iter())
        .zip(qe_report.attributes.iter())
        .enumerate()
    {
        let expected_masked = expected & mask;
        let qe_masked = qe_attr & mask;
        if expected_masked != qe_masked {
            bail!(
                "QE ATTRIBUTES mismatch at byte {}: expected {:02X} (masked), got {:02X} (masked)",
                i,
                expected_masked,
                qe_masked
            );
        }
    }

    // Match QE TCB level based on ISVSVN
    match_qe_tcb_level(qe_report.isv_svn, &qe_identity.tcb_levels)
}

/// Match QE ISVSVN against QE Identity TCB levels
///
/// TCB levels are expected to be sorted from highest to lowest ISVSVN.
/// Returns the matched QeTcbLevel.
fn match_qe_tcb_level(
    isv_svn: u16,
    tcb_levels: &[crate::qe_identity::QeTcbLevel],
) -> Result<QeTcbLevel> {
    for tcb_level in tcb_levels {
        if isv_svn >= tcb_level.tcb.isvsvn {
            return Ok(tcb_level.clone());
        }
    }

    match tcb_levels.last().map(|l| l.tcb.isvsvn) {
        Some(min_required) => {
            bail!("QE ISVSVN {isv_svn} is below minimum required {min_required} from QE Identity");
        }
        None => {
            bail!("No TCB levels found in QE Identity");
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::tcb_info::TcbStatus::*;
    use hex_literal::hex;

    fn make_test_qe_report() -> EnclaveReport {
        EnclaveReport {
            cpu_svn: [0u8; 16],
            misc_select: 0x00000000,
            reserved1: [0u8; 28],
            attributes: [
                0x11, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
                0x00, 0x00,
            ],
            mr_enclave: [0u8; 32],
            reserved2: [0u8; 32],
            mr_signer: hex::decode(
                "8C4F5775D796503E96137F77C68A829A0056AC8DED70140B081B094490C57BFF",
            )
            .unwrap()
            .try_into()
            .unwrap(),
            reserved3: [0u8; 96],
            isv_prod_id: 1,
            isv_svn: 8,
            reserved4: [0u8; 60],
            report_data: [0u8; 64],
        }
    }

    fn make_test_qe_identity() -> QeIdentity {
        use crate::qe_identity::{QeTcb, QeTcbLevel};

        QeIdentity {
            id: "QE".to_string(),
            version: 2,
            issue_date: "2025-06-19T10:01:18Z".to_string(),
            next_update: "2025-07-19T10:01:18Z".to_string(),
            tcb_evaluation_data_number: 17,
            miscselect: hex!("00000000"),
            miscselect_mask: hex!("FFFFFFFF"),
            attributes: hex!("11000000000000000000000000000000"),
            attributes_mask: hex!("FBFFFFFFFFFFFFFF0000000000000000"),
            mrsigner: hex!("8C4F5775D796503E96137F77C68A829A0056AC8DED70140B081B094490C57BFF"),
            isvprodid: 1,
            tcb_levels: vec![
                QeTcbLevel {
                    tcb: QeTcb { isvsvn: 8 },
                    tcb_date: "2024-03-13T00:00:00Z".to_string(),
                    tcb_status: UpToDate,
                    advisory_ids: vec![],
                },
                QeTcbLevel {
                    tcb: QeTcb { isvsvn: 6 },
                    tcb_date: "2021-11-10T00:00:00Z".to_string(),
                    tcb_status: OutOfDate,
                    advisory_ids: vec!["INTEL-SA-00615".to_string()],
                },
                QeTcbLevel {
                    tcb: QeTcb { isvsvn: 5 },
                    tcb_date: "2020-11-11T00:00:00Z".to_string(),
                    tcb_status: OutOfDate,
                    advisory_ids: vec!["INTEL-SA-00477".to_string(), "INTEL-SA-00615".to_string()],
                },
            ],
        }
    }

    #[test]
    fn test_qe_identity_policy_valid() {
        let qe_report = make_test_qe_report();
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_ok(), "Expected success, got: {:?}", result);
    }

    #[test]
    fn test_qe_identity_policy_mrsigner_mismatch() {
        let qe_report = make_test_qe_report();
        let mut qe_identity = make_test_qe_identity();
        // Change expected MRSIGNER to something different
        qe_identity.mrsigner =
            hex!("0000000000000000000000000000000000000000000000000000000000000000");

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("MRSIGNER mismatch"),
            "Expected MRSIGNER mismatch error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_qe_identity_policy_isvprodid_mismatch() {
        let qe_report = make_test_qe_report();
        let mut qe_identity = make_test_qe_identity();
        qe_identity.isvprodid = 999; // Different product ID

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ISVPRODID mismatch"),
            "Expected ISVPRODID mismatch error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_qe_identity_policy_miscselect_mismatch() {
        let mut qe_report = make_test_qe_report();
        qe_report.misc_select = 0x00000001; // Set a bit
        let mut qe_identity = make_test_qe_identity();
        qe_identity.miscselect = hex!("00000000");
        qe_identity.miscselect_mask = hex!("FFFFFFFF"); // All bits checked

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("MISCSELECT mismatch"),
            "Expected MISCSELECT mismatch error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_qe_identity_policy_miscselect_masked() {
        let mut qe_report = make_test_qe_report();
        qe_report.misc_select = 0x000000FF; // Set some bits
        let mut qe_identity = make_test_qe_identity();
        qe_identity.miscselect = hex!("00000000");
        qe_identity.miscselect_mask = hex!("00000000"); // No bits checked (mask all zeros)

        // Should pass because mask is all zeros - no bits are checked
        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(
            result.is_ok(),
            "Expected success with zero mask, got: {:?}",
            result
        );
    }

    #[test]
    fn test_qe_identity_policy_attributes_mismatch() {
        let mut qe_report = make_test_qe_report();
        qe_report.attributes[0] = 0;
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ATTRIBUTES mismatch"),
            "Expected ATTRIBUTES mismatch error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_qe_identity_policy_attributes_masked() {
        let mut qe_report = make_test_qe_report();
        // Set bits in the second half (bytes 8-15) which are masked out
        qe_report.attributes[8] = 0xFF;
        qe_report.attributes[15] = 0xFF;
        let qe_identity = make_test_qe_identity();
        // Mask is "FBFFFFFFFFFFFFFF0000000000000000" - second half is all zeros

        // Should pass because those bytes are masked out
        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(
            result.is_ok(),
            "Expected success with masked attributes, got: {:?}",
            result
        );
    }

    #[test]
    fn test_qe_identity_policy_isvsvn_up_to_date() {
        let qe_report = make_test_qe_report(); // isv_svn = 8
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_ok());
        let tcb_level = result.unwrap();
        assert_eq!(tcb_level.tcb_status, UpToDate);
        assert!(tcb_level.advisory_ids.is_empty());
    }

    #[test]
    fn test_qe_identity_policy_isvsvn_out_of_date() {
        let mut qe_report = make_test_qe_report();
        qe_report.isv_svn = 6; // Lower than 8, matches second TCB level
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_ok());
        let tcb_level = result.unwrap();
        assert_eq!(tcb_level.tcb_status, OutOfDate);
        assert_eq!(tcb_level.advisory_ids, vec!["INTEL-SA-00615"]);
    }

    #[test]
    fn test_qe_identity_policy_isvsvn_higher_than_required() {
        let mut qe_report = make_test_qe_report();
        qe_report.isv_svn = 10; // Higher than highest TCB level (8)
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_ok());
        let tcb_level = result.unwrap();
        assert_eq!(tcb_level.tcb_status, UpToDate); // Matches first level (isvsvn >= 8)
    }

    #[test]
    fn test_qe_identity_policy_isvsvn_too_low() {
        let mut qe_report = make_test_qe_report();
        qe_report.isv_svn = 4; // Lower than all TCB levels (min is 5)
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("ISVSVN") && err_msg.contains("below minimum"),
            "Expected ISVSVN below minimum error, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_qe_identity_policy_isvsvn_between_levels() {
        let mut qe_report = make_test_qe_report();
        qe_report.isv_svn = 7; // Between level 8 and 6
        let qe_identity = make_test_qe_identity();

        let result = verify_qe_identity_policy(&qe_report, &qe_identity);
        assert!(result.is_ok());
        let tcb_level = result.unwrap();
        // Should match level with isvsvn=6 (7 >= 6)
        assert_eq!(tcb_level.tcb_status, OutOfDate);
        assert_eq!(tcb_level.advisory_ids, vec!["INTEL-SA-00615"]);
    }
}
