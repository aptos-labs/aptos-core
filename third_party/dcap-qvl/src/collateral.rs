use alloc::string::{String, ToString};
use anyhow::{anyhow, bail, Context, Result};
use core::marker::PhantomData;
use der::Decode as DerDecode;
use scale::Decode;
use serde::Deserialize;
use x509_cert::{
    ext::pkix::{
        name::{DistributionPointName, GeneralName},
        CrlDistributionPoints,
    },
    Certificate,
};

use crate::config::{Config, ParsedCert, X509Codec};
use crate::configs::DefaultConfig;
use crate::constants::{
    PCK_ID_ENCRYPTED_PPID_2048, PCK_ID_ENCRYPTED_PPID_3072, PCK_ID_PCK_CERT_CHAIN,
};
use crate::http::{HttpClient, HttpResponse, ReqwestHttp};
use crate::quote::{EncryptedPpidParams, Quote};
use crate::QuoteCollateralV3;

#[derive(Deserialize)]
struct TcbInfoResponse {
    #[serde(rename = "tcbInfo")]
    tcb_info: serde_json::Value,
    signature: String,
}

#[derive(Deserialize)]
struct QeIdentityResponse {
    #[serde(rename = "enclaveIdentity")]
    enclave_identity: serde_json::Value,
    signature: String,
}

#[cfg(not(feature = "js"))]
use core::time::Duration;

/// Default PCCS URL (Phala Network's PCCS server).
/// This is the recommended default for most users as it provides better availability
/// and lower rate limits compared to Intel's PCS.
pub const PHALA_PCCS_URL: &str = "https://pccs.phala.network";

/// Intel's official PCS (Provisioning Certification Service) URL.
/// Pass this to [`CollateralClient::with_default_http`] to fetch directly from Intel.
pub const INTEL_PCS_URL: &str = "https://api.trustedservices.intel.com";

struct PcsEndpoints {
    base_url: String,
    tee: &'static str,
    fmspc: String,
    ca: String,
}

impl PcsEndpoints {
    fn new(base_url: &str, for_sgx: bool, fmspc: String, ca: &str) -> Self {
        let tee = if for_sgx { "sgx" } else { "tdx" };
        let base_url = base_url
            .trim_end_matches('/')
            .trim_end_matches("/sgx/certification/v4")
            .trim_end_matches("/tdx/certification/v4")
            .to_owned();
        Self {
            base_url,
            tee,
            fmspc,
            ca: ca.to_owned(),
        }
    }

    fn is_pcs(&self) -> bool {
        self.base_url.starts_with(INTEL_PCS_URL)
    }

    fn url_pckcrl(&self) -> String {
        self.mk_url("sgx", &format!("pckcrl?ca={}&encoding=der", self.ca))
    }

    fn url_rootcacrl(&self) -> String {
        self.mk_url("sgx", "rootcacrl")
    }

    fn url_tcb(&self) -> String {
        self.mk_url(self.tee, &format!("tcb?fmspc={}", self.fmspc))
    }

    fn url_qe_identity(&self) -> String {
        self.mk_url(self.tee, "qe/identity?update=standard")
    }

    fn mk_url(&self, tee: &str, path: &str) -> String {
        format!("{}/{}/certification/v4/{}", self.base_url, tee, path)
    }
}

fn get_header(response: &HttpResponse, name: &str) -> Result<String> {
    let value = response
        .header(name)
        .ok_or_else(|| anyhow!("Missing {name}"))?;
    let value = urlencoding::decode(value)?;
    Ok(value.into_owned())
}

/// Extracts the CRL Distribution Point URL from a certificate.
///
/// This function parses the certificate and looks for the CRL Distribution Points extension (OID 2.5.29.31).
/// It then extracts the first URL found in the extension's FullName field.
///
/// # Arguments
/// * `cert_der` - The DER-encoded certificate bytes
///
/// # Returns
/// * `Ok(Some(String))` - The CRL distribution point URL if found
/// * `Ok(None)` - If no CRL distribution point was found in the certificate
/// * `Err(_)` - If there was an error parsing the certificate or the extension
fn extract_crl_url(cert_der: &[u8]) -> Result<Option<String>> {
    let cert: Certificate = DerDecode::from_der(cert_der).context("Failed to parse certificate")?;

    let Some(extensions) = &cert.tbs_certificate().extensions() else {
        return Ok(None);
    };
    for ext in extensions.iter() {
        if ext.extn_id.to_string() != "2.5.29.31" {
            continue;
        }
        let crl_dist_points: CrlDistributionPoints = DerDecode::from_der(ext.extn_value.as_bytes())
            .context("Failed to parse CRL Distribution Points")?;

        for dist_point in crl_dist_points.0.iter() {
            let Some(dist_point_name) = &dist_point.distribution_point else {
                continue;
            };
            let DistributionPointName::FullName(general_names) = dist_point_name else {
                continue;
            };
            for general_name in general_names.iter() {
                let GeneralName::UniformResourceIdentifier(uri) = general_name else {
                    continue;
                };
                return Ok(Some(uri.to_string()));
            }
        }
    }
    Ok(None)
}

/// Fetch PCK certificate from PCCS using encrypted PPID parameters.
async fn fetch_pck_certificate<H: HttpClient>(
    client: &H,
    pccs_url: &str,
    qeid: &[u8],
    params: &EncryptedPpidParams,
) -> Result<String> {
    // PCCS normalizes parameters to uppercase, Intel PCS accepts both
    // Use uppercase for compatibility with both
    let qeid = hex::encode_upper(qeid);
    let encrypted_ppid = hex::encode_upper(&params.encrypted_ppid);
    let cpusvn = hex::encode_upper(params.cpusvn);
    let pcesvn = hex::encode_upper(params.pcesvn.to_le_bytes());
    let pceid = hex::encode_upper(params.pceid);

    let base_url = pccs_url
        .trim_end_matches('/')
        .trim_end_matches("/sgx/certification/v4")
        .trim_end_matches("/tdx/certification/v4");
    let url = format!(
        "{base_url}/sgx/certification/v4/pckcert?qeid={qeid}&encrypted_ppid={encrypted_ppid}&cpusvn={cpusvn}&pcesvn={pcesvn}&pceid={pceid}"
    );
    let response = client.get(&url).await?;

    if !response.is_success() {
        bail!(
            "Failed to fetch PCK certificate from {}: HTTP {}",
            url,
            response.status
        );
    }

    // Check if Intel returned a certificate for a different TCB level
    // SGX-TCBm header format: cpusvn (16 bytes) + pcesvn (2 bytes, little-endian)
    if let Some(tcbm_str) = response.header("SGX-TCBm") {
        let tcbm_bytes =
            hex::decode(tcbm_str).map_err(|e| anyhow!("SGX-TCBm header is not valid hex: {e}"))?;
        let (matched_cpusvn, matched_pcesvn) = <([u8; 16], u16)>::decode(&mut &tcbm_bytes[..])
            .context("SGX-TCBm header too short: expected 18 bytes")?;

        if matched_cpusvn != params.cpusvn || matched_pcesvn != params.pcesvn {
            bail!(
                "TCB level mismatch: Platform's current TCB (cpusvn={}, pcesvn={}) \
                is not registered with Intel PCS. Intel matched to a lower TCB level \
                (cpusvn={}, pcesvn={}). This typically means the platform had a \
                microcode/firmware update but MPA registration was not re-run afterward. \
                Solution: Run 'mpa_manage -c mpa_registration.conf' on the platform \
                to register the new TCB level with Intel.",
                hex::encode(params.cpusvn),
                params.pcesvn,
                hex::encode(matched_cpusvn),
                matched_pcesvn
            );
        }
    }

    // The response includes the PCK certificate chain in a header
    let pck_cert_chain = get_header(&response, "SGX-PCK-Certificate-Issuer-Chain")?;

    // The body is the leaf PCK certificate
    let pck_cert = String::from_utf8(response.body)
        .context("PCK certificate response body is not valid UTF-8")?;

    // Combine into a full PEM chain (leaf first, then issuer chain)
    Ok(format!("{pck_cert}\n{pck_cert_chain}"))
}

/// Extract FMSPC and CA type from a PEM certificate chain.
///
/// Generic over [`Config`]; the cert parse and issuer DN extraction go
/// through the configured [`X509Codec`].
fn extract_fmspc_and_ca_with<C: Config>(pem_chain: &str) -> Result<(String, &'static str)> {
    let certs = crate::utils::extract_certs(pem_chain.as_bytes())
        .context("Failed to extract certificates from PEM chain")?;
    let cert = certs
        .first()
        .ok_or_else(|| anyhow!("Empty certificate chain"))?;

    // Parse cert once; reuse for both extension lookup and issuer DN.
    let parsed = <C as Config>::X509::from_der(cert).context("Failed to decode certificate")?;

    // Extract FMSPC from Intel extension
    let extension = parsed
        .extension(crate::oids::SGX_EXTENSION.as_bytes())
        .context("Failed to get Intel extension from certificate")?
        .ok_or_else(|| anyhow!("Intel extension not found"))?;
    let fmspc = crate::utils::get_fmspc(&extension)?;
    let fmspc_hex = hex::encode_upper(fmspc);

    // Extract CA type from issuer (reuses parsed cert above). Legacy
    // fallback: an unrecognised issuer is treated as Processor.
    let ca = parsed
        .pck_ca()
        .unwrap_or(crate::config::PckCa::Processor)
        .as_id_str();

    Ok((fmspc_hex, ca))
}

/// Build a `reqwest::Client` with this crate's default settings (180s
/// timeout on non-`js` targets). Crate-private on purpose: the
/// `reqwest::Client` type is intentionally kept off the public API so
/// that a future `reqwest` major bump is an internal change. Callers
/// who need a custom HTTP stack implement [`HttpClient`] on their own
/// type and pass it to [`CollateralClient::new`].
pub(crate) fn default_http_client() -> Result<reqwest::Client> {
    let builder = reqwest::Client::builder();
    #[cfg(not(feature = "js"))]
    let builder = builder.timeout(Duration::from_secs(180));
    Ok(builder.build()?)
}

/// Get PCK certificate chain for a quote.
/// - cert_type 5: extracts from quote
/// - cert_type 2/3: fetches from PCCS using encrypted PPID
async fn get_pck_chain<H: HttpClient>(client: &H, pccs_url: &str, quote: &Quote) -> Result<String> {
    match quote.inner_cert_type() {
        PCK_ID_PCK_CERT_CHAIN => Ok(String::from_utf8_lossy(quote.inner_cert_data()).to_string()),
        PCK_ID_ENCRYPTED_PPID_2048 | PCK_ID_ENCRYPTED_PPID_3072 => {
            let params = quote.encrypted_ppid_params()?;
            fetch_pck_certificate(client, pccs_url, quote.qeid(), &params).await
        }
        other => bail!("Unsupported certification data type: {other}"),
    }
}

/// A PCCS / PCS client parameterized by [`Config`] for pluggable X.509 /
/// crypto backends, and by [`HttpClient`] for the HTTP transport.
///
/// `H` defaults to a crate-private `reqwest`-backed adapter (gated on
/// `feature = "reqwest"`). `reqwest::Client` deliberately does **not**
/// appear in any public function signature, so a future `reqwest`
/// major bump is an internal change rather than a breaking one for
/// downstream callers. To plug in a custom HTTP stack — different TLS
/// config, workspace-pinned `reqwest` major, non-`reqwest` transport,
/// wasm host fetch, … — implement [`HttpClient`] on your own type and
/// hand it to [`new`](Self::new).
///
/// Exposes three fetch methods:
///
/// * [`fetch`](Self::fetch) — from a raw DCAP quote.
/// * [`fetch_for_fmspc`](Self::fetch_for_fmspc) — when the caller already
///   has the FMSPC / CA type (skips PCK chain extraction).
/// * [`fetch_and_verify`](Self::fetch_and_verify) — fetch collateral and
///   run [`QuoteVerifier::verify`](crate::verify::QuoteVerifier::verify) in one shot
///   (feature-gated on `_anycrypto`).
///
/// # Examples
///
/// Default path — let the crate build a `reqwest`-backed transport:
///
/// ```no_run
/// use dcap_qvl::collateral::{CollateralClient, PHALA_PCCS_URL};
/// # async fn run(quote: Vec<u8>) -> anyhow::Result<()> {
/// let collateral = CollateralClient::with_default_http(PHALA_PCCS_URL)?
///     .fetch(&quote)
///     .await?;
/// # Ok(()) }
/// ```
///
/// Custom transport — implement [`HttpClient`] on a type you own:
///
/// ```ignore
/// use dcap_qvl::collateral::CollateralClient;
/// use dcap_qvl::http::{HttpClient, HttpResponse};
///
/// struct MyHttp { /* your HTTP client of choice */ }
///
/// impl HttpClient for MyHttp {
///     async fn get(&self, url: &str) -> anyhow::Result<HttpResponse> {
///         /* … */
/// #       todo!()
///     }
/// }
///
/// let client = CollateralClient::<_, MyHttp>::new(MyHttp { /* … */ }, "https://pccs.local");
/// ```
pub struct CollateralClient<C: Config = DefaultConfig, H: HttpClient = ReqwestHttp> {
    http: H,
    pccs_url: String,
    _cfg: PhantomData<fn() -> C>,
}

impl<C: Config, H: HttpClient + Clone> Clone for CollateralClient<C, H> {
    fn clone(&self) -> Self {
        Self {
            http: self.http.clone(),
            pccs_url: self.pccs_url.clone(),
            _cfg: PhantomData,
        }
    }
}

impl<C: Config, H: HttpClient> CollateralClient<C, H> {
    /// Build a client with a caller-provided HTTP transport.
    ///
    /// Any type implementing [`HttpClient`] is accepted. For the common
    /// "just give me something that works" path see
    /// [`with_default_http`](CollateralClient::<DefaultConfig>::with_default_http);
    /// the default-path `reqwest::Client` is intentionally not exposed
    /// on the public API.
    pub fn new(http: H, pccs_url: impl Into<String>) -> Self {
        Self {
            http,
            pccs_url: pccs_url.into(),
            _cfg: PhantomData,
        }
    }

    /// Rebind the `Config` type without rebuilding the HTTP client / URL.
    pub fn with_config<D: Config>(self) -> CollateralClient<D, H> {
        CollateralClient {
            http: self.http,
            pccs_url: self.pccs_url,
            _cfg: PhantomData,
        }
    }

    /// Fetch collateral for the given raw DCAP quote.
    ///
    /// Decodes the quote, fetches (or extracts) the PCK certificate
    /// chain, reads FMSPC + CA type from the leaf cert via the
    /// configured [`X509Codec`], and then fetches the remaining
    /// collateral items. The returned [`QuoteCollateralV3`] has the PCK
    /// chain attached in `pck_certificate_chain` for offline
    /// verification.
    pub async fn fetch(&self, quote: &[u8]) -> Result<QuoteCollateralV3> {
        let mut quote = quote;
        let parsed = Quote::decode(&mut quote).context("Failed to parse quote")?;

        let pck_chain = get_pck_chain(&self.http, &self.pccs_url, &parsed)
            .await
            .context("Failed to get PCK certificate chain")?;

        let (fmspc, ca) = extract_fmspc_and_ca_with::<C>(&pck_chain)?;

        let mut collateral = self
            .fetch_for_fmspc_without_pck_chain(&fmspc, ca, parsed.header.is_sgx())
            .await?;

        collateral.pck_certificate_chain = Some(pck_chain);
        Ok(collateral)
    }

    /// Fetch the per-fmspc collateral bundle (PCK CRL, TCB info, QE
    /// identity, root CA CRL) when FMSPC and CA type are already known.
    ///
    /// > **NOTE:**
    /// >
    /// > - Deliberately returns a [`QuoteCollateralV3`] with
    /// >   `pck_certificate_chain = None`, so the output on its own is
    /// >   insufficient to verify a quote whose certification data
    /// >   doesn't embed the PCK chain.
    /// > - Use [`fetch`](Self::fetch) for verification purposes.
    pub async fn fetch_for_fmspc_without_pck_chain(
        &self,
        fmspc: &str,
        ca: &str,
        for_sgx: bool,
    ) -> Result<QuoteCollateralV3> {
        let endpoints = PcsEndpoints::new(&self.pccs_url, for_sgx, fmspc.to_owned(), ca);
        let client = &self.http;

        // Send a GET and fail with a useful message on non-2xx, so the
        // header / body readers below don't surface misleading errors
        // like "missing issuer-chain header" on an HTTP 500.
        async fn checked_get<H: HttpClient>(client: &H, url: &str) -> Result<HttpResponse> {
            let response = client.get(url).await?;
            if !response.is_success() {
                bail!("Failed to fetch {url}: HTTP {}", response.status);
            }
            Ok(response)
        }

        let pck_crl_issuer_chain;
        let pck_crl;
        {
            let response = checked_get(client, &endpoints.url_pckcrl()).await?;
            pck_crl_issuer_chain = get_header(&response, "SGX-PCK-CRL-Issuer-Chain")?;
            pck_crl = response.body;
        };

        let tcb_info_issuer_chain;
        let raw_tcb_info;
        {
            let response = checked_get(client, &endpoints.url_tcb()).await?;
            tcb_info_issuer_chain = get_header(&response, "SGX-TCB-Info-Issuer-Chain")
                .or(get_header(&response, "TCB-Info-Issuer-Chain"))?;
            raw_tcb_info = String::from_utf8(response.body)
                .context("TCB Info response body is not valid UTF-8")?;
        };
        let qe_identity_issuer_chain;
        let raw_qe_identity;
        {
            let response = checked_get(client, &endpoints.url_qe_identity()).await?;
            qe_identity_issuer_chain = get_header(&response, "SGX-Enclave-Identity-Issuer-Chain")?;
            raw_qe_identity = String::from_utf8(response.body)
                .context("QE Identity response body is not valid UTF-8")?;
        };

        async fn http_get<H: HttpClient>(client: &H, url: &str) -> Result<Vec<u8>> {
            Ok(checked_get(client, url).await?.body)
        }

        // First try to get root CA CRL directly from the PCCS endpoint
        let mut root_ca_crl = None;
        if !endpoints.is_pcs() {
            root_ca_crl = http_get(client, &endpoints.url_rootcacrl()).await.ok();

            if let Some(ref crl) = root_ca_crl {
                // PCCS returns hex-encoded CRL instead of binary DER.
                let hex_str = core::str::from_utf8(crl)
                    .context("Failed to convert hex-encoded CRL to string")?;
                let ca_crl = hex::decode(hex_str)
                    .map_err(|_| anyhow!("Failed to decode hex-encoded root CA CRL"))?;
                root_ca_crl = Some(ca_crl);
            }
        }
        let root_ca_crl = match root_ca_crl {
            Some(crl) => crl,
            None => {
                let certs = crate::utils::extract_certs(qe_identity_issuer_chain.as_bytes())
                    .context("Failed to extract certificates from QE identity issuer chain")?;
                let root_cert_der = certs
                    .last()
                    .context("No certificate found in QE identity issuer chain")?;
                let crl_url = extract_crl_url(root_cert_der)?;
                let Some(url) = crl_url else {
                    bail!("Could not find CRL distribution point in root certificate");
                };
                http_get(client, &url).await?
            }
        };

        let tcb_info_resp: TcbInfoResponse =
            serde_json::from_str(&raw_tcb_info).context("TCB Info should be valid JSON")?;
        let tcb_info = tcb_info_resp.tcb_info.to_string();
        let tcb_info_signature = hex::decode(&tcb_info_resp.signature)
            .ok()
            .context("TCB Info signature must be valid hex")?;

        let qe_identity_resp: QeIdentityResponse =
            serde_json::from_str(&raw_qe_identity).context("QE Identity should be valid JSON")?;
        let qe_identity = qe_identity_resp.enclave_identity.to_string();
        let qe_identity_signature = hex::decode(&qe_identity_resp.signature)
            .ok()
            .context("QE Identity signature must be valid hex")?;

        Ok(QuoteCollateralV3 {
            pck_crl_issuer_chain,
            root_ca_crl,
            pck_crl,
            tcb_info_issuer_chain,
            tcb_info,
            tcb_info_signature,
            qe_identity_issuer_chain,
            qe_identity,
            qe_identity_signature,
            pck_certificate_chain: None,
        })
    }

    /// Fetch collateral and run verification in one step, using the
    /// configured `Config`'s crypto and x509 backends.
    #[cfg(feature = "_anycrypto")]
    pub async fn fetch_and_verify(&self, quote: &[u8]) -> Result<crate::verify::VerifiedReport> {
        use std::time::SystemTime;

        let collateral = self.fetch(quote).await?;
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .context("Failed to get current time")?
            .as_secs();
        crate::verify::QuoteVerifier::new_prod()
            .with_config::<C>()
            .verify(quote, &collateral, now)
    }
}

impl CollateralClient<DefaultConfig, ReqwestHttp> {
    /// Convenience constructor: build a default `reqwest`-backed
    /// transport (180s timeout on non-`js` targets) and pair it with
    /// the given PCCS URL. Uses [`DefaultConfig`] (audited `x509-cert`
    /// backend). The underlying `reqwest::Client` is intentionally not
    /// exposed; callers who need a custom HTTP stack implement
    /// [`HttpClient`] on their own type and use
    /// [`CollateralClient::new`].
    pub fn with_default_http(pccs_url: impl Into<String>) -> Result<Self> {
        Ok(Self::new(
            ReqwestHttp::new(default_http_client()?),
            pccs_url,
        ))
    }

    /// Zero-arg convenience constructor: default HTTP client + PCCS URL
    /// from the `PCCS_URL` env var (trimmed; empty is treated as unset),
    /// falling back to [`PHALA_PCCS_URL`]. Uses [`DefaultConfig`].
    pub fn from_env() -> Result<Self> {
        let pccs_url = std::env::var("PCCS_URL")
            .ok()
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| PHALA_PCCS_URL.to_owned());
        Self::with_default_http(pccs_url)
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use super::*;
    use crate::constants::{PLATFORM_ISSUER_ID, PROCESSOR_ISSUER_ID};

    // Sample PCK certificate chain (processor CA) for testing - extracted from sample/sgx_quote
    const TEST_PCK_CHAIN_PROCESSOR: &str = r#"-----BEGIN CERTIFICATE-----
MIIEjTCCBDSgAwIBAgIVAIG3dzK3YemOubljpKvR5bm/XdjWMAoGCCqGSM49BAMC
MHExIzAhBgNVBAMMGkludGVsIFNHWCBQQ0sgUHJvY2Vzc29yIENBMRowGAYDVQQK
DBFJbnRlbCBDb3Jwb3JhdGlvbjEUMBIGA1UEBwwLU2FudGEgQ2xhcmExCzAJBgNV
BAgMAkNBMQswCQYDVQQGEwJVUzAeFw0yMzA5MjAyMTUzNDNaFw0zMDA5MjAyMTUz
NDNaMHAxIjAgBgNVBAMMGUludGVsIFNHWCBQQ0sgQ2VydGlmaWNhdGUxGjAYBgNV
BAoMEUludGVsIENvcnBvcmF0aW9uMRQwEgYDVQQHDAtTYW50YSBDbGFyYTELMAkG
A1UECAwCQ0ExCzAJBgNVBAYTAlVTMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAE
kgmE7N3D+RspyaCZ2YoDTLDCuh5pnvAu4crPn2uAGujq9tOgwU8/y7jttShCB603
U6r+h9ayOk2nZ9jewk25lqOCAqgwggKkMB8GA1UdIwQYMBaAFNDoqtp11/kuSReY
PHsUZdDV8llNMGwGA1UdHwRlMGMwYaBfoF2GW2h0dHBzOi8vYXBpLnRydXN0ZWRz
ZXJ2aWNlcy5pbnRlbC5jb20vc2d4L2NlcnRpZmljYXRpb24vdjQvcGNrY3JsP2Nh
PXByb2Nlc3NvciZlbmNvZGluZz1kZXIwHQYDVR0OBBYEFIW4KX263PRxYJah2Cfj
AlrcvAC9MA4GA1UdDwEB/wQEAwIGwDAMBgNVHRMBAf8EAjAAMIIB1AYJKoZIhvhN
AQ0BBIIBxTCCAcEwHgYKKoZIhvhNAQ0BAQQQ0E7AbU5tktyQ0K089e4t3zCCAWQG
CiqGSIb4TQENAQIwggFUMBAGCyqGSIb4TQENAQIBAgELMBAGCyqGSIb4TQENAQIC
AgELMBAGCyqGSIb4TQENAQIDAgECMBAGCyqGSIb4TQENAQIEAgECMBEGCyqGSIb4
TQENAQIFAgIA/zAQBgsqhkiG+E0BDQECBgIBATAQBgsqhkiG+E0BDQECBwIBADAQ
BgsqhkiG+E0BDQECCAIBADAQBgsqhkiG+E0BDQECCQIBADAQBgsqhkiG+E0BDQEC
CgIBADAQBgsqhkiG+E0BDQECCwIBADAQBgsqhkiG+E0BDQECDAIBADAQBgsqhkiG
+E0BDQECDQIBADAQBgsqhkiG+E0BDQECDgIBADAQBgsqhkiG+E0BDQECDwIBADAQ
BgsqhkiG+E0BDQECEAIBADAQBgsqhkiG+E0BDQECEQIBDTAfBgsqhkiG+E0BDQEC
EgQQCwsCAv8BAAAAAAAAAAAAADAQBgoqhkiG+E0BDQEDBAIAADAUBgoqhkiG+E0B
DQEEBAYAoGcRAAAwDwYKKoZIhvhNAQ0BBQoBADAKBggqhkjOPQQDAgNHADBEAiBm
SMZEtlQEjnZgGa192W3ArnZ3iyY6ckM/sTsXxCRmJgIgLf20tZHNw3a1b31JDSOW
E6wesxoAmTeqJGRqZl621qI=
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
MIICmDCCAj6gAwIBAgIVANDoqtp11/kuSReYPHsUZdDV8llNMAoGCCqGSM49BAMC
MGgxGjAYBgNVBAMMEUludGVsIFNHWCBSb290IENBMRowGAYDVQQKDBFJbnRlbCBD
b3Jwb3JhdGlvbjEUMBIGA1UEBwwLU2FudGEgQ2xhcmExCzAJBgNVBAgMAkNBMQsw
CQYDVQQGEwJVUzAeFw0xODA1MjExMDUwMTBaFw0zMzA1MjExMDUwMTBaMHExIzAh
BgNVBAMMGkludGVsIFNHWCBQQ0sgUHJvY2Vzc29yIENBMRowGAYDVQQKDBFJbnRl
bCBDb3Jwb3JhdGlvbjEUMBIGA1UEBwwLU2FudGEgQ2xhcmExCzAJBgNVBAgMAkNB
MQswCQYDVQQGEwJVUzBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABL9q+NMp2IOg
tdl1bk/uWZ5+TGQm8aCi8z78fs+fKCQ3d+uDzXnVTAT2ZhDCifyIuJwvN3wNBp9i
HBSSMJMJrBOjgbswgbgwHwYDVR0jBBgwFoAUImUM1lqdNInzg7SVUr9QGzknBqww
UgYDVR0fBEswSTBHoEWgQ4ZBaHR0cHM6Ly9jZXJ0aWZpY2F0ZXMudHJ1c3RlZHNl
cnZpY2VzLmludGVsLmNvbS9JbnRlbFNHWFJvb3RDQS5kZXIwHQYDVR0OBBYEFNDo
qtp11/kuSReYPHsUZdDV8llNMA4GA1UdDwEB/wQEAwIBBjASBgNVHRMBAf8ECDAG
AQH/AgEAMAoGCCqGSM49BAMCA0gAMEUCIQCJgTbtVqOyZ1m3jqiAXM6QYa6r5sWS
4y/G7y8uIJGxdwIgRqPvBSKzzQagBLQq5s5A70pdoiaRJ8z/0uDz4NgV91k=
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
MIICjzCCAjSgAwIBAgIUImUM1lqdNInzg7SVUr9QGzknBqwwCgYIKoZIzj0EAwIw
aDEaMBgGA1UEAwwRSW50ZWwgU0dYIFJvb3QgQ0ExGjAYBgNVBAoMEUludGVsIENv
cnBvcmF0aW9uMRQwEgYDVQQHDAtTYW50YSBDbGFyYTELMAkGA1UECAwCQ0ExCzAJ
BgNVBAYTAlVTMB4XDTE4MDUyMTEwNDUxMFoXDTQ5MTIzMTIzNTk1OVowaDEaMBgG
A1UEAwwRSW50ZWwgU0dYIFJvb3QgQ0ExGjAYBgNVBAoMEUludGVsIENvcnBvcmF0
aW9uMRQwEgYDVQQHDAtTYW50YSBDbGFyYTELMAkGA1UECAwCQ0ExCzAJBgNVBAYT
AlVTMFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEC6nEwMDIYZOj/iPWsCzaEKi7
1OiOSLRFhWGjbnBVJfVnkY4u3IjkDYYL0MxO4mqsyYjlBalTVYxFP2sJBK5zlKOB
uzCBuDAfBgNVHSMEGDAWgBQiZQzWWp00ifODtJVSv1AbOScGrDBSBgNVHR8ESzBJ
MEegRaBDhkFodHRwczovL2NlcnRpZmljYXRlcy50cnVzdGVkc2VydmljZXMuaW50
ZWwuY29tL0ludGVsU0dYUm9vdENBLmRlcjAdBgNVHQ4EFgQUImUM1lqdNInzg7SV
Ur9QGzknBqwwDgYDVR0PAQH/BAQDAgEGMBIGA1UdEwEB/wQIMAYBAf8CAQEwCgYI
KoZIzj0EAwIDSQAwRgIhAOW/5QkR+S9CiSDcNoowLuPRLsWGf/Yi7GSX94BgwTwg
AiEA4J0lrHoMs+Xo5o/sX6O9QWxHRAvZUGOdRQ7cvqRXaqI=
-----END CERTIFICATE-----
"#;

    #[test]
    fn test_extract_fmspc_and_ca_processor() {
        let (fmspc, ca) =
            extract_fmspc_and_ca_with::<DefaultConfig>(TEST_PCK_CHAIN_PROCESSOR).unwrap();
        assert_eq!(fmspc, "00A067110000");
        assert_eq!(ca, PROCESSOR_ISSUER_ID);
    }

    #[test]
    fn test_pcs_endpoints_new() {
        // Test SGX endpoint initialization
        let sgx_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(sgx_endpoints.base_url, "https://pccs.example.com");
        assert_eq!(sgx_endpoints.tee, "sgx");
        assert_eq!(sgx_endpoints.fmspc, "B0C06F000000");
        assert_eq!(sgx_endpoints.ca, PROCESSOR_ISSUER_ID);

        // Test TDX endpoint initialization
        let tdx_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            false,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(tdx_endpoints.base_url, "https://pccs.example.com");
        assert_eq!(tdx_endpoints.tee, "tdx");
        assert_eq!(tdx_endpoints.fmspc, "B0C06F000000");
        assert_eq!(tdx_endpoints.ca, PROCESSOR_ISSUER_ID);

        // Test URL normalization during initialization
        let endpoints_with_trailing_slash = PcsEndpoints::new(
            "https://pccs.example.com/",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            endpoints_with_trailing_slash.base_url,
            "https://pccs.example.com"
        );

        // Test URL normalization with SGX certification path
        let endpoints_with_sgx_path = PcsEndpoints::new(
            "https://pccs.example.com/sgx/certification/v4",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(endpoints_with_sgx_path.base_url, "https://pccs.example.com");

        // Test URL normalization with TDX certification path
        let endpoints_with_tdx_path = PcsEndpoints::new(
            "https://pccs.example.com/tdx/certification/v4",
            false,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(endpoints_with_tdx_path.base_url, "https://pccs.example.com");
    }

    #[test]
    fn test_pcs_endpoints_url_pckcrl() {
        // Test with processor CA
        let processor_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            processor_endpoints.url_pckcrl(),
            "https://pccs.example.com/sgx/certification/v4/pckcrl?ca=processor&encoding=der"
        );

        // Test with platform CA
        let platform_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            true,
            "B0C06F000000".to_string(),
            PLATFORM_ISSUER_ID,
        );
        assert_eq!(
            platform_endpoints.url_pckcrl(),
            "https://pccs.example.com/sgx/certification/v4/pckcrl?ca=platform&encoding=der"
        );
    }

    #[test]
    fn test_pcs_endpoints_url_rootcacrl() {
        let endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            endpoints.url_rootcacrl(),
            "https://pccs.example.com/sgx/certification/v4/rootcacrl"
        );
    }

    #[test]
    fn test_pcs_endpoints_url_tcb() {
        // Test SGX TCB URL
        let sgx_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            sgx_endpoints.url_tcb(),
            "https://pccs.example.com/sgx/certification/v4/tcb?fmspc=B0C06F000000"
        );

        // Test TDX TCB URL
        let tdx_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            false,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            tdx_endpoints.url_tcb(),
            "https://pccs.example.com/tdx/certification/v4/tcb?fmspc=B0C06F000000"
        );
    }

    #[test]
    fn test_pcs_endpoints_url_qe_identity() {
        // Test SGX QE identity URL
        let sgx_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            true,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            sgx_endpoints.url_qe_identity(),
            "https://pccs.example.com/sgx/certification/v4/qe/identity?update=standard"
        );

        // Test TDX QE identity URL
        let tdx_endpoints = PcsEndpoints::new(
            "https://pccs.example.com",
            false,
            "B0C06F000000".to_string(),
            PROCESSOR_ISSUER_ID,
        );
        assert_eq!(
            tdx_endpoints.url_qe_identity(),
            "https://pccs.example.com/tdx/certification/v4/qe/identity?update=standard"
        );
    }

    #[test]
    fn test_intel_pcs_url() {
        // Test the Intel PCS URL constant
        assert_eq!(INTEL_PCS_URL, "https://api.trustedservices.intel.com");

        // Test the Phala PCCS URL constant
        assert_eq!(PHALA_PCCS_URL, "https://pccs.phala.network");

        // Test with the known FMSPC from memory
        let fmspc = "B0C06F000000";
        let intel_endpoints =
            PcsEndpoints::new(INTEL_PCS_URL, true, fmspc.to_string(), PROCESSOR_ISSUER_ID);

        assert_eq!(
            intel_endpoints.url_pckcrl(),
            "https://api.trustedservices.intel.com/sgx/certification/v4/pckcrl?ca=processor&encoding=der"
        );

        assert_eq!(
            intel_endpoints.url_rootcacrl(),
            "https://api.trustedservices.intel.com/sgx/certification/v4/rootcacrl"
        );

        assert_eq!(
            intel_endpoints.url_tcb(),
            "https://api.trustedservices.intel.com/sgx/certification/v4/tcb?fmspc=B0C06F000000"
        );

        assert_eq!(
            intel_endpoints.url_qe_identity(),
            "https://api.trustedservices.intel.com/sgx/certification/v4/qe/identity?update=standard"
        );
    }
}
