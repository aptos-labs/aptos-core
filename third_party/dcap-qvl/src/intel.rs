use anyhow::{anyhow, bail, Context, Result};
use asn1_der::{
    typed::{DerDecodable, Sequence},
    DerObject,
};

#[cfg(feature = "default-x509")]
use crate::configs::DefaultConfig;
use crate::{
    config::{Config, ParsedCert, PckCa, X509Codec},
    constants::{self, CpuSvn, Fmspc, Svn},
    oids,
    quote::{AuthData, Quote},
    utils,
};

/// Parsed values from the Intel SGX extension in a PCK certificate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PckExtension {
    pub ppid: Vec<u8>,
    pub cpu_svn: CpuSvn,
    pub pce_svn: Svn,
    pub pce_id: Vec<u8>,
    pub fmspc: Fmspc,
    pub sgx_type: u64,
    pub platform_instance_id: Option<Vec<u8>>,
    pub raw_extension: Vec<u8>,
    /// Whether the platform can be extended with additional packages
    /// (Platform CA certs only; `None` for Processor CA certs)
    pub dynamic_platform: Option<bool>,
    /// Whether platform root keys are cached by SGX Registration Backend
    /// (Platform CA certs only; `None` for Processor CA certs)
    pub cached_keys: Option<bool>,
    /// Whether SMT (simultaneous multithreading / hyperthreading) is enabled
    /// (Platform CA certs only; `None` for Processor CA certs)
    pub smt_enabled: Option<bool>,
}

impl PckExtension {
    /// Look up an arbitrary OID inside the raw Intel SGX extension.
    ///
    /// The search is recursive: nested SEQUENCE containers are walked
    /// automatically so the caller only needs to supply the leaf OID.
    pub fn get_value(&self, oid: &const_oid::ObjectIdentifier) -> Result<Option<Vec<u8>>> {
        let obj = DerObject::decode(&self.raw_extension).context("Failed to decode DER object")?;
        find_recursive(oid, obj, 0)
    }
}

const MAX_DER_RECURSION_DEPTH: usize = 10;

/// Return the PCK certificate chain (DER encoded) embedded inside the quote.
///
/// * For certification data type 5 this returns the entire chain.
/// * For certification data type 4 this returns the single PCK certificate.
pub fn extract_cert_chain(quote: &Quote) -> Result<Vec<Vec<u8>>> {
    if let Ok(chain_bytes) = quote.raw_cert_chain() {
        let certs = utils::extract_certs(chain_bytes)?;
        return Ok(certs
            .into_iter()
            .map(|cert| cert.as_ref().to_vec())
            .collect());
    }

    let cert_data = match &quote.auth_data {
        AuthData::V3(data) => &data.certification_data,
        AuthData::V4(data) => &data.qe_report_data.certification_data,
    };
    if cert_data.cert_type == constants::PCK_ID_PCK_CERTIFICATE {
        return Ok(vec![cert_data.body.data.clone()]);
    }

    bail!(
        "Certification data type {} is not supported (expecting 4 or 5)",
        cert_data.cert_type
    );
}

/// Generic version of [`parse_pck_extension`] using a custom [`Config`].
pub fn parse_pck_extension_with<C: Config>(cert_der: &[u8]) -> Result<PckExtension> {
    let extension = utils::get_intel_extension_with::<C>(cert_der)?;

    let ppid = find_extension_required(&[oids::PPID], &extension)?;
    let cpu_svn = utils::get_cpu_svn(&extension)?;
    let pce_svn = utils::get_pce_svn(&extension)?;
    let pce_id = find_extension_required(&[oids::PCEID], &extension)?;
    let fmspc = utils::get_fmspc(&extension)?;
    let sgx_type = decode_enumerated(&find_extension_required(&[oids::SGX_TYPE], &extension)?)?;
    let platform_instance_id = find_extension_optional(&[oids::PLATFORM_INSTANCE_ID], &extension)?;

    // Configuration flags (only present in Platform CA certs, under OID 1.2.840.113741.1.13.1.7)
    let dynamic_platform =
        find_extension_optional(&[oids::CONFIGURATION, oids::DYNAMIC_PLATFORM], &extension)?
            .map(|v| decode_boolean(&v))
            .transpose()?;
    let cached_keys =
        find_extension_optional(&[oids::CONFIGURATION, oids::CACHED_KEYS], &extension)?
            .map(|v| decode_boolean(&v))
            .transpose()?;
    let smt_enabled =
        find_extension_optional(&[oids::CONFIGURATION, oids::SMT_ENABLED], &extension)?
            .map(|v| decode_boolean(&v))
            .transpose()?;

    Ok(PckExtension {
        ppid,
        cpu_svn,
        pce_svn,
        pce_id,
        fmspc,
        sgx_type,
        platform_instance_id,
        raw_extension: extension,
        dynamic_platform,
        cached_keys,
        smt_enabled,
    })
}

/// Parse the Intel SGX extension values from a DER-encoded PCK certificate.
///
/// Uses the audited [`DefaultConfig`]. For a custom backend, use
/// [`parse_pck_extension_with`].
#[cfg(feature = "default-x509")]
pub fn parse_pck_extension(cert_der: &[u8]) -> Result<PckExtension> {
    parse_pck_extension_with::<DefaultConfig>(cert_der)
}

/// Generic version of [`parse_pck_extension_from_pem`] using a custom [`Config`].
pub fn parse_pck_extension_from_pem_with<C: Config>(pem_data: &[u8]) -> Result<PckExtension> {
    let certs = utils::extract_certs(pem_data)?;
    let leaf = certs
        .first()
        .ok_or_else(|| anyhow!("No certificates found in PEM chain"))?;
    parse_pck_extension_with::<C>(leaf)
}

/// Parse the Intel SGX extension from a PEM-encoded certificate chain.
///
/// The first (leaf) certificate in the chain is used. Uses the audited
/// [`DefaultConfig`]. For a custom backend, use
/// [`parse_pck_extension_from_pem_with`].
#[cfg(feature = "default-x509")]
pub fn parse_pck_extension_from_pem(pem_data: &[u8]) -> Result<PckExtension> {
    parse_pck_extension_from_pem_with::<DefaultConfig>(pem_data)
}

/// Classify the PCK certificate authority of a leaf cert as
/// [`PckCa::Processor`] or [`PckCa::Platform`] by inspecting the issuer DN.
///
/// Generic over [`Config`]; the classification goes through the
/// configured [`X509Codec`]. An unrecognised issuer is reported as
/// [`PckCa::Processor`] for backwards compatibility — see comment.
pub fn pck_ca_with<C: Config>(cert_der: &[u8]) -> Result<PckCa> {
    let parsed = C::X509::from_der(cert_der).context("Failed to decode certificate")?;
    // Legacy fallback: an unrecognised issuer is treated as Processor. This
    // policy lives at the call site, not in the backend, so future callers
    // can opt to error or log on `None` without needing trait changes.
    Ok(parsed.pck_ca().unwrap_or(PckCa::Processor))
}

/// [`pck_ca_with`] under the audited [`DefaultConfig`].
#[cfg(feature = "default-x509")]
pub fn pck_ca(cert_der: &[u8]) -> Result<PckCa> {
    pck_ca_with::<DefaultConfig>(cert_der)
}

/// Return the FMSPC of a quote's PCK leaf certificate.
///
/// Convenience over [`extract_cert_chain`] + [`parse_pck_extension_with`];
/// generic over [`Config`].
pub fn quote_fmspc_with<C: Config>(quote: &Quote) -> Result<Fmspc> {
    let chain = extract_cert_chain(quote)?;
    let leaf = chain.first().context("Empty PCK certificate chain")?;
    Ok(parse_pck_extension_with::<C>(leaf)?.fmspc)
}

/// [`quote_fmspc_with`] under the audited [`DefaultConfig`].
#[cfg(feature = "default-x509")]
pub fn quote_fmspc(quote: &Quote) -> Result<Fmspc> {
    quote_fmspc_with::<DefaultConfig>(quote)
}

/// Return the PCK CA classification of a quote's PCK leaf certificate.
///
/// Generic over [`Config`].
pub fn quote_ca_with<C: Config>(quote: &Quote) -> Result<PckCa> {
    let chain = extract_cert_chain(quote)?;
    let leaf = chain.first().context("Empty PCK certificate chain")?;
    pck_ca_with::<C>(leaf)
}

/// [`quote_ca_with`] under the audited [`DefaultConfig`].
#[cfg(feature = "default-x509")]
pub fn quote_ca(quote: &Quote) -> Result<PckCa> {
    quote_ca_with::<DefaultConfig>(quote)
}

fn find_extension_required(
    path: &[const_oid::ObjectIdentifier],
    extension: &[u8],
) -> Result<Vec<u8>> {
    find_extension_optional(path, extension)?
        .ok_or_else(|| anyhow!("Intel extension path {path:?} is missing"))
}

fn find_extension_optional(
    path: &[const_oid::ObjectIdentifier],
    extension: &[u8],
) -> Result<Option<Vec<u8>>> {
    let mut obj = DerObject::decode(extension).context("Failed to decode DER object")?;
    for oid in path {
        let seq = Sequence::load(obj).context("Failed to load sequence")?;
        match sub_object_opt(oid, seq)? {
            Some(value) => obj = value,
            None => return Ok(None),
        }
    }
    Ok(Some(obj.value().to_vec()))
}

fn sub_object_opt<'a>(
    oid: &const_oid::ObjectIdentifier,
    seq: Sequence<'a>,
) -> Result<Option<DerObject<'a>>> {
    for idx in 0..seq.len() {
        let entry = seq
            .get(idx)
            .context("Failed to read entry inside Intel extension")?;
        let entry_seq = Sequence::load(entry).context("Failed to load nested sequence")?;
        let name = entry_seq.get(0).context("Failed to read OID")?;
        let value = entry_seq.get(1).context("Failed to read value")?;
        if name.value() == oid.as_bytes() {
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn find_recursive<'a>(
    oid: &const_oid::ObjectIdentifier,
    obj: DerObject<'a>,
    depth: usize,
) -> Result<Option<Vec<u8>>> {
    if depth > MAX_DER_RECURSION_DEPTH {
        bail!("DER recursion depth exceeded");
    }
    let seq = match Sequence::load(obj) {
        Ok(s) => s,
        Err(_) => return Ok(None),
    };
    for idx in 0..seq.len() {
        let entry = match seq.get(idx) {
            Ok(e) => e,
            Err(_) => continue,
        };
        let entry_seq = match Sequence::load(entry) {
            Ok(s) => s,
            Err(_) => continue,
        };
        let name = match entry_seq.get(0) {
            Ok(n) => n,
            Err(_) => continue,
        };
        let value = match entry_seq.get(1) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if name.value() == oid.as_bytes() {
            return Ok(Some(value.value().to_vec()));
        }
        // Tag 0x30 = SEQUENCE — recurse into nested containers
        if value.tag() == 0x30 {
            let next_depth = depth
                .checked_add(1)
                .context("DER recursion depth overflow")?;
            if let Some(found) = find_recursive(oid, value, next_depth)? {
                return Ok(Some(found));
            }
        }
    }
    Ok(None)
}

fn decode_boolean(bytes: &[u8]) -> Result<bool> {
    match bytes[..] {
        [0x00] => Ok(false),
        [_] => Ok(true),
        _ => bail!("Unexpected BOOLEAN length: {}", bytes.len()),
    }
}

fn decode_enumerated(bytes: &[u8]) -> Result<u64> {
    match bytes[..] {
        [byte0] => Ok(u64::from(byte0)),
        [byte0, byte1] => Ok(u16::from_be_bytes([byte0, byte1]) as u64),
        _ => bail!("Unexpected ENUMERATED length"),
    }
}
