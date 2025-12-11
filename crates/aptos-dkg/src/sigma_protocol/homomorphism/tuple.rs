// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, EntrywiseMap},
    },
};
use ark_ec::{pairing::Pairing, CurveGroup};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid,
};
use ark_std::io::Write;
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
pub struct TupleHomomorphism<H1, H2>
where
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    pub hom1: H1,
    pub hom2: H2,
}

#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct PairingTupleHomomorphism<E, H1, H2>
where
    E: Pairing,
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    pub hom1: H1,
    pub hom2: H2,
    pub _pairing: std::marker::PhantomData<E>,
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

impl<E, H1, H2> homomorphism::Trait for PairingTupleHomomorphism<E, H1, H2>
where
    E: Pairing,
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

/// Implementation of `FixedBaseMsms` for a tuple of two homomorphisms.
///
/// This allows combining two homomorphisms that share the same `Domain`.
/// For simplicity, we currently require that the MSM types (`MsmInput` and `MsmOutput`) match;
/// this ensures compatibility with batch verification in a Σ-protocol and may be relaxed in the future.
/// For the moment, we **implicitly** assume that the two msm_eval methods are identical, but is probably
/// not necessary through enums.
///
/// The codomain shapes of the two homomorphisms are combined using `TupleCodomainShape`.
impl<H1, H2> fixed_base_msms::Trait for TupleHomomorphism<H1, H2>
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
    type Scalar = H1::Scalar;

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

impl<C: CurveGroup, H1, H2> sigma_protocol::Trait<C> for TupleHomomorphism<H1, H2>
where
    H1: sigma_protocol::Trait<C>,
    H2: sigma_protocol::Trait<C>,
    H2: fixed_base_msms::Trait<Domain = H1::Domain, MsmInput = H1::MsmInput>, // Huh MsmOutput = H1::MsmOutput yields compiler error??
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

use crate::sigma_protocol::{
    traits::{fiat_shamir_challenge_for_sigma_protocol, prove_homomorphism, FirstProofItem},
    Proof,
};
use anyhow::ensure;
use aptos_crypto::utils;
use ark_ff::{UniformRand, Zero};
use serde::Serialize;

// Slightly hacky implementation of a sigma protocol for `PairingTupleHomomorphism`
impl<E: Pairing, H1, H2> PairingTupleHomomorphism<E, H1, H2>
where
    H1: sigma_protocol::Trait<E::G1>,
    H2: sigma_protocol::Trait<E::G2>,
    H2: fixed_base_msms::Trait<Domain = H1::Domain>,
{
    fn dst(&self) -> Vec<u8> {
        let mut dst = Vec::new();

        let dst1 = self.hom1.dst();
        let dst2 = self.hom2.dst();

        // Domain-separate them properly so concatenation is unambiguous.
        // Prefix with their lengths so [a|b] and [ab|] don't collide.
        dst.extend_from_slice(b"PairingTupleHomomorphism(");
        dst.extend_from_slice(&(dst1.len() as u32).to_be_bytes());
        dst.extend_from_slice(&dst1);
        dst.extend_from_slice(&(dst2.len() as u32).to_be_bytes());
        dst.extend_from_slice(&dst2);
        dst.extend_from_slice(b")");

        dst
    }

    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(
        &self,
        input: &H1::Domain,
    ) -> (
        H1::CodomainShape<H1::MsmInput>,
        H2::CodomainShape<H2::MsmInput>,
    ) {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        (terms1, terms2)
    }

    pub fn prove<Ct: Serialize, R: rand_core::RngCore + rand_core::CryptoRng>(
        &self,
        witness: &<Self as homomorphism::Trait>::Domain,
        statement: &<Self as homomorphism::Trait>::Codomain,
        cntxt: &Ct, // for SoK purposes
        rng: &mut R,
    ) -> Proof<H1::Scalar, Self> {
        prove_homomorphism(self, witness, statement, cntxt, true, rng, &self.dst())
    }

    #[allow(non_snake_case)]
    pub fn verify<C: Serialize, H>(
        &self,
        public_statement: &<Self as homomorphism::Trait>::Codomain,
        proof: &Proof<H1::Scalar, H>, // Would like to set &Proof<E, Self>, but that ties the lifetime of H to that of Self, but we'd like it to be eg static
        cntxt: &C,
    ) -> anyhow::Result<()>
    where
        H: homomorphism::Trait<
            Domain = <Self as homomorphism::Trait>::Domain,
            Codomain = <Self as homomorphism::Trait>::Codomain,
        >,
    {
        let (first_msm_terms, second_msm_terms) =
            self.msm_terms_for_verify::<_, H>(public_statement, proof, cntxt);

        let first_msm_result = H1::msm_eval(first_msm_terms);
        ensure!(first_msm_result == H1::MsmOutput::zero());

        let second_msm_result = H2::msm_eval(second_msm_terms);
        ensure!(second_msm_result == H2::MsmOutput::zero());

        Ok(())
    }

    #[allow(non_snake_case)]
    fn msm_terms_for_verify<Ct: Serialize, H>(
        &self,
        public_statement: &<Self as homomorphism::Trait>::Codomain,
        proof: &Proof<H1::Scalar, H>,
        cntxt: &Ct,
    ) -> (H1::MsmInput, H2::MsmInput)
    where
        H: homomorphism::Trait<
            Domain = <Self as homomorphism::Trait>::Domain,
            Codomain = <Self as homomorphism::Trait>::Codomain,
        >, // need this?
    {
        let prover_first_message = match &proof.first_proof_item {
            FirstProofItem::Commitment(A) => A,
            FirstProofItem::Challenge(_) => {
                panic!("Missing implementation - expected commitment, not challenge")
            },
        };
        let c = fiat_shamir_challenge_for_sigma_protocol::<_, H1::Scalar, _>(
            cntxt,
            self,
            public_statement,
            &prover_first_message,
            &self.dst(),
        );

        let mut rng = ark_std::rand::thread_rng(); // TODO: make this part of the function input?
        let beta = H1::Scalar::rand(&mut rng);
        let len1 = public_statement.0.clone().into_iter().count(); // hmm maybe pass the into_iter version in merge_msm_terms?
        let len2 = public_statement.1.clone().into_iter().count();
        let powers_of_beta = utils::powers(beta, len1 + len2);
        let (first_powers_of_beta, second_powers_of_beta) = powers_of_beta.split_at(len1);

        let (first_msm_terms_of_response, second_msm_terms_of_response) = self.msm_terms(&proof.z);

        let first_input = H1::merge_msm_terms(
            first_msm_terms_of_response.into_iter().collect(),
            &prover_first_message.0,
            &public_statement.0,
            first_powers_of_beta,
            c,
        );
        let second_input = H2::merge_msm_terms(
            second_msm_terms_of_response.into_iter().collect(),
            &prover_first_message.1,
            &public_statement.1,
            second_powers_of_beta,
            c,
        );

        (first_input, second_input)
    }
}
