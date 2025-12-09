// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{homomorphism, homomorphism::EntrywiseMap, Witness},
};
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::fmt::Debug;
use aptos_crypto::arkworks::msm::IsMsmInput;

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
pub trait Trait: homomorphism::Trait<Codomain = Self::CodomainShape<Self::MsmOutput>> {
    /// Type representing a single MSM input (a set of bases and scalars). Normally, this would default
    /// to `MsmInput<Self::Base, Self::Scalar>`, but stable Rust does not yet support associated type defaults,
    /// hence we introduce a trait `IsMsmInput` and struct `MsmInput` elsewhere.
    type MsmInput: CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + IsMsmInput
        + Debug
        + Eq;

    /// The output type of evaluating an MSM. `Codomain` should equal `CodomainShape<MsmOutput>`, in the current version
    /// of the code. In a future version where MsmOutput might be an enum (E::G1 or E::G2), Codomain should probably follow suit.
    /// (TODO: Think this over)
    type MsmOutput: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;

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
    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput; // why not Self::MsmInput as input?

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
        msms.map(|msm_input| Self::msm_eval(msm_input))
    }
}

// Alternate version for tuple MSMs with incompatible types, namely whose output is G1 and G2
pub trait InhomogeneousTrait: homomorphism::Trait<Codomain = (Self::FirstCodomainShape<Self::FirstMsmOutput>, Self::SecondCodomainShape<Self::SecondMsmOutput>)> {
    /// Type representing a single MSM input (a set of bases and scalars). Normally, this would default
    /// to `MsmInput<Self::Base, Self::Scalar>`, but stable Rust does not yet support associated type defaults,
    /// hence we introduce a trait `IsMsmInput` and struct `MsmInput` below.
    type FirstMsmInput: CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + IsMsmInput
        + Debug
        + Eq;
    type SecondMsmInput: CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + IsMsmInput
        + Debug
        + Eq;

    /// The output type of evaluating an MSM. `Codomain` should equal `CodomainShape<MsmOutput>`, in the current version
    /// of the code. In a future version where MsmOutput might be an enum (E::G1 or E::G2), Codomain should probably follow suit.
    /// (TODO: Think this over)
    type FirstMsmOutput: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type SecondMsmOutput: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;

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
    type FirstCodomainShape<T>: IntoIterator<Item = T>
        + CanonicalSerialize
        + CanonicalDeserialize
        + Clone
        + Debug
        + Eq
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type SecondCodomainShape<T>: IntoIterator<Item = T>
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
    fn msm_terms(&self, input: &Self::Domain) -> (Self::FirstCodomainShape<Self::FirstMsmInput>, Self::SecondCodomainShape<Self::SecondMsmInput>);

    /// Evaluates a single MSM instance given slices of bases and scalars. Current instantiations always use E::G1Affine
    /// for the base, but we might want to use enums for the base and output in the future.
    fn first_msm_eval(input: Self::FirstMsmInput) -> Self::FirstMsmOutput;
    fn second_msm_eval(input: Self::SecondMsmInput) -> Self::SecondMsmOutput;

    /// Applies `msm_eval` elementwise to a collection of MSM inputs.
    fn apply_msm(
        &self,
        msms: (Self::FirstCodomainShape<Self::FirstMsmInput>, Self::SecondCodomainShape<Self::SecondMsmInput>),
    ) -> (Self::FirstCodomainShape<Self::FirstMsmOutput>, Self::SecondCodomainShape<Self::SecondMsmOutput>)
    where
        Self::FirstCodomainShape<Self::FirstMsmInput>: EntrywiseMap<
            Self::FirstMsmInput,
            Output<Self::FirstMsmOutput> = Self::FirstCodomainShape<Self::FirstMsmOutput>,
        >,
        Self::SecondCodomainShape<Self::SecondMsmInput>: EntrywiseMap<
            Self::SecondMsmInput,
            Output<Self::SecondMsmOutput> = Self::SecondCodomainShape<Self::SecondMsmOutput>,
        >,
    {
        (msms.0.map(|msm_input| Self::first_msm_eval(msm_input)), msms.1.map(|msm_input| Self::second_msm_eval(msm_input)))

        //msms.map(|first_msm_input, second_msm_input| (Self::first_msm_eval(&first_msm_input.bases(), &first_msm_input.scalars()), Self::second_msm_eval(&second_msm_input.bases(), &second_msm_input.scalars())))
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

    /// Returns the MSM terms corresponding to a given homomorphism input. The output is shaped so that applying the MSM elementwise yields the homomorphism output.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let projected = (self.projection)(input);
        self.hom.msm_terms(&projected)
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        H::msm_eval(input)
    }
}

impl<E: Pairing, H, LargerDomain> sigma_protocol::Trait<E>
    for homomorphism::LiftHomomorphism<H, LargerDomain>
where
    H: sigma_protocol::Trait<E>,
    LargerDomain: Witness<E::ScalarField>,
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
