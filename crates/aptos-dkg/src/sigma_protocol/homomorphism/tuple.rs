// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, EntrywiseMap},
        traits::verifier_challenges_with_length,
        Proof,
    },
};
use anyhow::ensure;
use aptos_crypto::arkworks::msm::MsmInput;
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::{PrimeField, Zero};
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

// When we know that the two homomorphisms are both going to be `FixedBaseMsms` with the same curve group,
// we can perform certain optimizations in the verifier of the sigma protocol; hence we set up a separate
// struct for this case
#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct CurveGroupTupleHomomorphism<C, H1, H2>
where
    C: CurveGroup,
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    pub hom1: H1,
    pub hom2: H2,
    pub _group: std::marker::PhantomData<C>,
}

/// Shared logic for tuple homomorphisms: apply both components and normalize.
fn tuple_apply<H1, H2>(
    hom1: &H1,
    hom2: &H2,
    x: &H1::Domain,
) -> TupleCodomainShape<H1::Codomain, H2::Codomain>
where
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    TupleCodomainShape(hom1.apply(x), hom2.apply(x))
}

fn tuple_normalize<H1, H2>(
    hom1: &H1,
    hom2: &H2,
    value: TupleCodomainShape<H1::Codomain, H2::Codomain>,
) -> TupleCodomainShape<H1::CodomainNormalized, H2::CodomainNormalized>
where
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    TupleCodomainShape(H1::normalize(hom1, value.0), H2::normalize(hom2, value.1))
}

fn tuple_statement_lengths<A, B>(stmt: &TupleCodomainShape<A, B>) -> (usize, usize)
where
    A: Clone + IntoIterator,
    B: Clone + IntoIterator,
{
    (
        stmt.0.clone().into_iter().count(), // TODO: cloning is not ideal here
        stmt.1.clone().into_iter().count(),
    )
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
        tuple_apply(&self.hom1, &self.hom2, x)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        tuple_normalize(&self.hom1, &self.hom2, value)
    }
}

impl<C, H1, H2> homomorphism::Trait for CurveGroupTupleHomomorphism<C, H1, H2>
where
    C: CurveGroup,
    H1: homomorphism::Trait,
    H2: homomorphism::Trait<Domain = H1::Domain>,
    H1::Codomain: CanonicalSerialize + CanonicalDeserialize,
    H2::Codomain: CanonicalSerialize + CanonicalDeserialize,
{
    type Codomain = TupleCodomainShape<H1::Codomain, H2::Codomain>;
    type CodomainNormalized = TupleCodomainShape<H1::CodomainNormalized, H2::CodomainNormalized>;
    type Domain = H1::Domain;

    fn apply(&self, x: &Self::Domain) -> Self::Codomain {
        tuple_apply(&self.hom1, &self.hom2, x)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        tuple_normalize(&self.hom1, &self.hom2, value)
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

/// Implementation of `FixedBaseMsms` for a tuple of two homomorphisms over the same group.
///
/// This allows combining two homomorphisms that share the same `Domain`.
/// Because they share the same group we **implicitly** assume that the two msm_eval methods
/// are identical. // TODO: maybe derive it automatically?
///
/// The codomain shapes of the two homomorphisms are combined using `TupleCodomainShape`.
impl<C, H1, H2> fixed_base_msms::Trait for CurveGroupTupleHomomorphism<C, H1, H2>
where
    C: CurveGroup,
    H1: fixed_base_msms::Trait<Base = C::Affine, Scalar = C::ScalarField, MsmOutput = C>,
    H2: fixed_base_msms::Trait<
        Domain = H1::Domain,
        Base = C::Affine,
        Scalar = C::ScalarField,
        MsmOutput = C,
    >,
{
    type Base = C::Affine;
    type CodomainShape<T>
        = TupleCodomainShape<H1::CodomainShape<T>, H2::CodomainShape<T>>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = C;
    type Scalar = C::ScalarField;

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

impl<C, H1, H2> sigma_protocol::CurveGroupTrait for CurveGroupTupleHomomorphism<C, H1, H2>
where
    C: CurveGroup,
    H1: sigma_protocol::CurveGroupTrait<Group = C>,
    H2: sigma_protocol::CurveGroupTrait<Group = C>,
    H2: homomorphism::Trait<Domain = H1::Domain>,
{
    type Group = C;

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

impl<H1, H2> sigma_protocol::Trait for TupleHomomorphism<H1, H2>
where
    H1: sigma_protocol::Trait,
    H2: sigma_protocol::Trait<Scalar = H1::Scalar>,
    H2: homomorphism::Trait<Domain = H1::Domain>,
    H1::Codomain: CanonicalSerialize + CanonicalDeserialize,
    H2::Codomain: CanonicalSerialize + CanonicalDeserialize,
{
    type Scalar = H1::Scalar;

    fn dst(&self) -> Vec<u8> {
        homomorphism::domain_separate_dsts(
            b"GenericTupleHomomorphism(",
            &[self.hom1.dst(), self.hom2.dst()],
            b")",
        )
    }

    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_commitment: &Self::CodomainNormalized,
        challenge: Self::Scalar,
        response: &Self::Domain,
        _verifier_batch_size: Option<usize>, // not ideal, should be splitting it...
        rng: &mut R,
    ) -> anyhow::Result<()> {
        let (stmt1, stmt2) = (&public_statement.0, &public_statement.1);
        let (commit1, commit2) = (&prover_commitment.0, &prover_commitment.1);
        self.hom1
            .verify_with_challenge(stmt1, commit1, challenge, response, None, rng)?;
        self.hom2
            .verify_with_challenge(stmt2, commit2, challenge, response, None, rng)?;
        Ok(())
    }
}

/// Extension methods for `TupleHomomorphism` when both components implement `CurveGroupTrait`
/// with the same scalar field (e.g. G1 and G2 of a pairing).
impl<H1, H2, F> TupleHomomorphism<H1, H2>
where
    F: PrimeField,
    H1: sigma_protocol::CurveGroupTrait<Group: PrimeGroup<ScalarField = F>>,
    H2: sigma_protocol::CurveGroupTrait<Domain = H1::Domain, Group: PrimeGroup<ScalarField = F>>,
{
    /// Returns the MSM terms for each homomorphism, combined into a tuple.
    fn msm_terms(
        &self,
        input: &H1::Domain,
    ) -> (
        H1::CodomainShape<MsmInput<H1::Base, F>>,
        H2::CodomainShape<MsmInput<H2::Base, F>>,
    ) {
        let terms1 = self.hom1.msm_terms(input);
        let terms2 = self.hom2.msm_terms(input);
        (terms1, terms2)
    }

    /// Merges MSM terms for both components; shared by verify_with_challenge and msm_terms_for_verify.
    fn merge_msm_terms_for_verify(
        &self,
        response: &H1::Domain,
        prover_commitment: &TupleCodomainShape<H1::CodomainNormalized, H2::CodomainNormalized>,
        public_statement: &TupleCodomainShape<H1::CodomainNormalized, H2::CodomainNormalized>,
        challenge: F,
        powers_of_beta: &[F],
        len1: usize,
    ) -> (MsmInput<H1::Base, F>, MsmInput<H2::Base, F>) {
        let (first_powers, second_powers) = powers_of_beta.split_at(len1);
        let (first_terms, second_terms) = self.msm_terms(response);
        let first_input = H1::merge_msm_terms(
            first_terms.into_iter().collect(),
            &prover_commitment.0,
            &public_statement.0,
            first_powers,
            challenge,
        );
        let second_input = H2::merge_msm_terms(
            second_terms.into_iter().collect(),
            &prover_commitment.1,
            &public_statement.1,
            second_powers,
            challenge,
        );
        (first_input, second_input)
    }

    // TODO: maybe remove, see comment below
    pub fn check_first_msm_eval(&self, input: MsmInput<H1::Base, F>) -> anyhow::Result<()> {
        let result = H1::msm_eval(input);
        ensure!(result == H1::MsmOutput::zero());
        Ok(())
    }

    // TODO: Doesn't get used atm... so we're implicitly mixing different MSM code :-/
    pub fn check_second_msm_eval(&self, input: MsmInput<H2::Base, F>) -> anyhow::Result<()> {
        let result = H2::msm_eval(input);
        ensure!(result == H2::MsmOutput::zero());
        Ok(())
    }

    #[allow(non_snake_case)]
    pub fn msm_terms_for_verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &<Self as homomorphism::Trait>::CodomainNormalized,
        proof: &Proof<F, H>,
        cntxt: &Ct,
        number_of_beta_powers: Option<(usize, usize)>, // (len1, len2); None => compute from statement (clones)
        rng: &mut R,
    ) -> (MsmInput<H1::Base, F>, MsmInput<H2::Base, F>)
    where
        H: homomorphism::Trait<
            Domain = <Self as homomorphism::Trait>::Domain,
            CodomainNormalized = <Self as homomorphism::Trait>::CodomainNormalized,
        >,
    {
        let prover_first_message = proof
            .prover_commitment()
            .expect("Missing implementation - expected commitment, not challenge");
        let (len1, len2) =
            number_of_beta_powers.unwrap_or_else(|| tuple_statement_lengths(public_statement));
        let (c, powers_of_beta) = verifier_challenges_with_length::<_, F, _, _>(
            cntxt,
            self,
            public_statement,
            prover_first_message,
            &sigma_protocol::Trait::dst(self),
            len1 + len2,
            rng,
        );
        self.merge_msm_terms_for_verify(
            &proof.z,
            prover_first_message,
            public_statement,
            c,
            &powers_of_beta,
            len1,
        )
    }
}
