// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms::Trait, EntrywiseMap},
        Witness,
    },
};
use ark_ec::pairing::Pairing;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError};
pub use ark_std::io::Write;

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
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    pub hom1: H1,
    pub hom2: H2,
    pub dst: Vec<u8>,
    pub dst_verifier: Vec<u8>,
}

/// Implements `Homomorphism` for `TupleHomomorphism` by applying both
/// component homomorphisms to the same input and returning their results
/// as a tuple.
///
/// In other words, for input `x: Domain`, this produces
/// `(hom1(x), hom2(x))`. For technical reasons, we then put the output inside a wrapper.
impl<H1, H2> homomorphism::Trait for TupleHomomorphism<H1, H2>
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

use ark_serialize::{Read, Valid};

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
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone> =
        TupleCodomainShape<A::Output<U>, B::Output<U>>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone,
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
impl<H1, H2> Trait for TupleHomomorphism<H1, H2>
where
    H1: Trait,
    H2: Trait<
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
        T: CanonicalSerialize + CanonicalDeserialize + Clone;
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

impl<E: Pairing, H1, H2> sigma_protocol::Trait<E> for TupleHomomorphism<H1, H2>
where
    H1: Trait<MsmOutput = E::G1, Base = E::G1Affine, Scalar = E::ScalarField>,
    H2: Trait<
        Domain = H1::Domain,
        Scalar = H1::Scalar,
        Base = H1::Base,
        MsmInput = H1::MsmInput,
        MsmOutput = H1::MsmOutput,
    >,
    H1::Domain: Witness<E>,
{
    fn dst(&self) -> Vec<u8> {
        self.dst.clone()
    }

    fn dst_verifier(&self) -> Vec<u8> {
        self.dst_verifier.clone()
    }
}
