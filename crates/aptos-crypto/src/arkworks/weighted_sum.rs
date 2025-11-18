// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Weighted sum utilities for scalars and elliptic curve points.

use ark_ec::{
    short_weierstrass::{Affine, SWCurveConfig},
    AffineRepr, VariableBaseMSM as _,
};
use ark_ff::{Fp, FpConfig, PrimeField};

/// Trait for computing a **weighted sum** (linear combination) of elements.
pub trait WeightedSum: Copy {
    /// The type of scalar weights used in the linear combination.
    type Scalar: PrimeField;

    /// Computes the weighted sum of the provided bases and scalars.
    /// Returns the linear combination `∑ s_i * b_i`.
    fn weighted_sum(bases: &[Self], scalars: &[Self::Scalar]) -> Self;
}

impl<const N: usize, P: FpConfig<N>> WeightedSum for Fp<P, N> {
    type Scalar = Fp<P, N>;

    fn weighted_sum(bases: &[Self], scalars: &[Self::Scalar]) -> Self {
        assert_eq!(bases.len(), scalars.len());

        bases.iter().zip(scalars).map(|(b, s)| b * s).sum()
    }
}

impl<P: SWCurveConfig> WeightedSum for Affine<P> {
    type Scalar = P::ScalarField;

    fn weighted_sum(bases: &[Self], scalars: &[Self::Scalar]) -> Self {
        <Self as AffineRepr>::Group::msm(bases, scalars)
            .expect("MSM failed weighted_sum()")
            .into()
    }
}
