// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Utilities for representing inputs to multi-scalar multiplications (MSMs).
//!
//! An MSM takes a collection of bases and corresponding scalars and computes
//! their linear combination. This module defines a simple container for such
//! inputs.

use ark_ec::CurveGroup;
use ark_ff::Zero;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::{collections::HashMap, fmt::Debug, hash::Hash};

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

/// Merges multiple `MsmInput`s into one, with the i-th input scaled by `scales[i]`.
/// Same base in different inputs is aggregated (scalars summed). Terms with zero scalar are dropped.
///
/// # Panics
/// If `term_sets.len() != scales.len()` or if the merged result cannot be built into `MsmInput`.
pub fn merge_scaled_msm_terms<C: CurveGroup>(
    term_sets: &[&MsmInput<C::Affine, C::ScalarField>],
    scales: &[C::ScalarField],
) -> MsmInput<C::Affine, C::ScalarField>
where
    C::Affine: Copy + Eq + Hash,
{
    assert_eq!(
        term_sets.len(),
        scales.len(),
        "term_sets and scales length mismatch"
    );
    let mut agg: HashMap<C::Affine, C::ScalarField> = HashMap::new();
    for (terms, scale) in term_sets.iter().zip(scales.iter()) {
        for (base, scalar) in terms.bases().iter().zip(terms.scalars().iter()) {
            let s = *scalar * scale;
            agg.entry(*base).and_modify(|s0| *s0 += s).or_insert(s);
        }
    }
    let (bases, scalars): (Vec<_>, Vec<_>) = agg.into_iter().filter(|(_, s)| !s.is_zero()).unzip();
    MsmInput::new(bases, scalars).expect("merged MSM terms")
}
