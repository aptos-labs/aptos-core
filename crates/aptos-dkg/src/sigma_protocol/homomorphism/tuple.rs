// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, EntrywiseMap},
    },
};
use ark_ec::CurveGroup;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid,
};
pub use ark_std::io::Write;
use std::fmt::Debug;

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
#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct TupleHomomorphism<H1, H2, const HOMOG: bool>
where
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    pub hom1: H1,
    pub hom2: H2,
}

/// Implements `Homomorphism` for `TupleHomomorphism` by applying both
/// component homomorphisms to the same input and returning their results
/// as a tuple.
///
/// In other words, for input `x: Domain`, this produces
/// `(hom1(x), hom2(x))`. For technical reasons, we then put the output inside a wrapper.
impl<H1, H2, const HOMOG: bool> homomorphism::Trait for TupleHomomorphism<H1, H2, HOMOG>
where
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
    H1::Codomain: CanonicalSerialize + CanonicalDeserialize,
    H2::Codomain: CanonicalSerialize + CanonicalDeserialize,
{
    type Codomain = TupleCodomainShape<H1::Codomain, H2::Codomain>;
    type Domain = H1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        TupleCodomainShape(self.hom1.apply(x), self.hom2.apply(x))
    }
}

/// A wrapper to combine the codomain shapes of two homomorphisms into a single type.
///
/// This is necessary because Rust tuples do **not** inherit traits like `IntoIterator`,
/// but `fixed_base_msms::CodomainShape<T>` requires them.
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

impl<A, B> CanonicalDeserialize for TupleCodomainShape<A, B>
where
    A: CanonicalDeserialize,
    B: CanonicalDeserialize,
{
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: ark_serialize::Validate,
    ) -> Result<Self, SerializationError> {
        let a = A::deserialize_with_mode(&mut reader, compress, validate)?;
        let b = B::deserialize_with_mode(&mut reader, compress, validate)?;
        Ok(Self(a, b))
    }
}

impl<A, B> Valid for TupleCodomainShape<A, B>
where
    A: Valid,
    B: Valid,
{
    fn check(&self) -> Result<(), SerializationError> {
        self.0.check()?;
        self.1.check()?;
        Ok(())
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
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> =
        TupleCodomainShape<A::Output<U>, B::Output<U>>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        TupleCodomainShape(self.0.map(&f), self.1.map(f))
    }
}

// TODO: Maak een DifferingTupleHomomorphism ???
/// Implementation of `FixedBaseMsms` for a tuple of two homomorphisms.
///
/// This allows combining two homomorphisms that share the same `Domain`.
/// For simplicity, we currently require that the MSM types (`MsmInput` and `MsmOutput`) match;
/// this ensures compatibility with batch verification in a Σ-protocol and may be relaxed in the future.
/// For the moment, we **implicitly** assume that the two msm_eval methods are identical, but is probably
/// not necessary through enums.
///
/// The codomain shapes of the two homomorphisms are combined using `TupleCodomainShape`.
impl<H1, H2> fixed_base_msms::Trait for TupleHomomorphism<H1, H2, true>
where
    H1: fixed_base_msms::Trait,
    H2: fixed_base_msms::Trait<
        Domain = H1::Domain,
        MsmInput = H1::MsmInput,
        MsmOutput = H1::MsmOutput,
    >,
{
    type CodomainShape<T>
        = TupleCodomainShape<H1::CodomainShape<T>, H2::CodomainShape<T>>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = H1::MsmInput;
    type MsmOutput = H1::MsmOutput;

    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        TupleCodomainShape(terms1, terms2)
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        H1::msm_eval(input)
    }
}

impl<C: CurveGroup, H1, H2> sigma_protocol::Trait<C> for TupleHomomorphism<H1, H2, true>
where
    H1: sigma_protocol::Trait<C>,
    H2: sigma_protocol::Trait<C>,
    H2: fixed_base_msms::Trait<Domain = H1::Domain, MsmInput = H1::MsmInput>,
{
    /// Concatenate the DSTs of the two homomorphisms, plus some
    /// additional metadata to ensure uniqueness.
    fn dst(&self) -> Vec<u8> {
        let mut dst = Vec::new();

        let dst1 = self.hom1.dst();
        let dst2 = self.hom2.dst();

        // Domain-separate them properly so concatenation is unambiguous.
        // Prefix with their lengths so [a|b] and [ab|] don't collide.
        dst.extend_from_slice(b"TupleHomomorphism(");
        dst.extend_from_slice(&(dst1.len() as u32).to_be_bytes());
        dst.extend_from_slice(&dst1);
        dst.extend_from_slice(&(dst2.len() as u32).to_be_bytes());
        dst.extend_from_slice(&dst2);
        dst.extend_from_slice(b")");

        dst
    }
}

// mwa je moet gewoon een aparte fixed_base_msms::Trait gaan maken... dan is HOMOG misschien niet meer nodig
impl<H1, H2> fixed_base_msms::Trait for TupleHomomorphism<H1, H2, false>
where
    H1: fixed_base_msms::Trait,
    H2: fixed_base_msms::Trait<
        Domain = H1::Domain,
        MsmInput = H1::MsmInput,
        MsmOutput = H1::MsmOutput,
    >,
{
    type CodomainShape<T>
        = TupleCodomainShape<H1::CodomainShape<T>, H2::CodomainShape<T>>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = H1::MsmInput;
    type MsmOutput = H1::MsmOutput;

    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        TupleCodomainShape(terms1, terms2)
    }

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        H1::msm_eval(input) // !!!!!!!!!!!!!! doesn't make sense, should put `fn eval` back,,, which is already in HomTrait
    }
}
