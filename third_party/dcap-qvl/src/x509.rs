//! Audited [`X509Codec`] implementation backed by `x509-cert` + `der`.
//!
//! Selected by [`crate::configs::RingConfig`] / [`crate::configs::RustCryptoConfig`]
//! / [`crate::configs::DefaultConfig`].

use alloc::string::ToString;
use alloc::vec::Vec;
use anyhow::{anyhow, bail, Context, Result};

use crate::config::{ParsedCert, PckCa, X509Codec};
use crate::constants;

/// Audited default [`X509Codec`] implementation, built on `x509-cert` + `der`.
///
/// Zero-sized factory; produces [`X509CertParsed`] from raw DER bytes. The
/// `x509_cert::Certificate` type owns its parsed data, so this backend does
/// not borrow from the input slice — but the GAT shape leaves room for
/// downstream zero-copy backends (e.g. `asn1_der`).
pub struct X509CertBackend;

/// Owned, parsed X.509 certificate produced by [`X509CertBackend`].
pub struct X509CertParsed {
    cert: x509_cert::Certificate,
}

impl X509Codec for X509CertBackend {
    type Parsed<'a> = X509CertParsed;

    fn from_der<'a>(cert_der: &'a [u8]) -> Result<Self::Parsed<'a>> {
        let cert: x509_cert::Certificate =
            der::Decode::from_der(cert_der).context("Failed to decode certificate")?;
        Ok(X509CertParsed { cert })
    }
}

impl ParsedCert for X509CertParsed {
    fn pck_ca(&self) -> Option<PckCa> {
        let issuer_dn = self.cert.tbs_certificate().issuer().to_string();
        if issuer_dn.contains(constants::PROCESSOR_ISSUER) {
            return Some(PckCa::Processor);
        }
        if issuer_dn.contains(constants::PLATFORM_ISSUER) {
            return Some(PckCa::Platform);
        }
        None
    }

    fn extension(&self, oid: &[u8]) -> Result<Option<Vec<u8>>> {
        let oid = const_oid::ObjectIdentifier::from_bytes(oid)
            .map_err(|_| anyhow!("Invalid OID encoding"))?;
        let mut iter = self
            .cert
            .tbs_certificate()
            .extensions()
            .map_or(&[][..], Vec::as_slice)
            .iter()
            .filter(|e| e.extn_id == oid)
            .map(|e| e.extn_value.clone());

        let extension = match iter.next() {
            Some(ext) => ext,
            None => return Ok(None),
        };
        if iter.next().is_some() {
            bail!("extension {} appears more than once", oid);
        }
        Ok(Some(extension.into_bytes().into_vec()))
    }
}
