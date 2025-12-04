// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{pvss::chunky::public_parameters::PublicParameters, traits::Convert, Scalar};
use aptos_crypto::{arkworks, Uniform};
use aptos_crypto_derive::{SilentDebug, SilentDisplay};
use ark_ec::pairing::Pairing;
use derive_more::Add;
use num_traits::Zero;
use std::ops::AddAssign;

#[derive(SilentDebug, SilentDisplay, PartialEq, Add)]
pub struct InputSecret<F: ark_ff::Field> {
    /// The actual secret being dealt; a scalar $a \in F$.
    a: F,
}

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(InputSecret: Clone);

impl<F: ark_ff::PrimeField> Uniform for InputSecret<F> {
    fn generate<R>(rng: &mut R) -> Self
    where
        R: rand::RngCore + rand::CryptoRng,
    {
        Self {
            a: arkworks::random::sample_field_element(rng),
        }
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

impl<F: ark_ff::Field> InputSecret<F> {
    pub fn get_secret_a(&self) -> &F {
        &self.a
    }
}

impl<E: Pairing> Convert<Scalar<E>, PublicParameters<E>> for InputSecret<E::ScalarField> {
    fn to(&self, _with: &PublicParameters<E>) -> Scalar<E> {
        Scalar(*self.get_secret_a())
    }
}
