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
use anyhow::{bail, ensure, Result};
use aptos_crypto::arkworks::{
    msm::{merge_msm_inputs, MsmInput},
    random::sample_field_element,
};
use ark_ec::{CurveGroup, PrimeGroup};
use ark_ff::{AdditiveGroup, Field, Fp, FpConfig, PrimeField};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use rand_core::{CryptoRng, RngCore};
use serde::Serialize;
use std::fmt::Debug;

/// We can construct a sigma protocol proof with any homomorphism. In the proving
/// process, we need the two methods of the `Witness` trait.
pub trait Trait:
    homomorphism::Trait<Domain: Witness<Self::Scalar>, CodomainNormalized: Statement> + Sized
// Need `Sized` here because of `Proof<Self::Scalar, Self>`
{
    type Scalar: PrimeField; // Because Fiat-Shamir challenges currently use `PrimeField`

    /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
    /// transcript operations within the protocol are uniquely namespaced
    fn dst(&self) -> Vec<u8>;

    /// Construct a sigma protocol proof. Returns the proof and the normalized statement.
    ///
    /// We're returning the normalised statement, because here in prove() might be the first
    /// time that the statement needs to be normalised; we can then do it together with normalising
    /// A for more efficiency
    #[allow(non_snake_case)]
    fn prove<Ct: Serialize, R: RngCore + CryptoRng>(
        &self,
        witness: &Self::Domain,
        statement: Self::Codomain, // TODO: should allow to either submit H::Codomain or H::CodomainNormalized
        cntxt: &Ct,                // needed for SoK purposes
        rng: &mut R,
    ) -> (Proof<Self::Scalar, Self>, Self::CodomainNormalized) {
        let store_prover_commitment = true; // TODO: should change this into a parameter when code is ready

        // Step 1: Sample randomness. Here the `witness` is only used to make sure that `r` has the right dimensions
        let r = witness.rand(rng);

        // Step 2: Compute commitment A = Ψ(r)
        let A_proj = self.apply(&r);
        let A = self.normalize(A_proj);
        let normalized_statement = self.normalize(statement); // TODO: combine these two normalisations

        // Step 3: Obtain Fiat-Shamir challenge
        let c = self.fiat_shamir_challenge_for_sigma_protocol(cntxt, &normalized_statement, &A);

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

    /// Computes the Fiat–Shamir challenge for a Σ-protocol instance.
    ///
    /// This function derives a non-interactive challenge scalar by appending
    /// protocol-specific data to a Merlin transcript. In the abstraction used here,
    /// the protocol proves knowledge of a preimage under a homomorphism. Therefore,
    /// all public data relevant to that homomorphism (e.g., its MSM bases) and
    /// the homomorphism image under consideration are included in the transcript.
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
    fn fiat_shamir_challenge_for_sigma_protocol<Ct: Serialize>(
        &self,
        cntxt: &Ct,
        statement: &Self::CodomainNormalized,
        prover_first_message: &Self::CodomainNormalized,
    ) -> Self::Scalar {
        // Initialise the transcript
        let mut fs_t = merlin::Transcript::new(&self.dst());

        // Append the "context" to the transcript
        <merlin::Transcript as fiat_shamir::SigmaProtocol<Self::Scalar, Self>>::append_sigma_protocol_cntxt(
            &mut fs_t, cntxt,
        );

        // Append the homomorphism data (e.g. MSM bases) to the transcript. // TODO: If the same hom is used for many proofs, maybe use a single transcript + a boolean to prevent it from repeating?
        <merlin::Transcript as fiat_shamir::SigmaProtocol<Self::Scalar, Self>>::append_sigma_protocol_msm_bases(
            &mut fs_t, self,
        );

        // Append the public statement (the homomorphism image of the witness) to the transcript
        <merlin::Transcript as fiat_shamir::SigmaProtocol<Self::Scalar, Self>>::append_sigma_protocol_public_statement(
            &mut fs_t,
            &statement,
        );

        // Add the first prover message (the commitment) to the transcript
        <merlin::Transcript as fiat_shamir::SigmaProtocol<Self::Scalar, Self>>::append_sigma_protocol_first_prover_message(
            &mut fs_t,
            prover_first_message,
        );

        // Generate the Fiat-Shamir challenge from the updated transcript
        <merlin::Transcript as fiat_shamir::SigmaProtocol<Self::Scalar, Self>>::challenge_for_sigma_protocol(
            &mut fs_t,
        )
    }

    /// Verify a sigma protocol proof. Returns `Ok(())` if the proof is valid, `Err(anyhow::Error)` otherwise.
    fn verify<Ct: Serialize, R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<Self::Scalar, Self>,
        cntxt: &Ct,
        rng: &mut R,
    ) -> Result<()> {
        let prover_first_message = match proof.prover_commitment() {
            Some(m) => m,
            None => bail!("proof must contain commitment for Fiat–Shamir"),
        };
        let c = self.fiat_shamir_challenge_for_sigma_protocol(
            cntxt,
            public_statement,
            prover_first_message,
        );
        self.verify_with_challenge(public_statement, prover_first_message, c, &proof.z, rng)
    }

    /// Verify the equations coming from the proof given an explicit Fiat–Shamir challenge
    /// (derived from the proof's first message).
    /// The reason for this separate method is tuple homomorphisms - we need to verify each component
    /// of the tuple homomorphism separately, but using the same challenge.
    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_commitment: &Self::CodomainNormalized,
        challenge: Self::Scalar,
        response: &Self::Domain,
        rng: &mut R,
    ) -> Result<()>;
}

// Specialised version where the homomorphism consists of fixed-base MSMs over one elliptic curve.
// Also used as a building block in tuples etc
#[allow(non_snake_case)]
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

    /// Checks that the given MSM input evaluates to the group identity.
    fn check_msm_eval_zero(
        &self,
        input: MsmInput<
            <Self::Group as CurveGroup>::Affine,
            <Self::Group as PrimeGroup>::ScalarField,
        >,
    ) -> Result<()> {
        let result = Self::msm_eval(input);
        ensure!(result == <Self::Group as AdditiveGroup>::ZERO);
        Ok(())
    }

    // Returns the MSM terms that `verify()` needs
    fn msm_terms_for_verify_with_challenge(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_first_message: &Self::CodomainNormalized,
        prover_response: &Self::Domain,
        challenge: <Self::Group as PrimeGroup>::ScalarField,
    ) -> Result<
        Vec<
            MsmInput<<Self::Group as CurveGroup>::Affine, <Self::Group as PrimeGroup>::ScalarField>,
        >,
    > {
        let msm_terms_for_prover_response = self.msm_terms(&prover_response);

        let minus_one = -<Self::Group as PrimeGroup>::ScalarField::ONE;
        let minus_challenge = -challenge;

        let msm_terms = msm_terms_for_prover_response
            .into_iter()
            .zip(prover_first_message.clone().into_iter()) // TODO: not sure the cloning is ideal here
            .zip(public_statement.clone().into_iter())
            .map(|((term, A), P)| {
                let mut bases = term.bases().to_vec();
                bases.push(A);
                bases.push(P);
                let mut scalars = term.scalars().to_vec();
                scalars.push(minus_one);
                scalars.push(minus_challenge);
                MsmInput::new(bases, scalars)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(msm_terms)
    }

    /// Returns the MSM terms that `verify()` needs, computing the Fiat–Shamir challenge
    /// from the transcript (context, statement, prover commitment) and then calling
    /// `msm_terms_for_verify_with_challenge()`.
    ///
    /// Accepts a proof produced with any homomorphism type `H2` that has the same
    /// `Domain` and `CodomainNormalized` as `Self`. This allows verifying a proof
    /// stored as `Proof<..., Homomorphism<'static, E>>` using a locally built
    /// `Homomorphism<'a, E>` (with a short lifetime `'a`) without forcing `'static`.
    ///
    /// We're not using this at the moment, but it could be useful when batching in the MSMs
    /// of a proper `CurveGroup`-style homomorphism (i.e., one that is not part of a tuple).
    fn msm_terms_for_verify<Ct: Serialize, H2>(
        &self,
        public_statement: &Self::CodomainNormalized,
        proof: &Proof<<Self::Group as PrimeGroup>::ScalarField, H2>,
        cntxt: &Ct,
    ) -> Result<
        Vec<
            MsmInput<<Self::Group as CurveGroup>::Affine, <Self::Group as PrimeGroup>::ScalarField>,
        >,
    >
    where
        H2: homomorphism::Trait<
            Domain = Self::Domain,
            CodomainNormalized = Self::CodomainNormalized,
        >,
    {
        let prover_first_message = match proof.prover_commitment() {
            Some(m) => m,
            None => bail!("proof must contain commitment for Fiat–Shamir"),
        };
        let c = self.fiat_shamir_challenge_for_sigma_protocol(
            cntxt,
            public_statement,
            prover_first_message,
        );
        self.msm_terms_for_verify_with_challenge(
            public_statement,
            prover_first_message,
            &proof.z,
            c,
        )
    }
}

// This is the default implementation for the `leaf` case (single MSM group / codomain)
impl<T: CurveGroupTrait> Trait for T {
    type Scalar = T::Scalar;

    fn dst(&self) -> Vec<u8> {
        CurveGroupTrait::dst(self) // `self.dst()` works but seems a bit too concise/circular
    }

    fn verify_with_challenge<R: RngCore + CryptoRng>(
        &self,
        public_statement: &Self::CodomainNormalized,
        prover_commitment: &Self::CodomainNormalized,
        challenge: Self::Scalar,
        response: &Self::Domain,
        rng: &mut R,
    ) -> Result<()> {
        let msm_terms = self.msm_terms_for_verify_with_challenge(
            public_statement,
            prover_commitment,
            response,
            challenge,
        )?;
        let merged = merge_msm_inputs(&msm_terms, rng)?;
        self.check_msm_eval_zero(merged)?;
        Ok(())
    }
}

// Standard method to get `trait Statement = CanonicalSerialize + ...`, because type aliases are experimental in Rust
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
