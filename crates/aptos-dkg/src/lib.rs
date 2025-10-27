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

use crate::constants::SCALAR_FIELD_ORDER;
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
pub use constants::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES};
pub use utils::random::DST_RAND_CORE_HELL;

pub mod algebra;
pub mod constants;
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
/// #[repr(transparent)]
#[repr(transparent)]
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub struct Scalar<E: Pairing>(pub E::ScalarField);

impl<E: Pairing> Scalar<E> {
    /// Converts a `&[Scalar<E>]` into a `&[E::ScalarField]` without copying.
    ///
    /// # Safety
    /// These functions is safe because `Scalar<E>` is `#[repr(transparent)]`
    /// over `E::ScalarField`, so the memory layouts are guaranteed to match.
    pub fn slice_as_inner(slice: &[Self]) -> &[E::ScalarField] {
        unsafe { std::slice::from_raw_parts(slice.as_ptr() as *const E::ScalarField, slice.len()) }
    }

    pub fn vec_into_inner(v: Vec<Self>) -> Vec<E::ScalarField> {
        let v = std::mem::ManuallyDrop::new(v);
        unsafe { Vec::from_raw_parts(v.as_ptr() as *mut E::ScalarField, v.len(), v.capacity()) }
    }

    pub fn vec_from_inner(v: Vec<E::ScalarField>) -> Vec<Self> {
        let v = std::mem::ManuallyDrop::new(v);
        unsafe { Vec::from_raw_parts(v.as_ptr() as *mut Self, v.len(), v.capacity()) }
    }

    pub fn vecvec_from_inner(vv: Vec<Vec<E::ScalarField>>) -> Vec<Vec<Self>> {
        vv.into_iter()
            .map(Self::vec_from_inner)
            .collect()
    }
}

use ark_std::rand::Rng;
use ark_std::UniformRand;

impl<E: Pairing> Scalar<E> {
    pub fn rand<R: Rng + ?Sized>(rng: &mut R) -> Self {
        Scalar(E::ScalarField::rand(rng))
    }
} // I don't know if this overwrites SigmaProtocolWitness?

use crate::pvss::traits::Reconstructable;
use crate::pvss::ThresholdConfig;
use crate::pvss::Player;
use crate::algebra::lagrange::lagrange_coefficients;
use more_asserts::assert_ge;
use more_asserts::assert_le;
use crate::pvss::traits::SecretSharingConfig;

impl<E: Pairing> Reconstructable<ThresholdConfig> for Scalar<E> {
    type Share = Scalar<E>;

    fn reconstruct(sc: &ThresholdConfig, shares: &Vec<(Player, Self::Share)>) -> Self {
        assert_ge!(shares.len(), sc.get_threshold());
        assert_le!(shares.len(), sc.get_total_num_players());

        let ids = shares.iter().map(|(p, _)| p.id).collect::<Vec<usize>>();
        let lagr = lagrange_coefficients(
            sc.get_batch_evaluation_domain(),
            ids.as_slice(),
            &Scalar::ZERO,
        );
        let shares = shares
            .iter()
            .map(|(_, share)| *share)
            .collect::<Vec<Scalar<E>>>();

        assert_eq!(lagr.len(), shares.len());

        shares
            .iter()
            .zip(lagr.iter())
            .map(|(&share, &lagr)| share * lagr)
            .sum::<Scalar>()
    }
}