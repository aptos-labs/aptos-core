//! Fulcio CA chain verification.
//!
//! This module verifies that signing certificates were issued by a trusted
//! Fulcio Certificate Authority from the Sigstore public-good instance.

use super::trust;
use crate::error::{Error, Result};
use der::{Decode, Encode};
use x509_cert::Certificate;

/// OID for P-256 curve (secp256r1): 1.2.840.10045.3.1.7
const OID_SECP256R1: &str = "1.2.840.10045.3.1.7";
/// OID for P-384 curve (secp384r1): 1.3.132.0.34
const OID_SECP384R1: &str = "1.3.132.0.34";

/// Find the issuer certificate's SPKI for a given certificate.
///
/// This searches through the trusted Fulcio CAs to find the one that
/// issued the given certificate (by matching issuer/subject DNs).
#[cfg(test)]
pub fn find_issuer_spki(cert: &Certificate) -> Result<Vec<u8>> {
    find_issuer_spki_with_root(cert, include_str!("../assets/trusted_root.json"))
}

pub fn find_issuer_spki_with_root(cert: &Certificate, trusted_root_json: &str) -> Result<Vec<u8>> {
    let fulcio_cas = trust::load_fulcio_cas_from_json(trusted_root_json)?;

    // Try each Fulcio CA to find the matching issuer
    for ca in &fulcio_cas {
        if ca.cert_chain_der.is_empty() {
            continue;
        }

        let issuer_cert = match Certificate::from_der(&ca.cert_chain_der[0]) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // Check if issuer DN matches
        if cert.tbs_certificate().issuer() == issuer_cert.tbs_certificate().subject() {
            // Return the issuer's SPKI in DER format
            return issuer_cert
                .tbs_certificate()
                .subject_public_key_info()
                .to_der()
                .map_err(|e| {
                    Error::SigstoreVerification(format!("Failed to encode issuer SPKI: {}", e))
                });
        }
    }

    Err(Error::SigstoreVerification(
        "Could not find issuer certificate for SCT verification".into(),
    ))
}

/// Verify that the signing certificate was issued by a trusted Fulcio CA.
///
/// This walks the full embedded Fulcio chain, not just the nearest issuer:
/// 1. The CA was valid at the time the signing cert was issued.
/// 2. The signing cert's issuer matches the first chain cert's subject.
/// 3. Each `chain[i]` is signed by `chain[i+1]` (issuer/subject + signature).
/// 4. The last cert in the chain is a self-signed root (subject == issuer,
///    self-signature verifies).
#[cfg(test)]
pub fn verify_fulcio_chain(cert_der: &[u8], cert_not_before: u64) -> Result<()> {
    verify_fulcio_chain_with_root(
        cert_der,
        cert_not_before,
        include_str!("../assets/trusted_root.json"),
    )
}

pub fn verify_fulcio_chain_with_root(
    cert_der: &[u8],
    cert_not_before: u64,
    trusted_root_json: &str,
) -> Result<()> {
    let signing_cert = Certificate::from_der(cert_der).map_err(|e| {
        Error::SigstoreVerification(format!("Failed to parse signing certificate: {}", e))
    })?;

    let fulcio_cas = trust::load_fulcio_cas_from_json(trusted_root_json)?;

    for ca in &fulcio_cas {
        if cert_not_before < ca.valid_from {
            continue;
        }
        if let Some(end) = ca.valid_until {
            if cert_not_before > end {
                continue;
            }
        }
        if ca.cert_chain_der.is_empty() {
            continue;
        }

        match verify_against_chain(&signing_cert, &ca.cert_chain_der) {
            Ok(()) => return Ok(()),
            Err(_) => continue,
        }
    }

    Err(Error::SigstoreVerification(
        "Certificate not issued by any trusted Fulcio CA".into(),
    ))
}

/// Verify `leaf` against a Fulcio-style chain `[intermediate(s)..., root]`.
fn verify_against_chain(leaf: &Certificate, chain_der: &[Vec<u8>]) -> Result<()> {
    // Parse all chain certificates up front.
    let chain_certs: Vec<Certificate> = chain_der
        .iter()
        .map(|der| {
            Certificate::from_der(der).map_err(|e| {
                Error::SigstoreVerification(format!("Failed to parse Fulcio chain cert: {}", e))
            })
        })
        .collect::<Result<_>>()?;

    // Step 1: leaf signed by chain[0].
    verify_issued_by(leaf, &chain_certs[0], "signing certificate")?;

    // Step 2: walk the chain.
    for i in 0..chain_certs.len() - 1 {
        verify_issued_by(
            &chain_certs[i],
            &chain_certs[i + 1],
            "Fulcio chain certificate",
        )?;
    }

    // Step 3: final cert must be a self-signed root.
    let root = chain_certs.last().unwrap();
    if root.tbs_certificate().subject() != root.tbs_certificate().issuer() {
        return Err(Error::SigstoreVerification(
            "Fulcio chain does not end in a self-signed root".into(),
        ));
    }
    verify_issued_by(root, root, "Fulcio root certificate")?;

    Ok(())
}

/// Verify that `child` was signed by `issuer` (DN match + cryptographic signature).
fn verify_issued_by(child: &Certificate, issuer: &Certificate, context: &str) -> Result<()> {
    if child.tbs_certificate().issuer() != issuer.tbs_certificate().subject() {
        return Err(Error::SigstoreVerification(format!(
            "{} issuer DN does not match issuer subject",
            context
        )));
    }

    let tbs_bytes = child
        .tbs_certificate()
        .to_der()
        .map_err(|e| Error::SigstoreVerification(format!("Failed to encode TBS: {}", e)))?;
    let sig_bytes = child.signature().raw_bytes();

    let issuer_spki = issuer.tbs_certificate().subject_public_key_info();
    let issuer_pubkey_bytes = issuer_spki.subject_public_key.raw_bytes();

    let curve_oid = issuer_spki
        .algorithm
        .parameters
        .as_ref()
        .and_then(|p| p.decode_as::<der::asn1::ObjectIdentifier>().ok())
        .map(|oid| oid.to_string());

    let verified = match curve_oid.as_deref() {
        Some(OID_SECP256R1) => verify_ecdsa_p256(&tbs_bytes, sig_bytes, issuer_pubkey_bytes),
        Some(OID_SECP384R1) => verify_ecdsa_p384(&tbs_bytes, sig_bytes, issuer_pubkey_bytes),
        _ => {
            return Err(Error::SigstoreVerification(format!(
                "{} issuer uses unsupported curve",
                context
            )));
        },
    };

    if !verified {
        return Err(Error::SigstoreVerification(format!(
            "{} signature did not verify against issuer",
            context
        )));
    }
    Ok(())
}

/// Verify ECDSA P-256 signature over TBS certificate bytes.
fn verify_ecdsa_p256(tbs_bytes: &[u8], sig_bytes: &[u8], pubkey_bytes: &[u8]) -> bool {
    use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};

    let Some(key) = VerifyingKey::from_sec1_bytes(pubkey_bytes).ok() else {
        return false;
    };
    let Some(sig) = Signature::from_der(sig_bytes).ok() else {
        return false;
    };
    key.verify(tbs_bytes, &sig).is_ok()
}

/// Verify ECDSA P-384 signature over TBS certificate bytes.
fn verify_ecdsa_p384(tbs_bytes: &[u8], sig_bytes: &[u8], pubkey_bytes: &[u8]) -> bool {
    ring::signature::UnparsedPublicKey::new(&ring::signature::ECDSA_P384_SHA384_ASN1, pubkey_bytes)
        .verify(tbs_bytes, sig_bytes)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AMD Genoa ASK certificate - a valid certificate that is definitely not
    /// issued by any Fulcio CA, so it should trigger the "not found" error.
    fn get_non_fulcio_certificate_der() -> Vec<u8> {
        let pem = r#"-----BEGIN CERTIFICATE-----
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
-----END CERTIFICATE-----"#;
        let parsed = pem::parse(pem).expect("Failed to parse PEM");
        parsed.into_contents()
    }

    #[test]
    fn test_find_issuer_spki_not_found() {
        let cert_der = get_non_fulcio_certificate_der();
        let cert = Certificate::from_der(&cert_der).expect("Failed to parse certificate");

        let result = find_issuer_spki(&cert);

        assert!(
            result.is_err(),
            "Non-Fulcio certificate should not find issuer"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("Could not find issuer certificate"),
            "Error should mention missing issuer, got: {}",
            err_msg
        );
    }

    #[test]
    fn test_verify_fulcio_chain_not_issued_by_fulcio() {
        let cert_der = get_non_fulcio_certificate_der();

        // Use a timestamp within the certificate's validity period
        let cert_not_before = 1667222028; // 2022-10-31, when the cert was issued

        let result = verify_fulcio_chain(&cert_der, cert_not_before);

        assert!(
            result.is_err(),
            "Non-Fulcio certificate should fail verification"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("not issued by any trusted Fulcio CA"),
            "Error should mention untrusted CA, got: {}",
            err_msg
        );
    }
}
