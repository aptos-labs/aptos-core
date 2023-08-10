// Copyright © Aptos Foundation

pub mod transcript;

use crate::pvss::player::Player;
use more_asserts::assert_lt;
use std::fmt::Display;

pub use transcript::Transcript;

/// Converts a type `Self` to `D` using auxiliary data from type `W`.
pub trait Convert<D, W> {
    fn to(&self, with: &W) -> D;
}

/// All PVSS public parameters must give access to the encryption public params.
pub trait HasEncryptionPublicParams {
    type EncryptionPublicParameters;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters;
}

/// A trait for keys that are secret-shareable and have an associated `Share` type.
pub trait IsSecretShareable {
    type Share: Clone;
}

pub trait SecretSharingConfig: Display {
    /// Creates a new player ID; a number from 0 to `n-1`, where `n = get_total_num_players(&self)`.
    fn get_player(&self, i: usize) -> Player {
        let n = self.get_total_num_players();
        assert_lt!(i, n);

        Player { id: i }
    }

    /// Returns a random subset of players who are capable of reconstructing the secret.
    /// Useful during testing.
    fn get_random_subset_of_capable_players<R>(&self, rng: &mut R) -> Vec<Player>
    where
        R: rand_core::RngCore + rand_core::CryptoRng;

    fn get_total_num_players(&self) -> usize;

    fn get_total_num_shares(&self) -> usize;
}

/// All dealt secret keys should be reconstructable from a subset of \[dealt secret key\] shares.
pub trait Reconstructable: IsSecretShareable {
    type SecretSharingConfig: SecretSharingConfig;

    fn reconstruct(sc: &Self::SecretSharingConfig, shares: &Vec<(Player, Self::Share)>) -> Self;
}
