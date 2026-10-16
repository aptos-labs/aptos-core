//! Small helpers the vendored modules expect from their parent crate.

use crate::error::{Error, Result};
use base64::Engine;

/// Decode standard base64.
///
/// Sigstore bundles are JSON with base64-encoded binary members. Upstream
/// decodes these with a permissive engine, and this keeps that behaviour: the
/// bytes are re-verified cryptographically downstream, so a canonicality check
/// here would reject bundles that legitimate signers produce without adding any
/// security.
///
/// This differs on purpose from the *attestation envelope*, where canonical
/// base64 is required — there the encoded bytes are hashed, so two spellings of
/// one byte string would be two different hashes.
pub fn decode_b64(value: &str) -> Result<Vec<u8>> {
    base64::engine::general_purpose::STANDARD
        .decode(value)
        .map_err(|err| Error::SigstoreVerification(format!("invalid base64: {err}")))
}
