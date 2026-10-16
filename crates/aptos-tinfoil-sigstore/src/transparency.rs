// Adapted from sigstore-rs (Apache 2.0 License)
// https://github.com/sigstore/sigstore-rs/blob/main/src/crypto/transparency.rs
//
// Copyright 2023 The Sigstore Authors.
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

//! Types for Certificate Transparency validation.
//!
//! Adapted from sigstore-rs transparency.rs to use x509-cert's SCT types
//! and our keyring implementation.

use super::keyring::{Keyring, KeyringError};
use der::{asn1::BitString, Decode, Encode};
use sha2::{Digest, Sha256};
use thiserror::Error;
use tls_codec::{SerializeBytes, TlsByteVecU16, TlsByteVecU24, TlsSerializeBytes, TlsSize};
use x509_cert::{
    ext::{
        pkix::sct::{SignedCertificateTimestamp, SignedCertificateTimestampList, Version},
        Extensions,
    },
    name::Name,
    serial_number::SerialNumber,
    time::Validity,
    AlgorithmIdentifier, Certificate, SubjectPublicKeyInfo,
};

/// OID for CT Precert SCTs extension (1.3.6.1.4.1.11129.2.4.2)
const CT_PRECERT_SCTS_OID: &str = "1.3.6.1.4.1.11129.2.4.2";

#[derive(Debug, Error)]
pub enum CertificateErrorKind {
    #[error("SCT list extension missing from leaf certificate")]
    LeafSCTMissing,

    #[error("cannot find leaf certificate's issuer")]
    IssuerMissing,

    #[error("cannot decode SCT")]
    LeafSCTMalformed,

    #[error(transparent)]
    Der(#[from] der::Error),

    #[error("TLS codec error: {0}")]
    Tls(String),
}

impl From<x509_cert::ext::pkix::sct::Error> for CertificateErrorKind {
    fn from(value: x509_cert::ext::pkix::sct::Error) -> Self {
        match value {
            x509_cert::ext::pkix::sct::Error::Der(e) => CertificateErrorKind::Der(e),
            x509_cert::ext::pkix::sct::Error::Tls(e) => {
                CertificateErrorKind::Tls(format!("{:?}", e))
            },
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum SCTError {
    #[error("failed to extract SCT from certificate")]
    Parsing(#[from] CertificateErrorKind),

    #[error("failed to reconstruct signed payload")]
    Serialization(#[source] tls_codec::Error),

    #[error("failed to verify SCT")]
    Verification(#[from] KeyringError),
}

/// DER mirror of x509-cert's `TbsCertificateInner`. x509-cert 0.3 keeps the
/// struct's fields private, so the SCT-extension filter in the `From` impl
/// below re-encodes through this copy. Field order and `asn1` attributes
/// replicate RFC 5280's TBSCertificate exactly; DER is canonical, so the
/// round-trip reproduces the original encoding byte-for-byte apart from the
/// filtered extension list.
#[derive(Debug, der::Sequence)]
struct TbsMirror {
    #[asn1(context_specific = "0", default = "Default::default")]
    version: x509_cert::Version,
    serial_number: SerialNumber,
    signature: AlgorithmIdentifier,
    issuer: Name,
    validity: Validity,
    subject: Name,
    subject_public_key_info: SubjectPublicKeyInfo,
    #[asn1(context_specific = "1", tag_mode = "IMPLICIT", optional = "true")]
    issuer_unique_id: Option<BitString>,
    #[asn1(context_specific = "2", tag_mode = "IMPLICIT", optional = "true")]
    subject_unique_id: Option<BitString>,
    #[asn1(context_specific = "3", tag_mode = "EXPLICIT", optional = "true")]
    extensions: Option<Extensions>,
}

#[derive(PartialEq, Debug, TlsSerializeBytes, TlsSize)]
#[repr(u8)]
enum SignatureType {
    CertificateTimestamp = 0,
    #[allow(dead_code)]
    TreeHash = 1,
}

#[derive(PartialEq, Debug)]
#[repr(u16)]
enum LogEntryType {
    #[allow(dead_code)]
    X509Entry = 0,
    PrecertEntry = 1,
}

#[derive(PartialEq, Debug, TlsSerializeBytes, TlsSize)]
struct PreCert {
    // opaque issuer_key_hash[32];
    issuer_key_hash: [u8; 32],
    // opaque TBSCertificate<1..2^24-1>;
    tbs_certificate: TlsByteVecU24,
}

#[derive(PartialEq, Debug, TlsSerializeBytes, TlsSize)]
#[repr(u16)]
enum SignedEntry {
    // opaque ASN.1Cert<1..2^24-1>;
    #[allow(dead_code)]
    #[tls_codec(discriminant = "LogEntryType::X509Entry")]
    X509Entry(TlsByteVecU24),
    #[tls_codec(discriminant = "LogEntryType::PrecertEntry")]
    PrecertEntry(PreCert),
}

#[derive(PartialEq, Debug, TlsSerializeBytes, TlsSize)]
pub struct DigitallySigned {
    version: Version,
    signature_type: SignatureType,
    timestamp: u64,
    signed_entry: SignedEntry,
    // opaque CtExtensions<0..2^16-1>;
    extensions: TlsByteVecU16,

    // These fields are not encoded into the TLS DigitallySigned blob,
    // but we need them to properly verify the reconstructed message.
    #[tls_codec(skip)]
    log_id: [u8; 32],
    #[tls_codec(skip)]
    signature: Vec<u8>,
}

/// Represents an SCT embedded in a certificate, ready for verification.
///
/// Adapted from sigstore-rs CertificateEmbeddedSCT.
#[derive(Debug)]
pub struct CertificateEmbeddedSCT<'a> {
    cert: &'a Certificate,
    sct: SignedCertificateTimestamp,
    issuer_id: [u8; 32],
}

impl<'a> CertificateEmbeddedSCT<'a> {
    /// Creates a new CertificateEmbeddedSCT from a certificate and issuer SPKI.
    pub fn new_with_spki(cert: &'a Certificate, spki: &[u8]) -> Result<Self, SCTError> {
        let scts = parse_scts_from_cert(cert)?;

        // We expect at least one SCT
        let sct = scts
            .into_iter()
            .next()
            .ok_or(CertificateErrorKind::LeafSCTMissing)?;

        let issuer_id = {
            let mut hasher = Sha256::new();
            hasher.update(spki);
            hasher.finalize().into()
        };

        Ok(Self {
            cert,
            sct,
            issuer_id,
        })
    }

    /// Returns the SCT's log ID (32-byte key hash).
    pub fn log_id(&self) -> [u8; 32] {
        self.sct.log_id.key_id
    }

    /// Creates CertificateEmbeddedSCTs for ALL SCTs in the certificate.
    ///
    /// Returns an error only if no SCTs are found. Each SCT can then be
    /// independently verified against the CT log keyring.
    pub fn all_from_cert(cert: &'a Certificate, spki: &[u8]) -> Result<Vec<Self>, SCTError> {
        let scts = parse_scts_from_cert(cert)?;

        if scts.is_empty() {
            return Err(CertificateErrorKind::LeafSCTMissing.into());
        }

        let issuer_id: [u8; 32] = {
            let mut hasher = Sha256::new();
            hasher.update(spki);
            hasher.finalize().into()
        };

        Ok(scts
            .into_iter()
            .map(|sct| Self {
                cert,
                sct,
                issuer_id,
            })
            .collect())
    }
}

impl From<&CertificateEmbeddedSCT<'_>> for DigitallySigned {
    fn from(value: &CertificateEmbeddedSCT) -> Self {
        // Construct the precert by filtering out the SCT extension. x509-cert
        // 0.3 keeps the TBS fields private, so the filter runs on a DER mirror
        // of the parsed TBS and the result is re-encoded.
        let tbs_der = value
            .cert
            .tbs_certificate()
            .to_der()
            .expect("failed to encode TBSCertificate!");
        let mut tbs_precert =
            TbsMirror::from_der(&tbs_der).expect("failed to re-decode TBSCertificate!");
        tbs_precert.extensions = tbs_precert.extensions.map(|exts| {
            exts.iter()
                .filter(|v| v.extn_id.to_string() != CT_PRECERT_SCTS_OID)
                .cloned()
                .collect()
        });

        let mut tbs_precert_der = Vec::new();
        tbs_precert
            .encode_to_vec(&mut tbs_precert_der)
            .expect("failed to re-encode Precertificate!");

        DigitallySigned {
            version: Version::V1,
            signature_type: SignatureType::CertificateTimestamp,
            timestamp: value.sct.timestamp,
            signed_entry: SignedEntry::PrecertEntry(PreCert {
                issuer_key_hash: value.issuer_id,
                tbs_certificate: tbs_precert_der.as_slice().into(),
            }),
            extensions: value.sct.extensions.clone(),

            log_id: value.sct.log_id.key_id,
            signature: value.sct.signature.signature.as_slice().to_vec(),
        }
    }
}

/// Verifies a given signing certificate's Signed Certificate Timestamp.
///
/// SCT verification as defined by [RFC 6962] guarantees that a given certificate has been submitted
/// to a Certificate Transparency log. Verification should be performed on the signing certificate
/// in Sigstore verify and sign flows. Certificates that fail SCT verification are misissued and
/// MUST NOT be trusted.
///
/// The CT log key's validity period is checked against the SCT timestamp to ensure the
/// key was valid when the SCT was issued (per sigstore-browser reference implementation).
///
/// [RFC 6962]: https://datatracker.ietf.org/doc/html/rfc6962
pub fn verify_sct<S>(sct: S, keyring: &Keyring) -> Result<(), SCTError>
where
    S: Into<DigitallySigned>,
{
    let sct: DigitallySigned = sct.into();
    let serialized = sct.tls_serialize().map_err(SCTError::Serialization)?;

    // SCT timestamp is in milliseconds since epoch (RFC 6962), convert to seconds
    let timestamp_secs = sct.timestamp / 1000;
    keyring.verify_at(&sct.log_id, &sct.signature, &serialized, timestamp_secs)?;

    Ok(())
}

/// Parse SCTs from certificate using x509-cert types
fn parse_scts_from_cert(cert: &Certificate) -> Result<Vec<SignedCertificateTimestamp>, SCTError> {
    let extensions = cert
        .tbs_certificate()
        .extensions()
        .ok_or(CertificateErrorKind::LeafSCTMissing)?;

    // Find SCT extension
    let sct_ext = extensions
        .iter()
        .find(|ext| ext.extn_id.to_string() == CT_PRECERT_SCTS_OID)
        .ok_or(CertificateErrorKind::LeafSCTMissing)?;

    // Parse using x509-cert's SCT types
    let sct_list = SignedCertificateTimestampList::from_der(sct_ext.extn_value.as_bytes())
        .map_err(CertificateErrorKind::from)?;

    let serialized_scts = sct_list
        .parse_timestamps()
        .map_err(CertificateErrorKind::from)?;

    let mut scts = Vec::new();
    for serialized in serialized_scts {
        let sct = serialized
            .parse_timestamp()
            .map_err(CertificateErrorKind::from)?;
        scts.push(sct);
    }

    Ok(scts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use der::DecodePem;
    use p256::{
        ecdsa::VerifyingKey,
        pkcs8::{DecodePublicKey, EncodePublicKey},
    };

    // Test data from sigstore-rs
    #[test]
    fn verify_embedded_sct() {
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

        let chain_pem = [r#"-----BEGIN CERTIFICATE-----
MIICGjCCAaGgAwIBAgIUALnViVfnU0brJasmRkHrn/UnfaQwCgYIKoZIzj0EAwMw
KjEVMBMGA1UEChMMc2lnc3RvcmUuZGV2MREwDwYDVQQDEwhzaWdzdG9yZTAeFw0y
MjA0MTMyMDA2MTVaFw0zMTEwMDUxMzU2NThaMDcxFTATBgNVBAoTDHNpZ3N0b3Jl
LmRldjEeMBwGA1UEAxMVc2lnc3RvcmUtaW50ZXJtZWRpYXRlMHYwEAYHKoZIzj0C
AQYFK4EEACIDYgAE8RVS/ysH+NOvuDZyPIZtilgUF9NlarYpAd9HP1vBBH1U5CV7
7LSS7s0ZiH4nE7Hv7ptS6LvvR/STk798LVgMzLlJ4HeIfF3tHSaexLcYpSASr1kS
0N/RgBJz/9jWCiXno3sweTAOBgNVHQ8BAf8EBAMCAQYwEwYDVR0lBAwwCgYIKwYB
BQUHAwMwEgYDVR0TAQH/BAgwBgEB/wIBADAdBgNVHQ4EFgQU39Ppz1YkEZb5qNjp
KFWixi4YZD8wHwYDVR0jBBgwFoAUWMAeX5FFpWapesyQoZMi0CrFxfowCgYIKoZI
zj0EAwMDZwAwZAIwPCsQK4DYiZYDPIaDi5HFKnfxXx6ASSVmERfsynYBiX2X6SJR
nZU84/9DZdnFvvxmAjBOt6QpBlc4J/0DxvkTCqpclvziL6BCCPnjdlIB3Pu3BxsP
mygUY7Ii2zbdCdliiow=
-----END CERTIFICATE-----"#];

        let ctfe_pem = r#"-----BEGIN PUBLIC KEY-----
MFkwEwYHKoZIzj0CAQYIKoZIzj0DAQcDQgAEiPSlFi0CmFTfEjCUqF9HuCEcYXNK
AaYalIJmBZ8yyezPjTqhxrKBpMnaocVtLJBI1eM3uXnQzQGAJdJ4gs9Fyw==
-----END PUBLIC KEY-----"#;

        let cert = Certificate::from_pem(cert_pem).unwrap();
        let issuer = Certificate::from_pem(chain_pem[0]).unwrap();
        let issuer_spki = issuer
            .tbs_certificate()
            .subject_public_key_info()
            .to_der()
            .unwrap();

        let sct = CertificateEmbeddedSCT::new_with_spki(&cert, &issuer_spki).unwrap();

        let ctfe_key: VerifyingKey = VerifyingKey::from_public_key_pem(ctfe_pem).unwrap();
        let keyring =
            super::super::keyring::Keyring::new([ctfe_key.to_public_key_der().unwrap().as_bytes()])
                .unwrap();

        assert!(verify_sct(&sct, &keyring).is_ok());
    }
}
