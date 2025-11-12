// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod transcript;

use crate::pvss::Player;
use aptos_crypto::traits;
pub use transcript::Transcript;

/// Converts a type `Self` to `ToType` using auxiliary data from type `AuxType`.
pub trait Convert<ToType, AuxType> {
    fn to(&self, with: &AuxType) -> ToType;
}

/// All PVSS public parameters must give access to the encryption public params.
pub trait HasEncryptionPublicParams {
    type EncryptionPublicParameters;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters;
}

/// All dealt secret keys should be reconstructable from a subset of \[dealt secret key\] shares.
/// TODO: Should we keep Vec<(Player, Self::Share)> ? Vec<ShamirShare> looks simpler / more descriptive
pub trait Reconstructable<SSC: traits::SecretSharingConfig> {
    type Share: Clone;

    fn reconstruct(sc: &SSC, shares: &Vec<(Player, Self::Share)>) -> Self;
}
