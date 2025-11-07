// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

pub mod transcript;

use crate::pvss::Player;
use aptos_crypto::arkworks;
use more_asserts::assert_lt;
use rand::{seq::IteratorRandom, Rng};
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

/// Trait for secret sharing schemes that expose a threshold `t`.
///
/// This trait is required because some operations (such as those in SCRAPE LDT) need access to `t`,
/// but not all secret sharing schemes are threshold secret sharing schemes; they can have more
/// general access structures.
pub trait ThresholdConfig: SecretSharingConfig + Sized {
    fn new(t: usize, n: usize) -> anyhow::Result<Self>;

    fn get_threshold(&self) -> usize;
}

impl<F: ark_ff::PrimeField> SecretSharingConfig for arkworks::shamir::ThresholdConfig<F> {
    /// For testing only.
    fn get_random_player<R>(&self, rng: &mut R) -> Player
    where
        R: rand_core::RngCore + rand_core::CryptoRng,
    {
        Player {
            id: rng.gen_range(0, self.n),
        }
    }

    /// For testing only.
    fn get_random_eligible_subset_of_players<R>(&self, mut rng: &mut R) -> Vec<Player>
    where
        R: rand_core::RngCore,
    {
        (0..self.get_total_num_shares())
            .choose_multiple(&mut rng, self.t)
            .into_iter()
            .map(|i| self.get_player(i))
            .collect::<Vec<Player>>()
    }

    fn get_total_num_players(&self) -> usize {
        self.n
    }

    fn get_total_num_shares(&self) -> usize {
        self.n
    }
}

/// All dealt secret keys should be reconstructable from a subset of \[dealt secret key\] shares.
/// TODO: Should we keep Vec<(Player, Self::Share)> ? Vec<ShamirShare> looks simpler / more descriptive
pub trait Reconstructable<SSC: SecretSharingConfig> {
    type Share: Clone;

    fn reconstruct(sc: &SSC, shares: &Vec<(Player, Self::Share)>) -> Self;
}
