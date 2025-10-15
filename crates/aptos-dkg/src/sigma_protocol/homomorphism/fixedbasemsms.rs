// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sigma_protocol::{homomorphism, homomorphism::EntrywiseMap};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

/// Temporary workaround because stable Rust does not yet support associated type defaults.
pub trait IsMsmInput<B, S> {
    /// Returns a reference to the slice of base elements in this MSM input.
    fn bases(&self) -> &[B];

    /// Returns a reference to the slice of scalar elements in this MSM input.
    fn scalars(&self) -> &[S];
}

/// Represents the input to a multi-scalar multiplication (MSM):
/// a collection of bases and corresponding scalars.
#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone)]
pub struct MsmInput<
    B: CanonicalSerialize + CanonicalDeserialize,
    S: CanonicalSerialize + CanonicalDeserialize,
> {
    pub bases: Vec<B>,
    pub scalars: Vec<S>,
}

impl<
        B: CanonicalSerialize + CanonicalDeserialize,
        S: CanonicalSerialize + CanonicalDeserialize,
    > IsMsmInput<B, S> for MsmInput<B, S>
{
    fn bases(&self) -> &[B] {
        &self.bases
    }

    fn scalars(&self) -> &[S] {
        &self.scalars
    }
}

/// A `FixedBaseMsms` instance represents a homomorphism whose outputs can be expressed
/// as one or more **fixed-base multi-scalar multiplications (MSMs)**, sharing consistent base and scalar types.
///
/// When such homomorphisms are used in a Σ-protocol, the resulting
/// verification equations can be batched efficiently via a
/// Schwartz–Zippel–style random linear combination.  
/// This requires uniformly iterating over all MSMs involved in the proof
/// and public statement.
///
/// Typically, MSM bases are fixed early in the protocol (e.g., as powers of τ or other public parameters),
/// and only the scalars vary as inputs.
///
/// This trait provides:
/// - Methods for computing the MSM representations of a homomorphism input.
/// - A uniform “shape” abstraction for collecting and flattening MSM outputs
///   for batch verification in Σ-protocols.
pub trait FixedBaseMsms:
    homomorphism::Trait<Codomain = Self::CodomainShape<Self::MsmOutput>>
{
    /// The scalar type used in the MSMs.
    type Scalar;

    /// The group/base type used in the MSMs.
    type Base;

    /// Type representing a single MSM input (a set of bases and scalars).
    /// Normally, this would default to `MsmInput<Self::Base, Self::Scalar>`,
    /// but stable Rust does not yet support associated type defaults.
    type MsmInput: CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + IsMsmInput<Self::Base, Self::Scalar>;

    /// The output type of evaluating an MSM. `Codomain` should equal `CodomainShape<MsmOutput>`
    type MsmOutput: CanonicalSerialize + CanonicalDeserialize + Clone;

    /// The "shape" of the homomorphism's output, parameterized by an inner type `T`.
    // TODO: type CodomainShape<T>: for<'a> IntoIterator<Item = &'a T> + 'static;
    type CodomainShape<T>: EntrywiseMap<T, Output<T> = Self::CodomainShape<T>>
        + IntoIterator<Item = T>
        + CanonicalSerialize
        + CanonicalDeserialize
        + Clone
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone;

    /// Returns the MSM terms corresponding to a given homomorphism input.
    ///
    /// The result is structured such that applying MSM evaluation elementwise
    /// yields the homomorphism’s output.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput>;

    /// Evaluates a single MSM instance given slices of bases and scalars.
    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput;

    /// Applies `msm_eval` elementwise to a collection of MSM inputs.
    fn apply_msm(
        &self,
        msms: Self::CodomainShape<Self::MsmInput>,
    ) -> Self::CodomainShape<Self::MsmOutput>
    where
        Self::CodomainShape<Self::MsmInput>: EntrywiseMap<
            Self::MsmInput,
            Output<Self::MsmOutput> = Self::CodomainShape<Self::MsmOutput>,
        >,
    {
        msms.map(|msm_input| Self::msm_eval(&msm_input.bases(), &msm_input.scalars()))
    }
}

impl<H, LargerDomain: Sync> FixedBaseMsms for homomorphism::LiftHomomorphism<H, LargerDomain>
where
    H: FixedBaseMsms,
{
    type Base = H::Base;
    type CodomainShape<T>
        = H::CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone;
    type MsmInput = H::MsmInput;
    type MsmOutput = H::MsmOutput;
    type Scalar = H::Scalar;

    /// Returns the MSM terms corresponding to a given homomorphism input. The output is shaped so that applying the MSM elementwise yields the homomorphism output.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let projected = (self.projection)(input);
        self.hom.msm_terms(&projected)
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        H::msm_eval(bases, scalars)
    }
}
