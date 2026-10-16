use alloc::vec::Vec;
use anyhow::{anyhow, bail, ensure, Context, Result};
use asn1_der::{
    typed::{DerDecodable, Sequence},
    DerObject,
};
use rustls_pki_types::{CertificateDer, TrustAnchor, UnixTime};
use webpki::CertRevocationList;
use webpki::{self, OwnedCertRevocationList};

use crate::{
    config::{Config, ParsedCert, X509Codec},
    constants::*,
    oids,
};

/// Look up the Intel SGX OID extension's OCTET STRING contents in a PCK
/// certificate. Generic over [`Config`].
pub fn get_intel_extension_with<C: Config>(der_encoded: &[u8]) -> Result<Vec<u8>> {
    C::X509::from_der(der_encoded)?
        .extension(oids::SGX_EXTENSION.as_bytes())?
        .context("Intel extension not found")
}

pub fn find_extension(path: &[&[u8]], raw: &[u8]) -> Result<Vec<u8>> {
    let obj = DerObject::decode(raw).context("Failed to decode DER object")?;
    let subobj = get_obj(path, obj).context("Failed to get subobject")?;
    Ok(subobj.value().to_vec())
}

fn get_obj<'a>(path: &[&[u8]], mut obj: DerObject<'a>) -> Result<DerObject<'a>> {
    for oid in path {
        let seq = Sequence::load(obj).context("Failed to load sequence")?;
        obj = sub_obj(oid, seq).context("Failed to get subobject")?;
    }
    Ok(obj)
}

fn sub_obj<'a>(oid: &[u8], seq: Sequence<'a>) -> Result<DerObject<'a>> {
    for i in 0..seq.len() {
        let entry = seq.get(i).context("Failed to get entry")?;
        let entry = Sequence::load(entry).context("Failed to load sequence")?;
        let name = entry.get(0).context("Failed to get name")?;
        let value = entry.get(1).context("Failed to get value")?;
        if name.value() == oid {
            return Ok(value);
        }
    }
    bail!("OID is missing");
}

pub(crate) fn get_fmspc(extension_section: &[u8]) -> Result<Fmspc> {
    let data = find_extension(&[oids::FMSPC.as_bytes()], extension_section)
        .context("Failed to find Fmspc")?;
    if data.len() != 6 {
        bail!("Fmspc length mismatch");
    }

    data.try_into()
        .map_err(|_| anyhow!("Failed to decode Fmspc"))
}

pub fn get_cpu_svn(extension_section: &[u8]) -> Result<CpuSvn> {
    let data = find_extension(
        &[oids::TCB.as_bytes(), oids::CPUSVN.as_bytes()],
        extension_section,
    )?;
    if data.len() != 16 {
        bail!("CpuSvn length mismatch");
    }

    data.try_into()
        .map_err(|_| anyhow!("Failed to decode CpuSvn"))
}

pub fn get_pce_svn(extension_section: &[u8]) -> Result<Svn> {
    let data = find_extension(
        &[oids::TCB.as_bytes(), oids::PCESVN.as_bytes()],
        extension_section,
    )
    .context("Failed to find PceSvn")?;

    match data[..] {
        [byte0] => Ok(u16::from(byte0)),
        [byte0, byte1] => Ok(u16::from_be_bytes([byte0, byte1])),
        _ => bail!("PceSvn length mismatch"),
    }
}

pub(crate) fn extract_raw_certs(cert_chain: &[u8]) -> Result<Vec<Vec<u8>>> {
    Ok(pem::parse_many(cert_chain)
        .context("Failed to parse certs")?
        .iter()
        .map(|i| i.contents().to_vec())
        .collect())
}

pub(crate) fn parse_rfc3339_unix_secs(value: &str) -> Result<u64> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map_err(|e| anyhow!("Failed to parse RFC3339 datetime: {e}"))?
        .timestamp()
        .try_into()
        .context("RFC3339 datetime is before Unix epoch")
}

pub(crate) mod serde_vec_bytes {
    use alloc::vec::Vec;
    use serde::ser::SerializeSeq;
    use serde::{Deserialize, Deserializer, Serializer};
    use serde_bytes::{ByteBuf, Bytes};

    pub fn serialize<S>(value: &[Vec<u8>], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(value.len()))?;
        for item in value {
            seq.serialize_element(Bytes::new(item))?;
        }
        seq.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Vec::<ByteBuf>::deserialize(deserializer)?;
        Ok(value.into_iter().map(ByteBuf::into_vec).collect())
    }
}

pub fn extract_certs<'a>(cert_chain: &'a [u8]) -> Result<Vec<CertificateDer<'a>>> {
    let mut certs = Vec::<CertificateDer<'a>>::new();

    let raw_certs = extract_raw_certs(cert_chain)?;
    for raw_cert in raw_certs.iter() {
        let cert = rustls_pki_types::CertificateDer::<'a>::from(raw_cert.to_vec());
        certs.push(cert);
    }

    Ok(certs)
}

/// Split a 64-byte raw `r ‖ s` payload at byte 32 and DER-encode it as
/// `Ecdsa-Sig-Value` (RFC 5480) using the [`Config::SigEncoder`] of `C`.
pub fn encode_as_der_with<C: Config>(data: &[u8]) -> Result<Vec<u8>> {
    use crate::config::EcdsaSigEncoder;
    let (first, second) = data.split_at_checked(32).context("Invalid key length")?;
    C::SigEncoder::encode_ecdsa_sig(first, second)
}

/// Extract the CRL Number (OID 2.5.29.20) from a DER-encoded CRL.
///
/// Returns `Ok(0)` if the CRL Number extension is not present.
#[cfg(feature = "default-x509")]
pub fn extract_crl_number(crl_der: &[u8]) -> Result<u32> {
    use der::Decode as _;
    let crl: x509_cert::crl::CertificateList<x509_cert::certificate::Rfc5280> =
        x509_cert::crl::CertificateList::from_der(crl_der).context("Failed to parse CRL")?;
    let Some(extensions) = &crl.tbs_cert_list.crl_extensions else {
        return Ok(0);
    };
    for ext in extensions.iter() {
        // OID 2.5.29.20 = id-ce-cRLNumber
        if ext.extn_id.to_string() == "2.5.29.20" {
            // CRL Number is encoded as an ASN.1 INTEGER
            let crl_num =
                der::asn1::UintRef::from_der(ext.extn_value.as_bytes()).context("CRL number")?;
            let bytes = crl_num.as_bytes();
            // Convert big-endian bytes to u32 (CRL numbers are typically small)
            ensure!(bytes.len() <= 4, "CRL number too large for u32");
            let mut val: u32 = 0;
            for &b in bytes {
                val = (val << 8) | u32::from(b);
            }
            return Ok(val);
        }
    }
    Ok(0)
}

/// Parse CRL DER bytes into CertRevocationList objects.
/// Call this once and pass the results to `verify_certificate_chain`.
pub fn parse_crls(crl_der: &[&[u8]]) -> Result<Vec<CertRevocationList<'static>>> {
    crl_der
        .iter()
        .map(|der| OwnedCertRevocationList::from_der(der).map(CertRevocationList::from))
        .collect::<Result<Vec<_>, _>>()
        .context("Failed to parse CRL")
}

/// Verifies that the `leaf_cert` in combination with the `intermediate_certs` establishes
/// a valid certificate chain that is rooted in one of the trust anchors that was compiled into the pallet
///
/// It will also check that the certificate is not revoked according to the CRL
pub fn verify_certificate_chain(
    leaf_cert: &webpki::EndEntityCert,
    intermediate_certs: &[CertificateDer],
    time: UnixTime,
    crls: &[CertRevocationList<'_>],
    trust_anchor: TrustAnchor<'_>,
) -> Result<()> {
    let sig_algs = webpki::ALL_VERIFICATION_ALGS;

    let crl_slice = crls.iter().collect::<Vec<_>>();

    // Create a RevocationOptions object with the CRL
    let builder = match webpki::RevocationOptionsBuilder::new(&crl_slice) {
        Ok(builder) => builder,
        Err(_) => bail!("Failed to create RevocationOptionsBuilder - CRLs required"),
    };
    let revocation = builder
        .with_depth(webpki::RevocationCheckDepth::Chain)
        .with_status_policy(webpki::UnknownStatusPolicy::Deny)
        .with_expiration_policy(webpki::ExpirationPolicy::Enforce)
        .build();

    leaf_cert
        .verify_for_usage(
            sig_algs,
            &[trust_anchor],
            intermediate_certs,
            time,
            webpki::KeyUsage::server_auth(),
            Some(revocation),
            None,
        )
        .context("Failed to verify certificate chain")?;

    Ok(())
}
