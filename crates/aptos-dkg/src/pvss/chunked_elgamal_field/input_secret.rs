// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::pvss::{dealt_pub_key::g2::DealtPubKey, chunked_elgamal_field, traits};
use blstrs::Scalar;
use std::ops::Mul;

// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::utils::random::random_scalar;
use aptos_crypto::Uniform;
use aptos_crypto_derive::{SilentDebug, SilentDisplay};
use ff::Field;
use num_traits::Zero;
use ark_std::rand::{CryptoRng, RngCore};
use std::ops::{Add, AddAssign};
use ark_ec::pairing::Pairing;

/// The *input secret* that will be given as input to the PVSS dealing algorithm. This will be of a
/// different type than the *dealt secret* that will be returned by the PVSS reconstruction algorithm.
///
/// This secret will NOT need to be stored by validators because a validator (1) picks such a secret
/// and (2) deals it via the PVSS. If the validator crashes during dealing, the entire task will be
/// restarted with a freshly-generated input secret.
/// 
use derive_more::{Add, AddAssign, Display, From, Into};
use ark_ec::AdditiveGroup;

#[derive(SilentDebug, SilentDisplay, PartialEq, Add, AddAssign)]
pub struct InputSecret<F: ark_ff::Field> {
    /// The actual secret being dealt; a scalar $a \in F$.
    a: F,
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
    fn rand<R: RngCore + ?Sized>(rng: &mut R) -> Self
    {
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

#[cfg(test)]
mod test {}
