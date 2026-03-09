// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Utilities for representing inputs to multi-scalar multiplications (MSMs).
//!
//! An MSM takes a collection of bases and corresponding scalars and computes
//! their linear combination. This module defines a simple container for such
//! inputs.

use crate::{arkworks::random::sample_field_element, utils};
use ark_ec::AffineRepr;
use ark_ff::Zero;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand::Rng;
use std::{collections::HashMap, fmt::Debug};

/// Input to a (not necessarily fixed-base) multi-scalar multiplication (MSM).
///
/// An MSM input consists of:
/// * a list of base elements, and
/// * a list of scalar elements,
/// which are interpreted pairwise.
///
/// Implementations that construct an `MsmInput` should ensure that
/// `bases.len() == scalars.len()`
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, PartialEq, Eq, Debug)]
pub struct MsmInput<
    B: CanonicalSerialize + CanonicalDeserialize,
    S: CanonicalSerialize + CanonicalDeserialize,
> {
    /// The bases of the MSM.
    pub bases: Vec<B>,
    /// The scalars of the MSM.
    pub scalars: Vec<S>,
}

impl<B, S> MsmInput<B, S>
where
    B: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug,
    S: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug,
{
    /// Returns a reference to the slice of base elements in this MSM input.
    #[inline]
    pub fn bases(&self) -> &[B] {
        &self.bases
    }

    /// Returns a reference to the slice of scalar elements in this MSM input.
    #[inline]
    pub fn scalars(&self) -> &[S] {
        &self.scalars
    }

    /// Constructs a new MSM input from the provided bases and scalars.
    ///
    /// Returns an error if the lengths of `bases` and `scalars` do not match.
    pub fn new(bases: Vec<B>, scalars: Vec<S>) -> anyhow::Result<Self> {
        if bases.len() != scalars.len() {
            anyhow::bail!(
                "MsmInput length mismatch: {} bases, {} scalars",
                bases.len(),
                scalars.len(),
            );
        }
        Ok(Self { bases, scalars })
    }
}

/// Merges multiple `MsmInput`s into one by scaling the i-th input by `scales[i]` and aggregating.
/// Same base across inputs is combined (scalars summed). Terms with zero scalar are dropped.
///
/// # Panics
/// If `inputs.len() != scales.len()` or if the merged result cannot be built into `MsmInput`.
pub fn merge_msm_inputs_with_scales<A: AffineRepr>(
    inputs: &[MsmInput<A, A::ScalarField>],
    scales: &[A::ScalarField],
) -> MsmInput<A, A::ScalarField> {
    assert_eq!(
        inputs.len(),
        scales.len(),
        "inputs and scales length mismatch"
    );
    let mut agg: HashMap<A, A::ScalarField> = HashMap::new();
    for (input, scale) in inputs.iter().zip(scales.iter()) {
        for (base, scalar) in input.bases().iter().zip(input.scalars().iter()) {
            let s = *scalar * scale;
            agg.entry(*base).and_modify(|s0| *s0 += s).or_insert(s);
        }
    }
    let (bases, scalars): (Vec<_>, Vec<_>) = agg.into_iter().filter(|(_, s)| !s.is_zero()).unzip();
    MsmInput::new(bases, scalars).expect("merged MSM inputs")
}

/// Merges multiple `MsmInput`s into one by sampling a random `beta`, using
/// `[1, beta, beta^2, ...]` as scales, and calling `merge_msm_inputs_with_scales`.
pub fn merge_msm_inputs<A: AffineRepr, R: Rng>(
    inputs: &[MsmInput<A, A::ScalarField>],
    rng: &mut R,
) -> MsmInput<A, A::ScalarField> {
    let beta = sample_field_element(rng);
    let scales = utils::powers(beta, inputs.len());
    merge_msm_inputs_with_scales(inputs, &scales)
}

/// Multi-scalar multiplication with boolean scalars.
///
/// Treats each `bool` as 0 or 1 and returns
/// `sum_i scalars[i] * bases[i]`.
///
/// # Arguments
/// * `bases` – curve points (affine).
/// * `scalars` – one boolean per base; `true` means include that base, `false` means skip it.
///
/// # Panics
/// In debug builds, panics if `bases.len() != scalars.len()`.
///
/// Probably won't be needed in future versions of `arkworks`.
pub fn msm_bool<A: AffineRepr>(bases: &[A], scalars: &[bool]) -> A::Group {
    // Bases and scalars must have the same length for multi-scalar multiplication.
    debug_assert_eq!(
        bases.len(),
        scalars.len(),
        "bases and scalars must have the same length for MSM (got {} bases, {} scalars)",
        bases.len(),
        scalars.len()
    );

    let mut acc = A::Group::zero();
    for (base, &bit) in bases.iter().zip(scalars) {
        if bit {
            acc += base;
        }
    }
    acc
}
