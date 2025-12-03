// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

pub mod transcript;

pub use aptos_crypto::arkworks::shamir::Reconstructable;
pub use transcript::{AggregatableTranscript, Transcript};

/// Converts a type `Self` to `ToType` using auxiliary data from type `AuxType`.
pub trait Convert<ToType, AuxType> {
    fn to(&self, with: &AuxType) -> ToType;
}

/// All PVSS public parameters must give access to the encryption public params.
pub trait HasEncryptionPublicParams {
    type EncryptionPublicParameters;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters;
}
