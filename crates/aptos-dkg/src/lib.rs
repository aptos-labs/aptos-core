// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

#![allow(clippy::redundant_static_lifetimes)]
#![allow(clippy::needless_return)]
#![allow(clippy::clone_on_copy)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_range_loop)]
#![allow(clippy::let_and_return)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::useless_conversion)]
#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::redundant_closure)]
#![allow(clippy::map_identity)]
#![allow(clippy::let_unit_value)]
#![allow(clippy::vec_init_then_push)]
#![allow(clippy::to_string_in_format_args)]
#![allow(clippy::borrow_interior_mutable_const)]

use crate::pvss::{traits, Player};
use aptos_crypto::arkworks::{
    random::{sample_field_element, UniformRand},
    shamir::{Reconstructable, ShamirShare, ShamirThresholdConfig},
};
pub use aptos_crypto::{
    blstrs as algebra,
    blstrs::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES},
};
use ark_ff::{Fp, FpConfig, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use more_asserts::{assert_ge, assert_le};
use rand::Rng;
pub use utils::random::DST_RAND_CORE_HELL;

pub mod dlog;
pub(crate) mod fiat_shamir;
pub mod pcs;
pub mod pvss;
pub mod range_proofs;
pub mod sigma_protocol;
pub mod sumcheck;
pub mod utils;
pub mod weighted_vuf;

/// A wrapper around `E::ScalarField` to prevent overlapping trait implementations.
///
/// Without this wrapper, implementing a trait both for `blstrs::Scalar` and for
/// `E::ScalarField` would cause conflicts. For example, Rust would reject:
/// - `impl<Trait> for blstrs::Scalar`
/// - `impl<Trait> for E::ScalarField`
///
/// because some pairing engine `E` might (now or in the future) define
/// `E::ScalarField = blstrs::Scalar`.
///
/// Similarly, this issue also arises with blanket implementations like:
/// `impl<T: Trait> for Vec<T>`, since `Vec<T>` could itself be an
/// `E::ScalarField`.
#[repr(transparent)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scalar<F: PrimeField>(pub F); // TODO: Maybe this should be Scalar<F: PrimeField> ?? (PrimeField is needed for ThresholdConfig below)

impl<F: PrimeField> Scalar<F> {
    /// Converts a `&[Scalar<E>]` into a `&[E::ScalarField]`; could do this without copying
    /// (and similarly for the other functions below) by using `#[repr(transparent)]` and
    /// unsafe Rust, but we want to avoid that
    pub fn slice_as_inner(slice: &[Self]) -> Vec<F> {
        slice.iter().map(|s| s.0).collect()
    }

    /// Converts a `Vec<Scalar<E>>` into a `Vec<E::ScalarField>` safely.
    pub fn vec_into_inner(v: Vec<Self>) -> Vec<F> {
        v.into_iter().map(|s| s.0).collect()
    }

    /// Converts a `Vec<E::ScalarField>` into a `Vec<Scalar<E>>` safely.
    pub fn vec_from_inner(v: Vec<F>) -> Vec<Self> {
        v.into_iter().map(Self).collect()
    }

    /// Converts a `Vec<Vec<E::ScalarField>>` into a `Vec<Vec<Scalar<E>>>` safely.
    pub fn vecvec_from_inner(vv: Vec<Vec<F>>) -> Vec<Vec<Self>> {
        vv.into_iter().map(Self::vec_from_inner).collect()
    }

    /// Converts a `&[E::ScalarField]` into a `Vec<Scalar<E>>` safely.
    pub fn vec_from_inner_slice(slice: &[F]) -> Vec<Self> {
        slice.iter().copied().map(Self).collect()
    }

    /// Converts a `Vec<Vec<Vec<E::ScalarField>>>` into a `Vec<Vec<Vec<Scalar<E>>>>` safely.
    pub fn vecvecvec_from_inner(vvv: Vec<Vec<Vec<F>>>) -> Vec<Vec<Vec<Self>>> {
        vvv.into_iter().map(Self::vecvec_from_inner).collect()
    }

    pub fn into_fr(&self) -> F {
        self.0
    }
}

impl<F: PrimeField> UniformRand for Scalar<F> {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        Scalar(sample_field_element(rng))
    }
}

impl<const N: usize, P: FpConfig<N>> Reconstructable<ShamirThresholdConfig<Fp<P, N>>>
    for Scalar<Fp<P, N>>
{
    type ShareValue = Scalar<Fp<P, N>>;

    fn reconstruct(
        sc: &ShamirThresholdConfig<Fp<P, N>>,
        shares: &[ShamirShare<Self::ShareValue>],
    ) -> anyhow::Result<Self> {
        assert_ge!(shares.len(), sc.get_threshold());
        assert_le!(shares.len(), sc.get_total_num_players());

        let shares_destructured: Vec<(Player, Fp<P, N>)> = shares
            .iter()
            .map(|(player, scalar)| (*player, scalar.0))
            .collect();

        Ok(Scalar(Fp::<P, N>::reconstruct(sc, &shares_destructured)?))
    }
}
