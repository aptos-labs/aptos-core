// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{homomorphism, homomorphism::EntrywiseMap, Witness},
};
use aptos_crypto::arkworks::msm::MsmInput;
use ark_ec::PrimeGroup;
use ark_ff::Zero;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::{fmt::Debug, hash::Hash};

/// A `FixedBaseMsms` instance represents a homomorphism whose outputs can be expressed as
/// one or more **fixed-base multi-scalar multiplications (MSMs)**, sharing consistent base and scalar types.
///
/// When such homomorphisms are used in a Σ-protocol, the resulting verification equations can be batched efficiently
/// via a Schwartz–Zippel–style random linear combination. This requires uniformly iterating over all MSMs involved
/// in the proof and public statement.
///
/// Typically, MSM bases are fixed early in the protocol (e.g., as powers of τ or other public parameters),
/// and only the scalars vary as inputs.
///
/// This trait provides:
/// - Methods for computing the MSM representations of a homomorphism input.
/// - A uniform “shape” abstraction for collecting and flattening MSM outputs
///   for batch verification in Σ-protocols.
pub trait Trait:
    homomorphism::Trait<
    Codomain = Self::CodomainShape<Self::MsmOutput>,
    CodomainNormalized = Self::CodomainShape<Self::Base>,
>
{
    /// Type of MSM base points (e.g. curve affine element). CodomainNormalized is `CodomainShape<Base>`.
    type Base: Copy + Eq + Hash + CanonicalSerialize + CanonicalDeserialize + Clone + Debug;

    /// Scalar type used in MSM terms.
    type Scalar: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq + Zero;

    /// The output type of evaluating an MSM. `Codomain` should equal `CodomainShape<MsmOutput>`, in the current version
    /// of the code.
    type MsmOutput: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq + Zero;

    /// Represents the **shape** of the homomorphism's output, parameterized by an inner type `T`.
    ///
    /// ### Overview
    /// The codomain of a homomorphism is often a **nested structure** — for example, `Vec<Vec<E::G1>>`.
    /// In the case of a `FixedBaseMsms`, the homomorphism then factorizes as:
    ///
    /// Domain ─▶ Vec<Vec<MsmInput>> ─▶ Vec<Vec<E::G1>> = Codomain
    ///
    /// For **efficient batch verification**, it’s useful to collect all MSM terms together and
    /// combine them with the sigma proof’s *commitment* (the first prover message) and the public statement.
    /// To do this consistently, we need a uniform way to iterate over nested structures such as `Vec<Vec<T>>`
    /// to access all elements in a **consistent** order.
    ///
    /// ### TODO
    /// - The use of `IntoIterator` leads to cloning, which might not be very efficient
    type CodomainShape<T>: IntoIterator<Item = T>
        + CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + Debug
        + Eq
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;

    /// Returns the MSM terms corresponding to a given homomorphism input.
    ///
    /// The result is structured such that applying MSM evaluation elementwise
    /// yields the homomorphism’s output.
    fn msm_terms(
        &self,
        input: &Self::Domain,
    ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>>;

    /// Evaluates a single MSM instance given slices of bases and scalars. Current instantiations always use E::G1Affine
    /// for the base, but we might want to use enums for the base and output in the future.
    fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput;

    /// Applies `msm_eval` elementwise to a collection of MSM inputs.
    fn apply_msm(
        &self, // TODO: might be able to get rid of this?
        msms: Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>>,
    ) -> Self::CodomainShape<Self::MsmOutput>
    where
        Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>>: EntrywiseMap<
            MsmInput<Self::Base, Self::Scalar>,
            Output<Self::MsmOutput> = Self::CodomainShape<Self::MsmOutput>,
        >,
    {
        msms.map(|msm_input| Self::msm_eval(msm_input))
    }

    // Depending on the elliptic curve library (arkworks, blstrs, etc.), the implementatation
    // will be called e.g. `C::batch_normalize()` or `C::normalize_batch()`
    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base>;

    // Instead of calling `normalize_outputs` with a single input, this is a tiny bit faster
    fn normalize_output(projective_output: Self::Codomain) -> Self::CodomainNormalized
    where
        Self::Codomain:
            EntrywiseMap<Self::MsmOutput, Output<Self::Base> = Self::CodomainNormalized>,
    {
        // 1. Collect all elements into a Vec
        let msm_vec: Vec<Self::MsmOutput> = projective_output.clone().into_iter().collect();
        // TODO: want projective_output.iter().cloned().collect();

        // 2. Apply batch_normalize
        let normalized_vec: Vec<Self::Base> = Self::batch_normalize(msm_vec);

        // 3. Replace elements in projective_output with normalized values
        let mut iter = normalized_vec.into_iter();

        projective_output.map(|_t| iter.next().expect("Not enough elements, somehow"))
    }

    fn normalize_outputs(projective_outputs: Vec<Self::Codomain>) -> Vec<Self::CodomainNormalized>
    where
        Self::Codomain:
            EntrywiseMap<Self::MsmOutput, Output<Self::Base> = Self::CodomainNormalized>,
    {
        // 1. Collect (codomain, its MsmOutput vec) for each so we can rebuild shapes later
        let outputs_with_flat_outputs: Vec<(Self::Codomain, Vec<Self::MsmOutput>)> =
            projective_outputs
                .into_iter()
                .map(|c| {
                    let flat_output_vec: Vec<Self::MsmOutput> = c.clone().into_iter().collect();
                    (c, flat_output_vec)
                })
                .collect();

        // 2. Flatten all elements into one Vec and normalize just once
        let all_outputs: Vec<Self::MsmOutput> = outputs_with_flat_outputs
            .iter()
            .flat_map(|(_, flat_output_vec)| flat_output_vec.clone())
            .collect();
        let normalized_output_vec: Vec<Self::Base> = Self::batch_normalize(all_outputs);
        let mut iter = normalized_output_vec.into_iter();

        // 3. Rebuild each CodomainNormalized from the single normalized slice
        outputs_with_flat_outputs
            .into_iter()
            .map(|(projective_output, _)| {
                projective_output.map(|_t| iter.next().expect("Not enough elements, somehow"))
            })
            .collect()
    }
}

// Implements FixedBaseMsms for the LiftHomomorphism wrapper.
// This allows us to perform multi-scalar multiplications (MSM) on a "lifted" homomorphism
// by delegating the actual MSM computation to the underlying homomorphism type `H`.
impl<H, LargerDomain> Trait for homomorphism::LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    type Base = H::Base;
    type CodomainShape<T>
        = H::CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = H::MsmOutput;
    type Scalar = H::Scalar;

    fn msm_terms(
        &self,
        input: &Self::Domain,
    ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
        let projected = (self.projection)(input);
        self.hom.msm_terms(&projected)
    }

    fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
        H::msm_eval(input)
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        H::batch_normalize(msm_output)
    }
}

impl<H, LargerDomain> sigma_protocol::CurveGroupTrait
    for homomorphism::LiftHomomorphism<H, LargerDomain>
where
    H: sigma_protocol::CurveGroupTrait,
    LargerDomain: Witness<<H::Group as PrimeGroup>::ScalarField>,
{
    type Group = H::Group;

    fn dst(&self) -> Vec<u8> {
        homomorphism::domain_separate_dsts(b"Lift(", &[self.hom.dst()], b")")
    }
}
