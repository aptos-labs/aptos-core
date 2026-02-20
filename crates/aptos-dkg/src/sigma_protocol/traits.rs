// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    fiat_shamir,
    sigma_protocol::{
        homomorphism::{
            self,
            fixed_base_msms::{self},
        },
        FirstProofItem, Proof,
    },
    Scalar,
};
use anyhow::ensure;
use aptos_crypto::{
    arkworks::{
        msm::{merge_scaled_msm_terms, MsmInput},
        random::sample_field_element,
    },
    utils,
};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::{AdditiveGroup, Field, Fp, FpConfig, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::Serialize;
use std::{fmt::Debug, hash::Hash};

pub trait Trait:
    homomorphism::Trait<Domain: Witness<Self::Scalar>, CodomainNormalized: Statement> + Sized
{
    type Scalar: PrimeField; // CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;

    fn dst(&self) -> Vec<u8>;

    fn prove<Ct: Serialize, R: RngCore + CryptoRng>(
        &self,
        witness: &Self::Domain,
        statement: Self::Codomain,
        cntxt: &Ct, // for SoK purposes
        rng: &mut R,
    ) -> (Proof<Self::Scalar, Self>, Self::CodomainNormalized) {
        prove_homomorphism(self, witness, statement, cntxt, true, rng, &self.dst())
    }

    /// Verify a sigma protocol proof.
    ///
    /// `verifier_batch_size`: number of components used for verifier batching.
    /// - `None`: infer from `public_statement` (may clone to count).
    /// - `Some(n)`: use `n` directly; avoids cloning when the caller already has the count
    ///   (e.g. when batching multiple proofs).
    fn verify<Ct: Serialize, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<Self::Scalar, Self>,
        cntxt: &Ct,
        verifier_batch_size: Option<usize>,
        rng: &mut R,
    ) -> anyhow::Result<()> {
        let prover_first_message = proof
            .prover_commitment()
            .expect("tuple proof must contain commitment for Fiat–Shamir"); // TODO: code alternative version
        let c = fiat_shamir_challenge_for_sigma_protocol::<_, Self::Scalar, _>(
            cntxt,
            self,
            public_statement,
            prover_first_message,
            &self.dst(),
        );
        self.verify_with_challenge(
            public_statement,
            prover_first_message,
            c,
            &proof.z,
            verifier_batch_size,
            rng,
        )
    }

    /// Verify the equations coming from the proof given an explicit Fiat–Shamir challenge
    /// (derived from the proof's first message).
    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_commitment: &Self::CodomainNormalized,
        challenge: Self::Scalar,
        response: &Self::Domain,
        verifier_batch_size: Option<usize>,
        rng: &mut R,
    ) -> anyhow::Result<()>;
}

impl<T: CurveGroupTrait> Trait for T {
    type Scalar = T::Scalar;

    fn dst(&self) -> Vec<u8> {
        self.dst()
    }

    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_commitment: &Self::CodomainNormalized,
        challenge: Self::Scalar,
        response: &Self::Domain,
        verifier_batch_size: Option<usize>,
        rng: &mut R,
    ) -> anyhow::Result<()> {
        let number_of_beta_powers =
            verifier_batch_size.unwrap_or_else(|| public_statement.clone().into_iter().count());
        let powers_of_beta = if number_of_beta_powers > 1 {
            let beta = sample_field_element(rng);
            utils::powers(beta, number_of_beta_powers)
        } else {
            vec![<<Self as CurveGroupTrait>::Group as PrimeGroup>::ScalarField::ONE]
        };
        let msm_terms_for_prover_response = self.msm_terms(response);
        let msm_terms = Self::merge_msm_terms(
            msm_terms_for_prover_response.into_iter().collect(),
            prover_commitment,
            public_statement,
            &powers_of_beta,
            challenge,
        );
        let msm_result = Self::msm_eval(msm_terms);
        ensure!(msm_result == <<Self as CurveGroupTrait>::Group as AdditiveGroup>::ZERO);
        Ok(())
    }
}

// TODO: rename this to CurveGroupTrait
// then make a more basic Trait
// then make CurveGroupTrait automatically implement that
// and then make a field hom implement the basic Trait
pub trait CurveGroupTrait:
    fixed_base_msms::Trait<
        Domain: Witness<<Self::Group as PrimeGroup>::ScalarField>,
        Base = <Self::Group as CurveGroup>::Affine,
        MsmOutput = Self::Group,
        Scalar = <Self::Group as PrimeGroup>::ScalarField,
    > + Sized
    + CanonicalSerialize
{
    type Group: CurveGroup;
    /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
    /// transcript operations within the protocol are uniquely namespaced
    fn dst(&self) -> Vec<u8>;

    #[allow(non_snake_case)]
    fn verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<<Self as fixed_base_msms::Trait>::Scalar, H>, // Would seem natural to set &Proof<E, Self>, but that ties the lifetime of H to that of Self, but we'd like it to be eg static
        cntxt: &Ct,
        verifier_batch_size: Option<usize>,
        rng: &mut R,
    ) -> anyhow::Result<()>
    where
        H: homomorphism::Trait<
            Domain = Self::Domain,
            CodomainNormalized = Self::CodomainNormalized,
        >, // need this because `H` is technically different from `Self` due to lifetime changes
    {
        let msm_terms = self.msm_terms_for_verify::<_, H, _>(
            public_statement,
            proof,
            cntxt,
            verifier_batch_size,
            rng,
        );

        let msm_result = Self::msm_eval(msm_terms);
        ensure!(msm_result == <Self::Group as AdditiveGroup>::ZERO); // or MsmOutput::zero()

        Ok(())
    }

    #[allow(non_snake_case)]
    fn compute_verifier_challenges<Ct, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_first_message: &Self::CodomainNormalized, // TODO: this input will have to be modified for `compact` proofs; we just need something serializable, could pass `FirstProofItem<F, H>` instead
        cntxt: &Ct,
        verifier_batch_size: Option<usize>,
        rng: &mut R,
    ) -> (
        <Self as fixed_base_msms::Trait>::Scalar,
        Vec<<Self as fixed_base_msms::Trait>::Scalar>,
    )
    where
        Ct: Serialize,
        // H: homomorphism::Trait<Domain = Self::Domain, Codomain = Self::Codomain>, // will probably need this if we use `FirstProofItem<F, H>` instead
    {
        let number_of_beta_powers =
            verifier_batch_size.unwrap_or_else(|| public_statement.clone().into_iter().count());
        verifier_challenges_with_length::<_, _, _, _>(
            cntxt,
            self,
            public_statement,
            prover_first_message,
            &self.dst(),
            number_of_beta_powers,
            rng,
        )
    }

    // Returns the MSM terms that `verify()` needs
    #[allow(non_snake_case)]
    fn msm_terms_for_verify<Ct: Serialize, H, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<<Self::Group as PrimeGroup>::ScalarField, H>,
        cntxt: &Ct,
        verifier_batch_size: Option<usize>,
        rng: &mut R,
    ) -> MsmInput<<Self::Group as CurveGroup>::Affine, <Self::Group as PrimeGroup>::ScalarField>
    where
        H: homomorphism::Trait<
            Domain = Self::Domain,
            CodomainNormalized = Self::CodomainNormalized,
        >, // Need this because the lifetime was changed
    {
        let prover_first_message = match &proof.first_proof_item {
            FirstProofItem::Commitment(A) => A,
            FirstProofItem::Challenge(_) => {
                panic!("Missing implementation - expected commitment, not challenge")
            },
        };

        let (c, powers_of_beta) = self.compute_verifier_challenges(
            public_statement,
            prover_first_message,
            cntxt,
            verifier_batch_size,
            rng,
        );

        let msm_terms_for_prover_response = self.msm_terms(&proof.z);

        Self::merge_msm_terms(
            msm_terms_for_prover_response.into_iter().collect(),
            prover_first_message,
            public_statement,
            &powers_of_beta,
            c,
        )
    }

    /// The MSM terms of the sigma protocol. Instead of computing the answer, returning the terms in this form.
    /// This is useful for combining with the MSM terms of other protocols, but note that beta powers are already being
    /// added here because it's convenient (and slightly faster) to do that when the c factor is being added
    #[allow(non_snake_case)]
    fn merge_msm_terms(
        msm_terms: Vec<
            MsmInput<<Self::Group as CurveGroup>::Affine, <Self::Group as PrimeGroup>::ScalarField>,
        >,
        prover_first_message: &Self::CodomainNormalized,
        statement: &Self::CodomainNormalized,
        powers_of_beta: &[<Self::Group as PrimeGroup>::ScalarField],
        c: <Self::Group as PrimeGroup>::ScalarField,
    ) -> MsmInput<<Self::Group as CurveGroup>::Affine, <Self::Group as PrimeGroup>::ScalarField>
    where
        <Self::Group as CurveGroup>::Affine: Copy + Eq + Hash,
    {
        let n = msm_terms.len();
        // Per index: (term_i * β^i) ∪ (A_i, −β^i) ∪ (P_i, −c·β^i), then in the final line merge all with scale 1.
        let term_inputs: Vec<
            MsmInput<<Self::Group as CurveGroup>::Affine, <Self::Group as PrimeGroup>::ScalarField>,
        > = msm_terms
            .into_iter()
            .zip(prover_first_message.clone().into_iter())
            .zip(statement.clone().into_iter())
            .zip(powers_of_beta.iter().copied())
            .map(|(((term, A), P), beta_power)| {
                let mut bases = term.bases().to_vec();
                bases.push(A);
                bases.push(P);
                let mut scalars: Vec<<Self::Group as PrimeGroup>::ScalarField> =
                    term.scalars().iter().map(|s| *s * beta_power).collect();
                scalars.push(-beta_power);
                scalars.push(-c * beta_power);
                MsmInput::new(bases, scalars).expect("sigma protocol MSM term")
            })
            .collect();
        debug_assert_eq!(
            term_inputs.len(),
            n,
            "merge_msm_terms: msm_terms iterator length mismatch"
        );
        debug_assert_eq!(
            powers_of_beta.len(),
            n,
            "merge_msm_terms: powers_of_beta iterator length mismatch"
        );
        let refs: Vec<
            &MsmInput<
                <Self::Group as CurveGroup>::Affine,
                <Self::Group as PrimeGroup>::ScalarField,
            >,
        > = term_inputs.iter().collect();
        let ones: Vec<<Self::Group as PrimeGroup>::ScalarField> = (0..refs.len())
            .map(|_| <Self::Group as PrimeGroup>::ScalarField::ONE)
            .collect();
        merge_scaled_msm_terms::<Self::Group>(&refs, &ones)
    }
}

// Standard method to get `trait Statement = Canonical Serialize + ...`, because type aliases are experimental in Rust
pub trait Statement: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq {}
impl<T> Statement for T where T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq {}

pub trait Witness<F: Field>: CanonicalSerialize + CanonicalDeserialize + Clone + Eq {
    /// Computes a scaled addition: `self + c * other`. Can take ownership because the
    /// randomness is discarded by the prover afterwards
    fn scaled_add(self, other: &Self, c: F) -> Self;

    /// Samples a random element in the domain. The prover has a witness `w` and calls `w.rand(rng)` to get
    /// the prover's first nonce (of the same "size" as `w`, hence why this cannot be a static method),
    /// which it then uses to compute the prover's first message in the sigma protocol.
    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self;
}

impl<const N: usize, P: FpConfig<N>> Witness<Fp<P, N>> for Fp<P, N> {
    fn scaled_add(self, other: &Self, c: Fp<P, N>) -> Self {
        self + c * other
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        sample_field_element(rng)
    }
}

impl<F: PrimeField> Witness<F> for Scalar<F> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        Scalar(self.0 + (c) * other.0)
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        Scalar(sample_field_element(rng))
    }
}

impl<F: PrimeField, W: Witness<F>> Witness<F> for Vec<W> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        self.into_iter()
            .zip(other.iter())
            .map(|(a, b)| a.scaled_add(b, c))
            .collect()
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        self.iter().map(|elem| elem.rand(rng)).collect()
    }
}

/// Computes the Fiat–Shamir challenge and verifier β-powers for a Σ-protocol,
/// with an explicit total length for the powers (e.g. `len1 + len2` for two-component protocols).
/// Callers can split the returned `powers_of_beta` slice as needed.
#[allow(non_snake_case)]
pub fn verifier_challenges_with_length<
    Ct: Serialize,
    F: PrimeField,
    H: homomorphism::Trait + CanonicalSerialize,
    R: RngCore + CryptoRng,
>(
    cntxt: &Ct,
    hom: &H,
    public_statement: &H::CodomainNormalized,
    prover_first_message: &H::CodomainNormalized,
    dst: &[u8],
    number_of_beta_powers: usize,
    rng: &mut R,
) -> (F, Vec<F>)
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
    let c = fiat_shamir_challenge_for_sigma_protocol::<_, F, _>(
        cntxt,
        hom,
        public_statement,
        prover_first_message,
        dst,
    );
    let powers_of_beta = if number_of_beta_powers > 1 {
        let beta = sample_field_element(rng);
        utils::powers(beta, number_of_beta_powers)
    } else {
        vec![F::ONE]
    };
    (c, powers_of_beta)
}

/// Computes the Fiat–Shamir challenge for a Σ-protocol instance.
///
/// This function derives a non-interactive challenge scalar by appending
/// protocol-specific data to a Merlin transcript. In the abstraction used here,
/// the protocol proves knowledge of a preimage under a homomorphism. Therefore,
/// all public data relevant to that homomorphism (e.g., its MSM bases) and
/// the image under consideration are included in the transcript.
///
/// # Arguments
/// - `cntxt`: Extra "context" material that needs to be hashed for the challenge.
/// - `hom`: The homomorphism structure carrying its public data (e.g., MSM bases).
/// - `statement`: The public statement, i.e. the image of a witness under the homomorphism.
/// - `prover_first_message`: the first message in the Σ-protocol (the prover's commitment)
/// - `dst`: A domain separation tag to ensure unique challenges per protocol.
///
/// # Returns
/// The derived Fiat–Shamir challenge scalar, after incorporating the domain
/// separator, public data, statement, and prover’s first message into the transcript.
pub fn fiat_shamir_challenge_for_sigma_protocol<
    Ct: Serialize,
    F: PrimeField,
    H: homomorphism::Trait + CanonicalSerialize,
>(
    cntxt: &Ct,
    hom: &H,
    statement: &H::CodomainNormalized,
    prover_first_message: &H::CodomainNormalized,
    dst: &[u8],
) -> F
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
{
    // Initialise the transcript
    let mut fs_t = merlin::Transcript::new(dst);

    // Append the "context" to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_ctxt(
        &mut fs_t, cntxt,
    );

    // Append the homomorphism data (e.g. MSM bases) to the transcript. (If the same hom is used for many proofs, maybe use a single transcript + a boolean to prevent it from repeating?)
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_msm_bases(
        &mut fs_t, hom,
    );

    // Append the public statement (the image of the witness) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_public_statement(
        &mut fs_t,
        &statement,
    );

    // Add the first prover message (the commitment) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::append_sigma_protocol_first_prover_message(
        &mut fs_t,
        prover_first_message,
    );

    // Generate the Fiat-Shamir challenge from the updated transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<F, H>>::challenge_for_sigma_protocol(
        &mut fs_t,
    )
}

// We're keeping this separate because it only needs the homomorphism property rather than being a bunch of "fixed-base MSMS",
// and moreover in this way it gets reused in the PairingTupleHomomorphism code which has a custom sigma protocol implementation
#[allow(non_snake_case)]
pub fn prove_homomorphism<Ct: Serialize, F: PrimeField, H: homomorphism::Trait, R>(
    homomorphism: &H,
    witness: &H::Domain,
    statement: H::Codomain,
    cntxt: &Ct,
    store_prover_commitment: bool, // true = store prover's commitment, false = store Fiat-Shamir challenge instead
    rng: &mut R,
    dst: &[u8],
) -> (Proof<F, H>, H::CodomainNormalized)
where
    H::Domain: Witness<F>,
    H::CodomainNormalized: Statement,
    R: RngCore + CryptoRng,
{
    // Step 1: Sample randomness. Here the `witness` is only used to make sure that `r` has the right dimensions
    let r = witness.rand(rng);

    // Step 2: Compute commitment A = Ψ(r)
    let A_proj = homomorphism.apply(&r);
    let A = homomorphism.normalize(A_proj);
    let normalized_statement = homomorphism.normalize(statement); // TODO: combine these two normalisations

    // Step 3: Obtain Fiat-Shamir challenge
    let c = fiat_shamir_challenge_for_sigma_protocol::<_, F, H>(
        cntxt,
        homomorphism,
        &normalized_statement,
        &A,
        dst,
    );

    // Step 4: Compute prover response
    let z = r.scaled_add(&witness, c);

    // Step 5: Pick first **recorded** item
    let first_proof_item = if store_prover_commitment {
        FirstProofItem::Commitment(A)
    } else {
        FirstProofItem::Challenge(c)
    };

    (
        Proof {
            first_proof_item,
            z,
        },
        normalized_statement,
    )
}
