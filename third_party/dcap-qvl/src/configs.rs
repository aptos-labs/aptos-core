//! Preset [`Config`] bundles built from the audited in-tree backends.
//!
//! Each bundle pairs the audited X.509 parser ([`X509CertBackend`]) and
//! signature encoder ([`DerSigEncoder`]) with one of the available
//! [`CryptoProvider`](crate::config::CryptoProvider) implementations.
//! [`DefaultConfig`] is a feature-selected type alias picking the right
//! preset based on which crypto feature is enabled.

use crate::config::Config;
use crate::signature::DerSigEncoder;
use crate::x509::X509CertBackend;

#[cfg(feature = "ring")]
use crate::crypto::RingCrypto;
#[cfg(feature = "rustcrypto")]
use crate::crypto::RustCryptoCrypto;

/// Audited config pairing the `der` / `x509-cert` parser with the `ring`
/// crypto provider. Selected as [`DefaultConfig`] when the `ring` feature is
/// enabled.
#[cfg(feature = "ring")]
pub struct RingConfig;

#[cfg(feature = "ring")]
impl Config for RingConfig {
    type X509 = X509CertBackend;
    type SigEncoder = DerSigEncoder;
    type Crypto = RingCrypto;
}

/// Audited config pairing the `der` / `x509-cert` parser with the RustCrypto
/// crypto provider.
#[cfg(feature = "rustcrypto")]
pub struct RustCryptoConfig;

#[cfg(feature = "rustcrypto")]
impl Config for RustCryptoConfig {
    type X509 = X509CertBackend;
    type SigEncoder = DerSigEncoder;
    type Crypto = RustCryptoCrypto;
}

/// Audited default config: prefers [`RingConfig`] if the `ring` feature is on,
/// otherwise [`RustCryptoConfig`].
#[cfg(feature = "ring")]
pub type DefaultConfig = RingConfig;

#[cfg(all(not(feature = "ring"), feature = "rustcrypto"))]
pub type DefaultConfig = RustCryptoConfig;
