// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! Utilities for representing inputs to multi-scalar multiplications (MSMs).
//!
//! An MSM takes a collection of bases and corresponding scalars and computes
//! their linear combination. This module defines a simple container for such
//! inputs, along with a small trait to abstract over concrete container types.

use crate::utils;
use anyhow::ensure;
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
    /// the bases of the MSM
    pub bases: Vec<B>,
    /// the scalars of the MSM
    pub scalars: Vec<S>,
}

/// Trait abstraction for types representing MSM inputs.
///
/// This exists as a workaround because stable Rust does not yet support default
/// associated types. (Now we have to do `type MsmInput: IsMsmInput` rather than `type MsmInput = IsMsmInput`)
/// TODO: we probably don't need this trait, can just do MsmInput<E::Base, E::Scalar> in function signatures???
pub trait IsMsmInput: Sized {
    // maybe make B and S associated types instead
    /// The scalar type used in the MSMs.
    type Scalar: Clone + CanonicalSerialize + CanonicalDeserialize + Eq + Debug;

    /// The group/base type used in the MSMs. Current instantiations always use E::G1Affine but as explained
    /// in the TODO of doc comment of `fn verify_msm_hom`, we might want to be working with enums here in the future.
    type Base: Clone + CanonicalSerialize + CanonicalDeserialize + Eq + Debug;

    /// Returns a reference to the slice of base elements in this MSM input.
    fn bases(&self) -> &[Self::Base];

    /// Returns a reference to the slice of scalar elements in this MSM input.
    fn scalars(&self) -> &[Self::Scalar];

    /// Constructs a new MSM input from the provided bases and scalars.
    ///
    /// Should return an error if the lengths of `bases` and `scalars` do not match.
    fn new(bases: Vec<Self::Base>, scalars: Vec<Self::Scalar>) -> anyhow::Result<Self>;
}

impl<B, S> IsMsmInput for MsmInput<B, S>
where
    B: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug,
    S: CanonicalSerialize + CanonicalDeserialize + Clone + Eq + Debug,
{
    type Base = B;
    type Scalar = S;

    fn bases(&self) -> &[Self::Base] {
        &self.bases
    }

    fn scalars(&self) -> &[Self::Scalar] {
        &self.scalars
    }

    fn new(bases: Vec<Self::Base>, scalars: Vec<Self::Scalar>) -> anyhow::Result<Self> {
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

/// Verifies that a collection of MSMs are all equal to zero, by combining
/// them into one big MSM using random linear combination, following the
/// Schwartz-Zippel philosophy.
///
/// In this particular function we assume that this process has already been
/// "started", which is *useful* since the sigma protocol's MSM scalars are
/// already manipulated with betas, and changing that would make things a
/// tiny bit slower
///
/// TODO: in theory the hash table approach as in the sigma protocol might be useful here again - move it
#[allow(non_snake_case)]
pub fn verify_msm_terms_with_start<C: CurveGroup>(
    new_msm_terms: Vec<MsmInput<C::Affine, C::ScalarField>>,
    existing_msm_terms: MsmInput<C::Affine, C::ScalarField>,
    powers_of_beta: Vec<C::ScalarField>,
) -> anyhow::Result<()> {
    assert_eq!(new_msm_terms.len(), powers_of_beta.len());

    let mut final_bases = existing_msm_terms.bases().to_vec();
    let mut final_scalars = existing_msm_terms.scalars().to_vec();

    for (term, beta_power) in new_msm_terms.into_iter().zip(powers_of_beta) {
        let mut scalars = term.scalars().to_vec();

        for scalar in scalars.iter_mut() {
            *scalar *= beta_power;
        }

        final_bases.extend(term.bases());
        final_scalars.extend(scalars);
    }

    let msm_result = C::msm(&final_bases, &final_scalars).expect("Could not compute batch MSM");
    ensure!(msm_result == C::ZERO);

    Ok(())
}

/// Verifies that a collection of MSMs are all equal to zero, by combining
/// them into one big MSM using random linear combination, following the
/// Schwartz-Zippel philosophy; delegates the actual work to
/// `verify_msm_terms_with_start()`
///
/// TODO: doesn't get used?
#[allow(non_snake_case)]
pub fn verify_msm_terms<C: CurveGroup>(
    msm_terms: Vec<MsmInput<C::Affine, C::ScalarField>>,
    beta: C::ScalarField,
) -> anyhow::Result<()> {
    let powers_of_beta = utils::powers(beta, msm_terms.len());

    verify_msm_terms_with_start::<C>(
        msm_terms,
        MsmInput::new(Vec::new(), Vec::new()).unwrap(),
        powers_of_beta,
    )
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
