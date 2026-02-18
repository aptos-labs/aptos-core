// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use crate::shared::ids::Id;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BatchEncryptionError {
    #[error("Error during digest key initialization: {0}")]
    DigestInitError(DigestKeyInitError),
    #[error("Tried to setup w/ happy path MPK that doesn't match slow path MPK")]
    HappySlowPathMismatchError,
    #[error("Tried to setup w/ VK that does not match MSK share")]
    VKMSKMismatchError,
    #[error("Serialization error")]
    SerializationError,
    #[error("Deserialization error")]
    DeserializationError,
    #[error("Symmetric encryption error")]
    SymmetricEncryptionError,
    #[error("Symmetric decryption error")]
    SymmetricDecryptionError,
    #[error("Could not initialize an FFT domain of the appropriate size")]
    FFTDomainError,
    #[error("Error when verifying ciphertext: {0}")]
    CTVerifyError(CTVerifyError),
    #[error("Error when verifying eval proof")]
    EvalProofVerifyError,
    #[error("Decryption key share verification error")]
    DecryptionKeyShareVerifyError,
    #[error("Decryption key verification error")]
    DecryptionKeyVerifyError,
    #[error("Tried to compute eval proofs for an id set whose coefficients weren't computed yet")]
    EvalProofsWithUncomputedCoefficients,
    #[error("Hash2Curve failed: couldn't find a quadratic residue, or couldn't map to subgroup")]
    Hash2CurveFailure,
}

#[derive(Debug, Error)]
#[error("Tried to decrypt a ciphertext whose eval proof wasn't yet computed")]
pub struct MissingEvalProofError(pub Id);

#[derive(Debug, Error)]
pub enum CTVerifyError {
    #[error("The ID of the ciphertext does not match the hashed verification key")]
    IdDoesNotMatchHashedVK,
    #[error(
        "The associated data of the CT does not match what was input to the verification function"
    )]
    AssociatedDataDoesNotMatch,
    #[error("Signature failed to verify: {0}")]
    SigVerificationFailed(ed25519_dalek::SignatureError),
}

#[derive(Debug, Error)]
pub enum ReconstructError {
    #[error("Tried to reconstruct with number of shares != t")]
    ReconstructImproperNumShares,
    #[error("Tried to reconstruct decryption key shares with mismatching digests")]
    ReconstructDigestsDontMatch,
}

#[derive(Debug, Error)]
pub enum DigestKeyInitError {
    #[error(
        "Tried to compute a digest key w/ a batch size not a power of 2, which is unsupported."
    )]
    BatchSizeMustBePowerOfTwo,
    #[error("Failed to initialize FK domain")]
    FKDomainInitFailure,
}
