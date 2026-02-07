// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{homomorphism, homomorphism::EntrywiseMap, Witness},
};
use aptos_crypto::arkworks::msm::IsMsmInput;
use ark_ec::CurveGroup;
use ark_ff::Zero;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::fmt::Debug;

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
    CodomainNormalized = Self::CodomainShape<<Self::MsmInput as IsMsmInput>::Base>,
>
{
    // Type representing the scalar used in the `MsmInput`s. Convenient to repeat here, and currently used in `prove_homomorphism()` where it could be replaced by e.g. `C::ScalarField`... (or maybe by going inside of MsmInput)
    type Scalar: ark_ff::PrimeField; // Probably need less here but this what it'll be in practice

    /// Type representing a single MSM input (a set of bases and scalars). Normally, this would default
    /// to `MsmInput<..., ...>`, but stable Rust does not yet support associated type defaults,
    /// hence we introduce a trait `IsMsmInput` and struct `MsmInput` elsewhere.
    type MsmInput: CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + IsMsmInput<Scalar = Self::Scalar>
        + Debug
        + Eq;

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
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput>;

    /// Evaluates a single MSM instance given slices of bases and scalars. Current instantiations always use E::G1Affine
    /// for the base, but we might want to use enums for the base and output in the future.
    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput;

    /// Applies `msm_eval` elementwise to a collection of MSM inputs.
    fn apply_msm(
        &self, // TODO: remove this
        msms: Self::CodomainShape<Self::MsmInput>,
    ) -> Self::CodomainShape<Self::MsmOutput>
    where
        Self::CodomainShape<Self::MsmInput>: EntrywiseMap<
            Self::MsmInput,
            Output<Self::MsmOutput> = Self::CodomainShape<Self::MsmOutput>,
        >,
    {
        msms.map(|msm_input| Self::msm_eval(msm_input))
    }

    fn batch_normalize(
        msm_output: Vec<Self::MsmOutput>,
    ) -> Vec<<Self::MsmInput as IsMsmInput>::Base>;

    fn normalize_output(projective_output: &Self::Codomain) -> Self::CodomainNormalized
    where
        Self::Codomain: EntrywiseMap<
            Self::MsmOutput,
            Output<<Self::MsmInput as IsMsmInput>::Base> = Self::CodomainNormalized,
        >,
    {
        // 1. Collect all elements into a Vec
        let msm_vec: Vec<Self::MsmOutput> = projective_output.clone().into_iter().collect();

        // 2. Apply batch_normalize
        let normalized_vec: Vec<<Self::MsmInput as IsMsmInput>::Base> =
            Self::batch_normalize(msm_vec);

        // 3. Replace elements in projective_output with normalized values
        let mut iter = normalized_vec.into_iter();

        projective_output
            .clone()
            .map(|_t| iter.next().expect("Not enough elements, somehow"))
    }
}

// Implements FixedBaseMsms for the LiftHomomorphism wrapper.
// This allows us to perform multi-scalar multiplications (MSM) on a "lifted" homomorphism
// by delegating the actual MSM computation to the underlying homomorphism type `H`.
impl<H, LargerDomain> Trait for homomorphism::LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    type CodomainShape<T>
        = H::CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = H::MsmInput;
    type MsmOutput = H::MsmOutput;
    type Scalar = H::Scalar;

    /// Returns the MSM terms corresponding to a given homomorphism input. The output is shaped so that applying the MSM elementwise yields the homomorphism output.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let projected = (self.projection)(input);
        self.hom.msm_terms(&projected)
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        H::msm_eval(input)
    }

    fn batch_normalize(
        msm_output: Vec<Self::MsmOutput>,
    ) -> Vec<<Self::MsmInput as IsMsmInput>::Base> {
        H::batch_normalize(msm_output)
    }
}

impl<C: CurveGroup, H, LargerDomain> sigma_protocol::Trait<C>
    for homomorphism::LiftHomomorphism<H, LargerDomain>
where
    H: sigma_protocol::Trait<C>,
    LargerDomain: Witness<C::ScalarField>,
{
    fn dst(&self) -> Vec<u8> {
        let mut dst = Vec::new();

        let dst_original = self.hom.dst();

        // Domain-separate them properly so concatenation is unambiguous.
        // Prefix with their lengths so [a|b] and [ab|] don't collide.
        dst.extend_from_slice(b"Lift(");
        dst.extend_from_slice(&(dst_original.len() as u32).to_be_bytes());
        dst.extend_from_slice(&dst_original);
        dst.extend_from_slice(b")");

        dst
    }
}
