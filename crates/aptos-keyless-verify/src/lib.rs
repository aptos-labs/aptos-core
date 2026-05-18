// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Standalone keyless signature verification for off-chain consumers.
//!
//! The intent of this crate is to give downstream services (key-recovery
//! services, oracles, indexers, application-specific worker components, …) a
//! way to verify Aptos keyless signatures without depending on `aptos-types`
//! or `aptos-crypto`, both of which transitively pull in the full aptos-core
//! workspace build environment (forked `merlin`, `tokio_unstable` cfg, the
//! `slh-dsa` / `pkcs8` pre-release pin, `aptos-dkg`, `aptos-runtimes`, …).
//!
//! ## What this crate offers
//!
//! - Self-contained types ([`KeylessPublicKey`], [`KeylessSignature`],
//!   [`EphemeralCertificate`], [`RsaJwk`], [`Groth16VerificationKey`],
//!   [`Configuration`], …) that share BCS wire format with their counterparts
//!   in `aptos-types`.
//! - A single pure-function entry point [`verify_keyless`] that takes the
//!   parsed types plus the chain data the caller is responsible for fetching
//!   (the relevant [`RsaJwk`], the [`Groth16VerificationKey`], the
//!   [`Configuration`]) and a wall-clock timestamp.
//! - No network I/O, no async, no Move-VM types, no system clock — everything
//!   needed at runtime is taken by reference.
//!
//! ## What this crate does *not* do
//!
//! - It does not fetch JWKs, verification keys, or account auth keys from a
//!   fullnode; the caller is expected to bring those.
//! - It does not interact with the pepper service or the prover service; this
//!   crate verifies an already-produced signature.
//! - It does not perform account-address-binding checks (computing the
//!   keyless auth key and comparing against the on-chain authentication key
//!   is the caller's responsibility — but
//!   [`KeylessPublicKey::account_authentication_key`] is provided to make
//!   that easy).
//!
//! ## Relationship with `aptos-types`
//!
//! In this initial PR the crate is *additive*: `aptos-types` and
//! `aptos-crypto` still own the existing keyless code. A follow-up PR will
//! migrate `aptos-types::keyless` to re-export from this crate so there is a
//! single source of truth. Until then, BCS wire-format compatibility is the
//! contract: signatures produced by `aptos-types::keyless::KeylessSignature`
//! deserialize cleanly into [`KeylessSignature`] in this crate, and the
//! converse.

pub mod any;
pub mod bn254_circom;
pub mod configuration;
pub mod ephemeral;
pub mod errors;
pub mod groth16_sig;
pub mod groth16_vk;
pub mod jwk;
pub mod openid_sig;
pub mod public_key;
pub mod signature;
pub mod verify;
pub mod zkp_sig;

#[cfg(feature = "test-utils")]
pub mod test_utils;

pub use any::{AnyPublicKey, AnySignature};
pub use bn254_circom::{G1Bytes, G2Bytes};
pub use configuration::Configuration;
pub use ephemeral::{EphemeralPublicKey, EphemeralSignature};
pub use errors::VerifyError;
pub use groth16_sig::{Groth16Proof, ZeroKnowledgeSig};
pub use groth16_vk::Groth16VerificationKey;
pub use jwk::RsaJwk;
pub use openid_sig::{Claims, OidcClaims, OpenIdSig};
pub use public_key::{IdCommitment, KeylessPublicKey, Pepper};
pub use signature::{EphemeralCertificate, JwtHeader, KeylessSignature};
pub use verify::verify_keyless;
pub use zkp_sig::{ZkProof, ZkpVariant};
