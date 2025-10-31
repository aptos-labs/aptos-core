// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use aptos_crypto::Uniform;
use aptos_crypto_derive::{SilentDebug, SilentDisplay};
use ark_ec::{pairing::Pairing};
use derive_more::{Add};
use num_traits::Zero;
use std::ops::{AddAssign};
use crate::{
    pvss::chunked_elgamal_field::public_parameters::PublicParameters, traits::Convert, Scalar,
};

#[derive(SilentDebug, SilentDisplay, PartialEq, Add)]
pub struct InputSecret<F: ark_ff::Field> {
    /// The actual secret being dealt; a scalar $a \in F$.
    a: F,
}

#[cfg(feature = "assert-private-keys-not-cloneable")]
static_assertions::assert_not_impl_any!(InputSecret: Clone);

impl<F: ark_ff::Field> Uniform for InputSecret<F> {
    fn generate<R>(_rng: &mut R) -> Self
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