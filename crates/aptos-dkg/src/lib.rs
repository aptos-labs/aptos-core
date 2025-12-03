// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Innovation-Enabling Source Code License

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
    shamir::{Reconstructable, ShamirThresholdConfig},
};
pub use aptos_crypto::{
    blstrs as algebra,
    blstrs::{G1_PROJ_NUM_BYTES, G2_PROJ_NUM_BYTES, SCALAR_NUM_BYTES},
};
use ark_ec::pairing::Pairing;
use ark_ff::{Fp, FpConfig};
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
pub mod utils;
pub mod weighted_vuf;
use aptos_crypto::arkworks::shamir::ShamirShare;

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
pub struct Scalar<E: Pairing>(pub E::ScalarField); // TODO: Maybe this should be Scalar<F: PrimeField> ?? (PrimeField is needed for ThresholdConfig below)

impl<E: Pairing> Scalar<E> {
    /// Converts a `&[Scalar<E>]` into a `&[E::ScalarField]`; could do this without copying
    /// (and similarly for the other functions below) by using `#[repr(transparent)]` and
    /// unsafe Rust, but we want to avoid that
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

    /// Converts a `&[E::ScalarField]` into a `Vec<Scalar<E>>` safely.
    pub fn vec_from_inner_slice(slice: &[E::ScalarField]) -> Vec<Self> {
        slice.iter().copied().map(Self).collect()
    }
}

impl<E: Pairing> UniformRand for Scalar<E> {
    fn rand<R: Rng>(rng: &mut R) -> Self {
        Scalar(sample_field_element(rng))
    }
}

// TODO: maybe move the Reconstructable trait to the SecretSharingConfig in the PVSS trait, with associated Scalar equal to InputSecret
// then make the existing implementation of `fn reconstruct()` part of a trait... and then we can remove the trivial implementation below!
impl<const N: usize, P: FpConfig<N>, E: Pairing<ScalarField = Fp<P, N>>>
    Reconstructable<ShamirThresholdConfig<E::ScalarField>> for Scalar<E>
{
    type ShareValue = Scalar<E>;

    fn reconstruct(
        sc: &ShamirThresholdConfig<E::ScalarField>,
        shares: &[ShamirShare<Self::ShareValue>],
    ) -> anyhow::Result<Self> {
        assert_ge!(shares.len(), sc.get_threshold());
        assert_le!(shares.len(), sc.get_total_num_players());

        let shares_destructured: Vec<(Player, E::ScalarField)> = shares
            .iter()
            .map(|(player, scalar)| (*player, scalar.0))
            .collect();

        Ok(Scalar(E::ScalarField::reconstruct(
            &sc,
            &shares_destructured,
        )?))
    }
}
