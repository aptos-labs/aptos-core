// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod transcript;

use crate::pvss::player::Player;
use more_asserts::assert_lt;
use std::fmt::Display;
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

pub trait SecretSharingConfig: Display {
    /// Creates a new player ID; a number from 0 to `n-1`, where `n = get_total_num_players(&self)`.
    fn get_player(&self, i: usize) -> Player {
        let n = self.get_total_num_players();
        assert_lt!(i, n);

        Player { id: i }
    }

    /// Useful during testing.
    fn get_random_player<R>(&self, rng: &mut R) -> Player
    where
        R: rand_core::RngCore + rand_core::CryptoRng;

    /// Returns a random subset of players who are capable of reconstructing the secret.
    /// Useful during testing.
    fn get_random_eligible_subset_of_players<R>(&self, rng: &mut R) -> Vec<Player>
    where
        R: rand_core::RngCore;

    fn get_total_num_players(&self) -> usize;

    fn get_total_num_shares(&self) -> usize;
}

/// All dealt secret keys should be reconstructable from a subset of \[dealt secret key\] shares.
pub trait Reconstructable<SSC: SecretSharingConfig> {
    type Share: Clone;

    fn reconstruct(sc: &SSC, shares: &Vec<(Player, Self::Share)>) -> Self;
}
