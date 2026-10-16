//! # dcap-qvl
//!
//! Verify Intel SGX and TDX (DCAP — Data Center Attestation Primitives)
//! attestation quotes, in pure Rust. Supports both SGX (Software Guard
//! Extensions) and TDX (Trust Domain Extensions).
//!
//! # What it does
//! - Verify SGX and TDX quotes against Intel's trust chain
//! - Fetch collateral from a PCCS or Intel PCS, or verify fully offline
//! - Extract report fields (measurements, report data, TCB status) from a quote
//!
//! By default the collateral client uses Phala Network's PCCS
//! (`https://pccs.phala.network`).
//!
//! Native bindings for Python, JavaScript, Go, Kotlin, and Swift are published
//! from the same core — see the [project README][readme].
//!
//! [readme]: https://github.com/Phala-Network/dcap-qvl
//!
//! # Example
//!
//! ```no_run
//! use dcap_qvl::collateral::CollateralClient;
//! use dcap_qvl::verify::verify;
//! use dcap_qvl::PHALA_PCCS_URL;
//!
//! #[tokio::main]
//! async fn main() {
//!     let quote = std::fs::read("quote").expect("quote file not found");
//!
//!     // Use default Phala PCCS, or override with custom URL
//!     let pccs_url = std::env::var("PCCS_URL").unwrap_or_else(|_| PHALA_PCCS_URL.to_string());
//!     let collateral = CollateralClient::with_default_http(pccs_url)
//!         .expect("failed to build HTTP client")
//!         .fetch(&quote)
//!         .await
//!         .expect("failed to get collateral");
//!
//!     let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
//!     let report = verify(&quote, &collateral, now).expect("failed to verify quote");
//!     println!("{report:?}");
//! }
//! ```
//!
//! # Crypto backends
//!
//! Two backends are available: **ring** (optimized, uses assembly) and
//! **rustcrypto** (pure Rust). Both are enabled by default and `ring` takes
//! priority. For predictable behavior, call an explicit backend module:
//!
//! ```ignore
//! use dcap_qvl::verify::ring::verify;        // always ring
//! use dcap_qvl::verify::rustcrypto::verify;  // always rustcrypto
//! ```
//!
//! The top-level [`verify::verify`] selects the backend from enabled features:
//! `ring` wins when both are on, `rustcrypto` is used when only it is enabled,
//! and enabling neither is a compile error. Because Cargo features are additive,
//! any crate in your dependency tree that enables `ring` makes the top-level
//! `verify()` use ring — reach for the explicit modules to avoid surprises.
//!
//! # Feature flags
//!
//! ```toml
//! # Default: both backends, std, the PCCS collateral client, and x509 parsing.
//! dcap-qvl = "0.5"
//!
//! # Minimal verifier for WASM / on-chain (smaller, ring only, no_std-friendly):
//! dcap-qvl = { version = "0.5", default-features = false, features = ["std", "ring"] }
//! ```
//!
//! `no_std` builds are supported by disabling default features. The `report`
//! feature pulls in the async PCCS collateral client (`reqwest` + `tokio`); drop
//! it for offline verification on size-constrained targets.

#![cfg_attr(all(not(test), not(feature = "std")), no_std)]

#[macro_use]
extern crate alloc;

use scale::{Decode, Encode};
use serde::{Deserialize, Serialize};

#[cfg(feature = "borsh_schema")]
use borsh::BorshSchema;
#[cfg(feature = "borsh")]
use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Encode, Decode, Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "borsh", derive(BorshSerialize, BorshDeserialize))]
#[cfg_attr(feature = "borsh_schema", derive(BorshSchema))]
pub struct QuoteCollateralV3 {
    pub pck_crl_issuer_chain: String,
    #[serde(with = "serde_bytes")]
    pub root_ca_crl: Vec<u8>,
    #[serde(with = "serde_bytes")]
    pub pck_crl: Vec<u8>,
    pub tcb_info_issuer_chain: String,
    pub tcb_info: String,
    #[serde(with = "serde_bytes")]
    pub tcb_info_signature: Vec<u8>,
    pub qe_identity_issuer_chain: String,
    pub qe_identity: String,
    #[serde(with = "serde_bytes")]
    pub qe_identity_signature: Vec<u8>,
    /// PCK certificate chain (PEM format).
    /// For cert_type 5: extracted from quote during collateral fetch.
    /// For cert_type 3: fetched from PCCS using encrypted PPID.
    /// Used by verify() for offline verification.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pck_certificate_chain: Option<String>,
}

impl From<&QuoteCollateralV3> for QuoteCollateralV3 {
    fn from(value: &QuoteCollateralV3) -> Self {
        value.clone()
    }
}

#[cfg(feature = "report")]
pub mod collateral;

#[cfg(feature = "report")]
pub use collateral::PHALA_PCCS_URL;

#[cfg(feature = "report")]
pub mod http;

pub mod config;
#[cfg(feature = "default-x509")]
pub mod configs;
pub mod crypto;
pub mod oids;
#[cfg(feature = "default-x509")]
pub mod signature;
#[cfg(feature = "default-x509")]
pub mod x509;

mod constants;
pub mod intel;
pub mod policy;
pub mod qe_identity;
pub mod tcb_info;
mod utils;

pub use constants::{CpuSvn, Fmspc, MrEnclave, MrSigner, Svn, INTEL_QE_VENDOR_ID};
pub use policy::{
    PckCertFlag, PckIdentity, PlatformInfo, Policy, QeInfo, QuoteClaims, QuotePolicy,
    QuotePolicyConfig, TcbVerdict,
};
pub use qe_identity::{QeIdentity, QeTcb, QeTcbLevel};
pub use tcb_info::{Tcb, TcbComponents, TcbInfo, TcbLevel, TcbStatus, TcbStatusWithAdvisory};

pub mod quote;
pub mod verify;

#[cfg(feature = "python")]
pub mod python;

#[cfg(feature = "go")]
mod ffi;

#[cfg(feature = "python")]
use pyo3::prelude::*;

#[cfg(feature = "python")]
#[pymodule]
fn _dcap_qvl(m: &Bound<'_, PyModule>) -> PyResult<()> {
    python::register_module(m)
}
