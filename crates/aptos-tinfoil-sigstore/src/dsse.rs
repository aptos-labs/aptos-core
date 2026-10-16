//! DSSE (Dead Simple Signing Envelope) verification.
//!
//! This module handles DSSE envelope signature verification, including
//! certificate validation and SCT (Signed Certificate Timestamp) verification.

use der::Decode;
use p256::ecdsa::{Signature, VerifyingKey, signature::Verifier};
use x509_cert::Certificate;

use super::{certificate, fulcio, transparency, trust};
use crate::error::{Error, Result};
use crate::util::decode_b64;

/// Compute DSSE Pre-Authentication Encoding (PAE)
///
/// PAE(type, body) = "DSSEv1" + SP + LEN(type) + SP + type + SP + LEN(body) + SP + body
/// Where:
///   SP = ASCII space (0x20)
///   LEN(s) = ASCII decimal encoding of the byte length of s
pub fn compute_pae(payload_type: &str, payload: &[u8]) -> Vec<u8> {
    let type_bytes = payload_type.as_bytes();
    let type_len = type_bytes.len().to_string();
    let body_len = payload.len().to_string();

    let mut pae = Vec::new();
    pae.extend_from_slice(b"DSSEv1");
    pae.push(0x20); // SP
    pae.extend_from_slice(type_len.as_bytes());
    pae.push(0x20); // SP
    pae.extend_from_slice(type_bytes);
    pae.push(0x20); // SP
    pae.extend_from_slice(body_len.as_bytes());
    pae.push(0x20); // SP
    pae.extend_from_slice(payload);

    pae
}

/// Verify the DSSE envelope signature cryptographically
///
/// DSSE (Dead Simple Signing Envelope) verification:
/// 1. Extract certificate public key from bundle
/// 2. Validate certificate extensions (KeyUsage, ExtendedKeyUsage)
/// 3. Verify SCT (Signed Certificate Timestamp)
/// 4. Compute PAE (Pre-Authentication Encoding) of payload
/// 5. Verify ECDSA signature over PAE
#[cfg(test)]
pub fn verify_dsse_signature(bundle: &serde_json::Value) -> Result<()> {
    verify_dsse_signature_with_root(bundle, include_str!("../assets/trusted_root.json"))
}

/// Verify a DSSE bundle using only the caller-supplied Sigstore trust root.
pub fn verify_dsse_signature_with_root(
    bundle: &serde_json::Value,
    trusted_root_json: &str,
) -> Result<()> {
    // Get the certificate
    let cert_b64 = bundle
        .get("verificationMaterial")
        .and_then(|vm| vm.get("certificate"))
        .and_then(|c| c.get("rawBytes"))
        .and_then(|rb| rb.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No certificate in bundle".into()))?;

    let cert_der = decode_b64(cert_b64)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to decode certificate: {}", e)))?;

    // Parse certificate and extract public key
    let cert = Certificate::from_der(&cert_der)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to parse certificate: {}", e)))?;

    // Validate certificate has required extensions (KeyUsage: digitalSignature, ExtKeyUsage: codeSigning)
    certificate::validate_certificate_extensions(&cert)?;

    // Find the issuer certificate's SPKI for SCT verification
    let issuer_spki_der = fulcio::find_issuer_spki_with_root(&cert, trusted_root_json)?;

    // Verify Signed Certificate Timestamps (SCTs) with full cryptographic verification
    verify_sct_with_root(&cert_der, &issuer_spki_der, trusted_root_json)?;

    let pubkey_bytes = cert
        .tbs_certificate()
        .subject_public_key_info()
        .subject_public_key
        .raw_bytes();

    // The public key should be P-256 (secp256r1) - Fulcio uses this
    let verifying_key = VerifyingKey::from_sec1_bytes(pubkey_bytes)
        .map_err(|e| Error::SigstoreVerification(format!("Invalid public key: {}", e)))?;

    // Get DSSE envelope
    let dsse = bundle
        .get("dsseEnvelope")
        .ok_or_else(|| Error::SigstoreVerification("No dsseEnvelope in bundle".into()))?;

    let payload_type = dsse
        .get("payloadType")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No payloadType".into()))?;

    let payload_b64 = dsse
        .get("payload")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No payload".into()))?;

    let signatures = dsse
        .get("signatures")
        .and_then(|s| s.as_array())
        .ok_or_else(|| {
            Error::SigstoreVerification("No signatures array in DSSE envelope".into())
        })?;

    if signatures.len() != 1 {
        return Err(Error::SigstoreVerification(format!(
            "DSSE envelope must have exactly 1 signature, got {}",
            signatures.len()
        )));
    }

    let signature_b64 = signatures[0]
        .get("sig")
        .and_then(|s| s.as_str())
        .ok_or_else(|| Error::SigstoreVerification("No sig field in DSSE signature".into()))?;

    // Decode payload (it's base64 in the envelope)
    let payload = decode_b64(payload_b64)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to decode payload: {}", e)))?;

    // Compute PAE (Pre-Authentication Encoding)
    let pae = compute_pae(payload_type, &payload);

    // Decode signature - could be DER-encoded or raw
    let signature_bytes = decode_b64(signature_b64)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to decode signature: {}", e)))?;

    // Try DER format first (starts with 0x30), then raw format
    let signature = if signature_bytes.first() == Some(&0x30) {
        Signature::from_der(&signature_bytes)
            .map_err(|e| Error::SigstoreVerification(format!("Invalid DER signature: {}", e)))?
    } else {
        // Raw r||s format (64 bytes for P-256)
        Signature::from_slice(&signature_bytes)
            .map_err(|e| Error::SigstoreVerification(format!("Invalid raw signature: {}", e)))?
    };

    // Verify!
    verifying_key.verify(&pae, &signature).map_err(|e| {
        Error::SigstoreVerification(format!("DSSE signature verification failed: {}", e))
    })?;

    Ok(())
}

/// Verify Signed Certificate Timestamps (SCTs) embedded in the certificate.
///
/// Uses the sigstore-rs adapted transparency module for RFC 6962 compliant verification:
/// 1. Parses ALL SCTs from the certificate using x509-cert types
/// 2. For each SCT, reconstructs the PreCert (issuer key hash + TBS without SCT extension)
/// 3. Builds the digitally-signed struct with proper TLS encoding
/// 4. Verifies the ECDSA signature against the CT log's public key
/// 5. Checks that the CT log key was valid at the SCT's timestamp
/// 6. Requires at least one valid SCT from a known Sigstore CT log
///
/// All SCTs are tried (not just the first), so that if one SCT is from a
/// retired/expired CT log, a valid SCT from a current log can still succeed.
fn verify_sct_with_root(
    cert_der: &[u8],
    issuer_spki_der: &[u8],
    trusted_root_json: &str,
) -> Result<()> {
    let cert = Certificate::from_der(cert_der).map_err(|e| {
        Error::SigstoreVerification(format!("Failed to parse certificate for SCT: {}", e))
    })?;

    // Parse ALL SCTs from the certificate
    let all_scts = transparency::CertificateEmbeddedSCT::all_from_cert(&cert, issuer_spki_der)
        .map_err(|e| Error::SigstoreVerification(format!("Failed to extract SCTs: {}", e)))?;

    // Reject duplicate SCTs (same log ID) before verification.
    // Prevents inflating the verified count if a threshold > 1 is ever used.
    let mut seen_log_ids = std::collections::HashSet::new();
    for sct in &all_scts {
        if !seen_log_ids.insert(sct.log_id()) {
            return Err(Error::SigstoreVerification(
                "Duplicate SCT found (same log ID)".into(),
            ));
        }
    }

    // Load CT log keyring (with validity periods)
    let ct_keyring = trust::load_ctlog_keyring_from_json(trusted_root_json)?;

    // Try each SCT - succeed if at least one verifies against a valid CT log key
    let mut last_err = None;
    for sct in &all_scts {
        match transparency::verify_sct(sct, &ct_keyring) {
            Ok(()) => return Ok(()),
            Err(e) => last_err = Some(e),
        }
    }

    Err(Error::SigstoreVerification(format!(
        "No valid SCT found ({} SCTs checked). Last error: {}",
        all_scts.len(),
        last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "none".into())
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pae_encoding() {
        // Test vector from DSSE spec
        let payload_type = "http://example.com/HelloWorld";
        let payload = b"hello world";
        let pae = compute_pae(payload_type, payload);

        // Expected: "DSSEv1 29 http://example.com/HelloWorld 11 hello world"
        let expected = b"DSSEv1 29 http://example.com/HelloWorld 11 hello world";
        assert_eq!(pae, expected);
    }
}
