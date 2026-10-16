//! The crate's error type.
//!
//! Adapter for the vendored tinfoil-rs modules, which were written against
//! their parent crate's `crate::error::{Error, Result}`. Only the variants
//! those modules actually construct are provided; the vendored code carries its
//! own local enums (`KeyringError`, `SCTError`, `MerkleProofError`,
//! `ParseCheckpointError`) for its internal failure modes and converts them at
//! the boundary.

/// Everything that can go wrong verifying a Sigstore bundle.
///
/// Deliberately coarse. A verifier that reports *why* it refused a bundle in
/// fine detail is an oracle for what would be accepted, and callers here are
/// expected to collapse any failure into a single opaque denial anyway.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Sigstore verification failed. The string is for logs, never for clients.
    #[error("sigstore verification failed: {0}")]
    SigstoreVerification(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Build a [`Error::SigstoreVerification`] from anything displayable.
    pub fn verification(message: impl std::fmt::Display) -> Self {
        Self::SigstoreVerification(message.to_string())
    }
}
