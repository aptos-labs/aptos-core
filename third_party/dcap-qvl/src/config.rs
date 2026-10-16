//! Pluggable configuration trait surface for quote verification.
//!
//! Quote verification depends on a small set of operations on certificates,
//! signatures, and crypto primitives. Each is captured by a small trait so a
//! downstream consumer (e.g. a smart contract building for WASM) can supply a
//! smaller-footprint implementation without forking this crate. The audited
//! in-tree implementations live in [`crate::x509`], [`crate::signature`], and
//! [`crate::crypto`]; the bundles in [`crate::configs`] (incl.
//! [`DefaultConfig`](crate::configs::DefaultConfig)) pair them into ready-to-use
//! [`Config`]s.
//!
//! ## Config trait
//!
//! [`Config`] bundles the pluggable components as associated types so they can
//! be swapped independently — e.g. keep the audited cert parser but bring your
//! own micro-encoder. A custom config is just a marker type implementing
//! [`Config`]:
//!
//! ```ignore
//! struct MyConfig;
//! impl dcap_qvl::config::Config for MyConfig {
//!     type X509       = dcap_qvl::x509::X509CertBackend; // keep audited
//!     type SigEncoder = MyMicroEncoder;                  // custom
//!     type Crypto     = dcap_qvl::crypto::RingCrypto;
//! }
//!
//! dcap_qvl::verify::QuoteVerifier::<MyConfig>::new_with_config(root_ca_der)
//!     .verify(quote, &collateral, now)?;
//! ```
//!
//! ## Auditing
//!
//! Only the in-tree backends ([`crate::x509::X509CertBackend`],
//! [`crate::signature::DerSigEncoder`], [`crate::crypto::RingCrypto`] /
//! [`crate::crypto::RustCryptoCrypto`]) and the bundles in [`crate::configs`]
//! are audited as part of `dcap-qvl`. Custom implementations of [`X509Codec`]
//! / [`ParsedCert`] / [`EcdsaSigEncoder`] / [`CryptoProvider`] are the
//! implementer's responsibility; they SHOULD be checked for byte-for-byte
//! equivalence against [`DefaultConfig`](crate::configs::DefaultConfig) on a
//! representative corpus (see `tests/config_conformance.rs`).
//!
//! ## Trait shape
//!
//! Sub-trait methods (other than [`X509Codec::from_der`]) are associated
//! functions, not `&self` methods, so backends are expected to be zero-sized
//! marker types. [`X509Codec`] is the exception: it produces a [`ParsedCert`]
//! that owns or borrows the parsed data, so multiple field accesses share a
//! single parse.

use alloc::vec::Vec;
use anyhow::Result;

/// X.509 certificate parser factory.
///
/// Implementations MUST conform to RFC 5280 and X.690 (DER). A backend
/// parses a certificate once via [`X509Codec::from_der`], returning a
/// [`ParsedCert`] that exposes the fields needed by quote verification as
/// `&self` accessors — so a caller that touches multiple fields of the same
/// certificate pays the parsing cost only once.
///
/// The associated type [`X509Codec::Parsed`] is generic over the input
/// lifetime, allowing implementations to choose whether to own their parsed
/// representation or borrow zero-copy from the input DER bytes.
pub trait X509Codec {
    /// Parsed representation. May own its data, or may borrow from the input
    /// `cert_der` slice (zero-copy backends).
    type Parsed<'a>: ParsedCert
    where
        Self: 'a;

    /// Parse a DER-encoded X.509 certificate.
    fn from_der<'a>(cert_der: &'a [u8]) -> Result<Self::Parsed<'a>>;
}

/// Intel PCK certificate authority that issued a leaf cert.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PckCa {
    Processor,
    Platform,
}

impl PckCa {
    /// Lowercase identifier used in PCS URLs and FFI bindings
    /// (`"processor"` / `"platform"`).
    pub fn as_id_str(&self) -> &'static str {
        match self {
            PckCa::Processor => crate::constants::PROCESSOR_ISSUER_ID,
            PckCa::Platform => crate::constants::PLATFORM_ISSUER_ID,
        }
    }
}

/// Read-only accessors over a parsed X.509 certificate.
pub trait ParsedCert {
    /// Classify the issuer of an Intel PCK leaf certificate.
    ///
    /// Returns `None` when the issuer matches neither CA, or when the
    /// backend cannot interpret the issuer DN. Callers decide how to
    /// handle `None`.
    fn pck_ca(&self) -> Option<PckCa>;

    /// Returns the OCTET STRING contents of the unique extension whose
    /// `extnID` equals `oid`, where `oid` is the DER-encoded OID body
    /// (no tag/length). Callers SHOULD construct `oid` from a
    /// [`const_oid::ObjectIdentifier`] and pass `.as_bytes()`.
    ///
    /// * `Ok(Some(value))` — exactly one matching extension was found.
    /// * `Ok(None)` — no matching extension, *including* when `oid` is not a
    ///   well-formed OID body: a malformed needle cannot equal any cert's
    ///   `extnID` (which was DER-validated during [`X509Codec::from_der`]),
    ///   so "not found" is vacuously correct. Implementations are not
    ///   required to reject malformed `oid` inputs — runtime validation
    ///   would be pure overhead for callers that supply
    ///   `const_oid`-constructed OIDs.
    /// * `Err(_)` — the extension was found more than once, or the certificate
    ///   is malformed.
    fn extension(&self, oid: &[u8]) -> Result<Option<Vec<u8>>>;
}

/// DER encoding of ECDSA signatures.
///
/// DCAP quotes carry signatures as raw `r ‖ s` byte strings, but webpki's
/// signature verifier expects them encoded as `Ecdsa-Sig-Value` (RFC 5480):
/// `SEQUENCE { r INTEGER, s INTEGER }`. This trait performs that conversion.
pub trait EcdsaSigEncoder {
    /// Encodes `(r, s)` as DER `SEQUENCE { INTEGER r, INTEGER s }`.
    ///
    /// Each component is treated as an unsigned big-endian magnitude:
    /// implementations MUST strip leading zero bytes and prepend a single
    /// `0x00` when the high bit of the leading byte would otherwise make the
    /// integer negative under DER's signed-integer rules.
    fn encode_ecdsa_sig(r: &[u8], s: &[u8]) -> Result<Vec<u8>>;
}

/// Cryptographic primitives required by quote verification: the ECDSA
/// signature verification algorithm passed to webpki, plus SHA-256 and
/// SHA-384 hashes.
///
/// Implementations are typically zero-sized marker types whose methods are
/// associated functions delegating to a chosen crypto crate (ring, RustCrypto,
/// hardware accelerator, etc.).
pub trait CryptoProvider {
    /// ECDSA P-256 SHA-256 algorithm passed to webpki's signature verifier.
    fn sig_algo() -> &'static dyn rustls_pki_types::SignatureVerificationAlgorithm;
    /// SHA-256 of `data`.
    fn sha256(data: &[u8]) -> [u8; 32];
    /// SHA-384 of `data`, used for Intel QAL `root_key_id`.
    fn sha384(data: &[u8]) -> [u8; 48];
}

/// Configuration bundle selecting an implementation for each pluggable
/// component.
///
/// Implementations are typically zero-sized marker types. Adding a new
/// pluggable component in the future means adding a new associated type here,
/// which is backwards-compatible (existing custom configs continue to work as
/// long as they implement the new associated type — defaulted with a `where`
/// clause if needed).
pub trait Config {
    /// X.509 certificate parser.
    type X509: X509Codec;
    /// ECDSA `r ‖ s` → DER `Ecdsa-Sig-Value` encoder.
    type SigEncoder: EcdsaSigEncoder;
    /// Cryptographic primitives (ECDSA signature verification + SHA-256).
    type Crypto: CryptoProvider;
}
