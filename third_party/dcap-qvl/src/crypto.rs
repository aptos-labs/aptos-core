//! Audited [`CryptoProvider`] implementations.
//!
//! [`RingCrypto`] (gated by `ring`) and [`RustCryptoCrypto`] (gated by
//! `rustcrypto`) are selected by [`crate::configs::RingConfig`] /
//! [`crate::configs::RustCryptoConfig`] respectively.

use crate::config::CryptoProvider;

/// Audited [`CryptoProvider`] backed by the `ring` crate (gated by the `ring`
/// feature). Selected by [`crate::configs::RingConfig`] /
/// [`crate::configs::DefaultConfig`].
#[cfg(feature = "ring")]
pub struct RingCrypto;

#[cfg(feature = "ring")]
impl CryptoProvider for RingCrypto {
    fn sig_algo() -> &'static dyn rustls_pki_types::SignatureVerificationAlgorithm {
        webpki::ring::ECDSA_P256_SHA256
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        let digest = ::ring::digest::digest(&::ring::digest::SHA256, data);
        let mut out = [0u8; 32];
        out.copy_from_slice(digest.as_ref());
        out
    }

    fn sha384(data: &[u8]) -> [u8; 48] {
        let digest = ring::digest::digest(&ring::digest::SHA384, data);
        let mut out = [0u8; 48];
        out.copy_from_slice(digest.as_ref());
        out
    }
}

/// Audited [`CryptoProvider`] backed by RustCrypto (`sha2` + `p256`, gated by
/// the `rustcrypto` feature). Selected by [`crate::configs::RustCryptoConfig`].
#[cfg(feature = "rustcrypto")]
pub struct RustCryptoCrypto;

#[cfg(feature = "rustcrypto")]
impl CryptoProvider for RustCryptoCrypto {
    fn sig_algo() -> &'static dyn rustls_pki_types::SignatureVerificationAlgorithm {
        webpki::rustcrypto::ECDSA_P256_SHA256
    }

    fn sha256(data: &[u8]) -> [u8; 32] {
        use sha2::Digest;
        sha2::Sha256::digest(data).into()
    }

    fn sha384(data: &[u8]) -> [u8; 48] {
        use sha2::Digest as _;
        sha2::Sha384::digest(data).into()
    }
}
