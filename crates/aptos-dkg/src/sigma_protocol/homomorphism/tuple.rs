// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, EntrywiseMap},
        traits::{prove_homomorphism, verifier_challenges_with_length},
        Proof,
    },
};
use anyhow::ensure;
use aptos_crypto::arkworks::msm::MsmInput;
use ark_ec::pairing::Pairing;
use ark_ff::Zero;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, Read, SerializationError, Valid,
};
use ark_std::io::Write;
use rand_core::{CryptoRng, RngCore};
use serde::Serialize;
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

// We need to add `E: Pairing` because of the sigma protocol implementation below, Rust wouldn't allow that otherwise
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
/// In other words, for input `x: Domain`, this produces `(hom1(x), hom2(x))`.
/// For technical reasons, we then put the output inside a wrapper.
impl<H1, H2> homomorphism::Trait for TupleHomomorphism<H1, H2>
where
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
    H1::Codomain: CanonicalSerialize + CanonicalDeserialize,
    H2::Codomain: CanonicalSerialize + CanonicalDeserialize,
{
    type Codomain = TupleCodomainShape<H1::Codomain, H2::Codomain>;
    type CodomainNormalized = TupleCodomainShape<H1::CodomainNormalized, H2::CodomainNormalized>;
    type Domain = H1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        TupleCodomainShape(self.hom1.apply(x), self.hom2.apply(x))
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        TupleCodomainShape(
            H1::normalize(&self.hom1, value.0),
            H2::normalize(&self.hom2, value.1),
        )
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
    type CodomainNormalized = TupleCodomainShape<H1::CodomainNormalized, H2::CodomainNormalized>;
    type Domain = H1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        TupleCodomainShape(self.hom1.apply(x), self.hom2.apply(x))
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        TupleCodomainShape(
            H1::normalize(&self.hom1, value.0),
            H2::normalize(&self.hom2, value.1),
        )
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

    fn map<U, F>(self, mut f: F) -> Self::Output<U>
    where
        F: FnMut(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        TupleCodomainShape(self.0.map(&mut f), self.1.map(f))
    }
}

/// Implementation of `FixedBaseMsms` for a tuple of two homomorphisms.
///
/// This allows combining two homomorphisms that share the same `Domain`.
/// For simplicity, we currently require that the MSM types (Base, Scalar, MsmOutput) match;
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
        Base = H1::Base,
        Scalar = H1::Scalar,
        MsmOutput = H1::MsmOutput,
    >,
{
    type Base = H1::Base;
    type CodomainShape<T>
        = TupleCodomainShape<H1::CodomainShape<T>, H2::CodomainShape<T>>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = H1::MsmOutput;
    type Scalar = H1::Scalar;

    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(
        &self,
        input: &Self::Domain,
    ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        TupleCodomainShape(terms1, terms2)
    }

    fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
        H1::msm_eval(input)
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        H1::batch_normalize(msm_output)
    }
}

impl<H1, H2> sigma_protocol::CurveGroupTrait for TupleHomomorphism<H1, H2>
where
    H1: sigma_protocol::CurveGroupTrait,
    H2: sigma_protocol::CurveGroupTrait<Group = H1::Group>,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    type Group = H1::Group;

    /// Concatenate the DSTs of the two homomorphisms, plus some
    /// additional metadata to ensure uniqueness.
    fn dst(&self) -> Vec<u8> {
        homomorphism::domain_separate_dsts(
            b"TupleHomomorphism(",
            &[self.hom1.dst(), self.hom2.dst()],
            b")",
        )
    }
}

/// Slightly hacky implementation of a sigma protocol for `PairingTupleHomomorphism`
///
/// We need `E: Pairing` here because the sigma protocol needs to know which curves `H1` and `H2` are working over
impl<E: Pairing, H1, H2> PairingTupleHomomorphism<E, H1, H2>
where
    H1: sigma_protocol::CurveGroupTrait<Group = E::G1>,
    H2: sigma_protocol::CurveGroupTrait<Group = E::G2>,
    H2: fixed_base_msms::Trait<Domain = H1::Domain>,
{
    fn dst(&self) -> Vec<u8> {
        homomorphism::domain_separate_dsts(
            b"PairingTupleHomomorphism(",
            &[self.hom1.dst(), self.hom2.dst()],
            b")",
        )
    }

    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(
        &self,
        input: &H1::Domain,
    ) -> (
        H1::CodomainShape<MsmInput<H1::Base, H1::Scalar>>,
        H2::CodomainShape<MsmInput<H2::Base, H2::Scalar>>,
    ) {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        (terms1, terms2)
    }

    // TODO: maybe remove, see comment below
    pub fn check_first_msm_eval(
        &self,
        input: MsmInput<H1::Base, H1::Scalar>,
    ) -> anyhow::Result<()> {
        let result = H1::msm_eval(input);
        ensure!(result == H1::MsmOutput::zero());
        Ok(())
    }

    // TODO: Doesn't get used atm... so we're implicitly mixing different MSM code :-/
    pub fn check_second_msm_eval(
        &self,
        input: MsmInput<H2::Base, H2::Scalar>,
    ) -> anyhow::Result<()> {
        let result = H2::msm_eval(input);
        ensure!(result == H2::MsmOutput::zero());
        Ok(())
    }

    pub fn prove<Ct: Serialize, R: RngCore + CryptoRng>(
        &self,
        witness: &<Self as homomorphism::Trait>::Domain,
        statement: <Self as homomorphism::Trait>::Codomain,
        cntxt: &Ct, // for SoK purposes
        rng: &mut R,
    ) -> (
        Proof<H1::Scalar, Self>,
        <Self as homomorphism::Trait>::CodomainNormalized,
    ) {
        prove_homomorphism(self, witness, statement, cntxt, true, rng, &self.dst())
    }

    // Probably not using this atm
    #[allow(non_snake_case)]
    pub fn verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &<Self as homomorphism::Trait>::CodomainNormalized,
        proof: &Proof<H1::Scalar, H>, // Would like to set &Proof<E, Self>, but that ties the lifetime of H to that of Self, but we'd like it to be eg static
        cntxt: &Ct,
        rng: &mut R,
    ) -> anyhow::Result<()>
    where
        H: homomorphism::Trait<
            Domain = <Self as homomorphism::Trait>::Domain,
            CodomainNormalized = <Self as homomorphism::Trait>::CodomainNormalized,
        >,
    {
        let (first_msm_terms, second_msm_terms) =
            self.msm_terms_for_verify::<_, H, _>(public_statement, proof, cntxt, None, rng);

        let first_msm_result = H1::msm_eval(first_msm_terms);
        ensure!(first_msm_result == H1::MsmOutput::zero());

        let second_msm_result = H2::msm_eval(second_msm_terms);
        ensure!(second_msm_result == H2::MsmOutput::zero());

        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn msm_terms_for_verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &<Self as homomorphism::Trait>::CodomainNormalized,
        proof: &Proof<H1::Scalar, H>,
        cntxt: &Ct,
        number_of_beta_powers: Option<(usize, usize)>, // (len1, len2); None => compute from statement (clones)
        rng: &mut R,
    ) -> (
        MsmInput<H1::Base, H1::Scalar>,
        MsmInput<H2::Base, H2::Scalar>,
    )
    where
        H: homomorphism::Trait<
            Domain = <Self as homomorphism::Trait>::Domain,
            CodomainNormalized = <Self as homomorphism::Trait>::CodomainNormalized,
        >, // need this?
    {
        let prover_first_message = proof
            .prover_commitment()
            .expect("Missing implementation - expected commitment, not challenge");
        let (len1, len2) = number_of_beta_powers.unwrap_or_else(|| {
            (
                public_statement.0.clone().into_iter().count(),
                public_statement.1.clone().into_iter().count(),
            )
        });
        let (c, powers_of_beta) = verifier_challenges_with_length::<_, H1::Scalar, _, _>(
            cntxt,
            self,
            public_statement,
            prover_first_message,
            &self.dst(),
            len1 + len2,
            rng,
        );
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

// TODO: mediocre idea for Shplonked: do another "custom" sigma protocol trait for:
// a tuple with on the LHS an ordinary fixed-base-msms
// on the RHS, something that might be a homomorphism from F^k -> F
// so maybe if we define a custom sigma protocol trait for such a hom
// then the tuple version can be automatic just as above
