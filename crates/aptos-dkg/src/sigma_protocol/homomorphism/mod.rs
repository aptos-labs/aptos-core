// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError};
pub use ark_std::io::Write;

/// A `Homomorphism` represents a structure-preserving map between algebraic objects.
///
/// Formally, it is a function from a `Domain` to a `Codomain` that preserves
/// the relevant algebraic structure (e.g. group, ring, or module operations).
///
/// In the context of sigma protocols, homomorphisms are the key building blocks:
/// they capture the algebraic relations that proofs are designed to demonstrate.
pub trait Trait {
    type Domain: Sync;
    type Codomain: Sync;

    fn apply(&self, element: &Self::Domain) -> Self::Codomain;
}

/// `LiftHomomorphism` adapts a homomorphism `H` defined on some `Domain`
/// so that it can act on a larger `LargerDomain` by precomposing `H`
/// with a natural projection map `π`, which should also be a homomorphism.
///
/// In other words, given:
/// - a homomorphism `h: Domain -> Codomain`
/// - another homomorphism `π: LargerDomain -> Domain`
///
/// `LiftHomomorphism` represents the composed homomorphism:
/// `h ∘ π : LargerDomain -> Codomain`.
///
/// # Example
///
/// A common case is when `LargerDomain` is a Cartesian product type like `X × Y`
/// and the projection is `(x, y) ↦ x`. Then `LiftHomomorphism`
/// lets `h` act on the first component of the pair, so `(h ∘ π)(x,y) = h(x)`.
///
/// Naturally this method extends to composing arbitrary homomorphisms,
/// but we don't need that formalism for now.
pub struct LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    pub hom: H,
    pub projection: fn(&LargerDomain) -> H::Domain,
}

/// Implements `Homomorphism` for `LiftHomomorphism` by composing
/// the original homomorphism with the projection.
///
/// That is, applying `LiftHomomorphism` to an input `x` is equivalent to
/// first computing `projection(x)` and then applying `hom` to the result.
impl<H, LargerDomain: Sync> Trait for LiftHomomorphism<H, LargerDomain>
where
    H: Trait,
{
    type Codomain = H::Codomain;
    type Domain = LargerDomain;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let projected = (self.projection)(input);
        self.hom.apply(&projected)
    }
}

/// `TupleHomomorphism` combines two homomorphisms with the same domain
/// into a single homomorphism that outputs a tuple of codomains.
///
/// Formally, given:
/// - `h1: Domain -> Codomain1`
/// - `h2: Domain -> Codomain2`
///
/// we obtain a new homomorphism `h: Domain -> (Codomain1, Codomain2)` defined by
/// `h(x) = (h1(x), h2(x))`.
///
/// In category-theoretic terms, this is the composition of the diagonal map
/// `Δ: Domain -> Domain × Domain` with the product map `h1 × h2`.
pub struct TupleHomomorphism<H1, H2>
where
    H1: Trait,
    H2: Trait<Domain = H1::Domain>,
{
    pub hom1: H1,
    pub hom2: H2,
}

/// Implements `Homomorphism` for `TupleHomomorphism` by applying both
/// component homomorphisms to the same input and returning their results
/// as a tuple.
///
/// In other words, for input `x: Domain`, this produces
/// `(hom1(x), hom2(x))`.
impl<H1, H2> Trait for TupleHomomorphism<H1, H2>
where
    H1: Trait,
    H2: Trait<Domain = H1::Domain>,
{
    type Codomain = (H1::Codomain, H2::Codomain);
    type Domain = H1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        (self.hom1.apply(x), self.hom2.apply(x))
    }
}

/// Temporary workaround because stable Rust does not yet support associated type defaults.
pub trait IsMsmInput<B, S> {
    /// Returns a reference to the slice of base elements in this MSM input.
    fn bases(&self) -> &[B];

    /// Returns a reference to the slice of scalar elements in this MSM input.
    fn scalars(&self) -> &[S];
}

/// Represents the input to a multi-scalar multiplication (MSM):
/// a collection of bases and corresponding scalars.
#[derive(CanonicalSerialize, CanonicalDeserialize, Debug, Clone, PartialEq, Eq)]
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

/// A trait for types that support **entrywise mapping** over their contents.
///
/// Given a value of this type, you can apply a function to each "entry" independently,
/// producing a new value of the same shape but possibly with a different inner type.
pub trait EntrywiseMap<T>: Sized {
    /// The resulting type after mapping the inner elements to type `U`.
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Eq>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Eq;
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
pub trait FixedBaseMsms: Trait {
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
        + IsMsmInput<Self::Base, Self::Scalar>
        + Eq;

    /// The output type of evaluating an MSM. `Codomain` should equal `CodomainShape<MsmOutput>`, which can't be enforced directly with the current version of Rust
    type MsmOutput: CanonicalSerialize + CanonicalDeserialize + Clone + Eq;

    /// The "shape" of the homomorphism's output, parameterized by an inner type `T`.
    // TODO: type CodomainShape<T>: for<'a> IntoIterator<Item = &'a T> + 'static;
    type CodomainShape<T>: EntrywiseMap<T, Output<T> = Self::CodomainShape<T>>
        + IntoIterator<Item = T>
        + CanonicalSerialize
        + Clone
        + Eq
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq;

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

impl<H, LargerDomain: Sync> FixedBaseMsms for LiftHomomorphism<H, LargerDomain>
where
    H: FixedBaseMsms,
{
    type Base = H::Base;
    type CodomainShape<T>
        = H::CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq;
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

/// A wrapper to combine the codomain shapes of two homomorphisms into a single type.
///
/// This is necessary because Rust tuples do **not** inherit traits like `IntoIterator`,
/// but `FixedBaseMsms::CodomainShape<T>` require them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TupleCodomainShape<A, B>(pub A, pub B);

impl<A, B> CanonicalSerialize for TupleCodomainShape<A, B>
where
    A: CanonicalSerialize,
    B: CanonicalSerialize,
{
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.0.serialize_with_mode(&mut writer, compress)?;
        self.1.serialize_with_mode(&mut writer, compress)?;
        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        self.0.serialized_size(compress) + self.1.serialized_size(compress)
    }
}

impl<T, A, B> IntoIterator for TupleCodomainShape<A, B>
where
    A: IntoIterator<Item = T>,
    B: IntoIterator<Item = T>,
{
    type IntoIter = std::iter::Chain<A::IntoIter, B::IntoIter>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter().chain(self.1.into_iter())
    }
}

impl<T, A, B> EntrywiseMap<T> for TupleCodomainShape<A, B>
where
    A: EntrywiseMap<T>,
    B: EntrywiseMap<T>,
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Eq> =
        TupleCodomainShape<A::Output<U>, B::Output<U>>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Eq,
    {
        TupleCodomainShape(self.0.map(&f), self.1.map(f))
    }
}

/// Implementation of `FixedBaseMsms` for a tuple of two homomorphisms.
///
/// This allows combining two homomorphisms that share the same `Domain`.
/// For simplicity, we currently require that the MSM types (`MsmInput` and `MsmOutput`) match;
/// this ensures compatibility with batch verification in a Σ-protocol and may be relaxed in the future. Similarly, we **implicitly** that the two msm_eval methods are identical.
///
/// The codomain shapes of the two homomorphisms are combined using `TupleCodomainShape`.
impl<H1, H2> FixedBaseMsms for TupleHomomorphism<H1, H2>
where
    H1: FixedBaseMsms,
    H2: FixedBaseMsms<
        Domain = H1::Domain,
        Scalar = H1::Scalar,
        Base = H1::Base,
        MsmInput = H1::MsmInput,
        MsmOutput = H1::MsmOutput,
    >,
{
    type Base = H1::Base;
    type CodomainShape<T>
        = TupleCodomainShape<H1::CodomainShape<T>, H2::CodomainShape<T>>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq;
    type MsmInput = H1::MsmInput;
    type MsmOutput = H1::MsmOutput;
    type Scalar = H1::Scalar;

    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        TupleCodomainShape(terms1, terms2)
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        H1::msm_eval(bases, scalars)
    }
}

// Codomain example

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, PartialEq, Eq)]
pub struct TrivialShape<T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq>(pub T); // TODO: this is a copy-paste...

// Implement EntrywiseMap for the wrapper
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq> EntrywiseMap<T>
    for TrivialShape<T>
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Eq> = TrivialShape<U>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Eq,
    {
        TrivialShape(f(self.0))
    }
}

impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Eq> IntoIterator for TrivialShape<T> {
    type IntoIter = std::iter::Once<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once(self.0)
    }
}
