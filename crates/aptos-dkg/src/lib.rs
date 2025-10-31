// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

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
use aptos_crypto::arkworks::shamir::{ShamirShare, ThresholdConfig};
pub use aptos_crypto::blstrs::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES};
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::{rand::Rng, UniformRand};
use more_asserts::{assert_ge, assert_le};
pub use utils::random::DST_RAND_CORE_HELL;

pub mod algebra;
pub(crate) mod fiat_shamir;
pub mod pcs;
pub mod pvss;
pub mod range_proofs;
pub mod sigma_protocol;
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
pub struct Scalar<E: Pairing>(pub E::ScalarField); // TODO: Maybe this should be  Scalar<F: PrimeField> ??

impl<E: Pairing> Scalar<E> {
    /// Converts a `&[Scalar<E>]` into a `&[E::ScalarField]`; could do this without copying
    /// by using #[repr(transparent)] and unsafe Rust, but we want to avoid that
    pub fn slice_as_inner(slice: &[Self]) -> Vec<E::ScalarField>
    where
        E::ScalarField: Clone,
    {
        slice.iter().map(|s| s.0.clone()).collect()
    }

    /// Converts a `Vec<Scalar<E>>` into a `Vec<E::ScalarField>` safely.
    pub fn vec_into_inner(v: Vec<Self>) -> Vec<E::ScalarField> {
        v.into_iter().map(|s| s.0).collect()
    }

    /// Converts a `Vec<E::ScalarField>` into a `Vec<Scalar<E>>` safely.
    pub fn vec_from_inner(v: Vec<E::ScalarField>) -> Vec<Self> {
        v.into_iter().map(Self).collect()
    }

    /// Converts a `Vec<Vec<E::ScalarField>>` into a `Vec<Vec<Scalar<E>>>` safely.
    pub fn vecvec_from_inner(vv: Vec<Vec<E::ScalarField>>) -> Vec<Vec<Self>> {
        vv.into_iter().map(Self::vec_from_inner).collect()
    }
}

impl<E: Pairing> Scalar<E> {
    pub fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self {
        Scalar(E::ScalarField::rand(rng))
    }
}

impl<E: Pairing> traits::Reconstructable<ThresholdConfig<E::ScalarField>> for Scalar<E> {
    type Share = Scalar<E>;

    fn reconstruct(
        sc: &ThresholdConfig<E::ScalarField>,
        shares: &Vec<(Player, Self::Share)>,
    ) -> Self {
        assert_ge!(shares.len(), sc.get_threshold());
        assert_le!(shares.len(), sc.get_total_num_players());

        // Convert shares to a Vec of ShamirShare // TODO: get rid of this?
        let shamir_shares: Vec<ShamirShare<E::ScalarField>> = shares
            .iter()
            .map(|(p, share)| ShamirShare {
                x: E::ScalarField::from(p.id as u64),
                y: share.0,
            })
            .collect();

        Scalar(sc.reconstruct(&shamir_shares).unwrap())
    }
}
