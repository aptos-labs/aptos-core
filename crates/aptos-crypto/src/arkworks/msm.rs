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
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

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
    type Scalar: Clone + CanonicalSerialize + CanonicalDeserialize; // scrap and make associated type of MsmInput

    /// The group/base type used in the MSMs. Current instantiations always use E::G1Affine but as explained
    /// in the TODO of doc comment of `fn verify_msm_hom`, we might want to be working with enums here in the future.
    type Base: Clone + CanonicalSerialize + CanonicalDeserialize; // scrap and make associated type of MsmInput

    /// Returns a reference to the slice of base elements in this MSM input.
    fn bases(&self) -> &[Self::Base];

    /// Returns a reference to the slice of scalar elements in this MSM input.
    fn scalars(&self) -> &[Self::Scalar];

    /// Returns a new instance
    fn new(bases: Vec<Self::Base>, scalars: Vec<Self::Scalar>) -> anyhow::Result<Self>;
}

impl<B, S> IsMsmInput for MsmInput<B, S>
where
    B: CanonicalSerialize + CanonicalDeserialize + Clone,
    S: CanonicalSerialize + CanonicalDeserialize + Clone,
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
#[allow(non_snake_case)]
pub fn verify_msm_terms_with_start<C: CurveGroup>(
    msm_terms: Vec<MsmInput<C::Affine, C::ScalarField>>,
    mut final_bases: Vec<C::Affine>,
    mut final_scalars: Vec<C::ScalarField>,
    powers_of_beta: Vec<C::ScalarField>,
) -> anyhow::Result<()> {
    assert_eq!(msm_terms.len(), powers_of_beta.len());

    for (term, beta_power) in msm_terms.into_iter().zip(powers_of_beta) {
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
#[allow(non_snake_case)]
pub fn verify_msm_terms<C: CurveGroup>(
    msm_terms: Vec<MsmInput<C::Affine, C::ScalarField>>,
    beta: C::ScalarField,
) -> anyhow::Result<()> {
    let powers_of_beta = utils::powers(beta, msm_terms.len());

    verify_msm_terms_with_start::<C>(msm_terms, Vec::new(), Vec::new(), powers_of_beta)
}
