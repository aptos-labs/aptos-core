// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::Uniform;
use aptos_crypto_derive::{SilentDebug, SilentDisplay};
use ark_ec::{pairing::Pairing, AdditiveGroup};
use ark_std::rand::{CryptoRng, RngCore};
/// The *input secret* that will be given as input to the PVSS dealing algorithm. This will be of a
/// different type than the *dealt secret* that will be returned by the PVSS reconstruction algorithm.
///
/// This secret will NOT need to be stored by validators because a validator (1) picks such a secret
/// and (2) deals it via the PVSS. If the validator crashes during dealing, the entire task will be
/// restarted with a freshly-generated input secret.
///
use derive_more::{Add, Display, From, Into};
use ff::Field;
use num_traits::Zero;
use std::ops::{Add, AddAssign};

#[derive(SilentDebug, SilentDisplay, PartialEq, Add)]
pub struct InputSecret<F: ark_ff::Field> {
    /// The actual secret being dealt; a scalar $a \in F$.
    a: F,
}

impl<F: ark_ff::Field> Uniform for InputSecret<F> {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + rand::CryptoRng,
    {
        Self {
            a: F::rand(&mut ark_std::rand::thread_rng()),
        } // Workaround because rand versions differ
    }
}

impl<F: ark_ff::Field> AddAssign<&InputSecret<F>> for InputSecret<F> {
    fn add_assign(&mut self, other: &InputSecret<F>) {
        self.a += other.a;
    }
}

impl<F: ark_ff::Field> Zero for InputSecret<F> {
    fn zero() -> Self {
        InputSecret { a: F::ZERO }
    }

    fn is_zero(&self) -> bool {
        self.a.is_zero()
    }
}

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(InputSecret: Clone);

impl<F: ark_ff::Field> InputSecret<F> {
    pub fn get_secret_a(&self) -> &F {
        &self.a
    }
}

impl<F: ark_ff::Field> ark_std::UniformRand for InputSecret<F> {
    fn rand<R: RngCore + ?Sized>(rng: &mut R) -> Self {
        InputSecret { a: F::rand(rng) }
    }
}

// impl traits::Convert<Scalar, chunked_elgamal_field::PublicParameters> for InputSecret {
//     fn to(&self, _with: &chunked_elgamal_field::PublicParameters) -> Scalar {
//         *self.get_secret_a()
//     }
// }

// impl traits::Convert<DealtPubKey, chunked_elgamal_field::PublicParameters> for InputSecret {
//     /// Computes the public key associated with the given input secret.
//     /// NOTE: In the SCRAPE PVSS, a `DealtPublicKey` cannot be computed from a `DealtSecretKey` directly.
//     fn to(&self, pp: &chunked_elgamal_field::PublicParameters) -> DealtPubKey {
//         DealtPubKey::new(pp.get_commitment_base().mul(self.get_secret_a()))
//     }
// }

use crate::{
    pvss::chunked_elgamal_field::public_parameters::PublicParameters, traits::Convert, Scalar,
};

impl<E: Pairing> Convert<Scalar<E>, PublicParameters<E>> for InputSecret<E::ScalarField> {
    fn to(&self, _with: &PublicParameters<E>) -> Scalar<E> {
        Scalar(*self.get_secret_a())
    }
}

#[cfg(test)]
mod test {}
