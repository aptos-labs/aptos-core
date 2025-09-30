// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    algebra::msm::{FixedBaseMSM, Map},
    fiat_shamir, utils,
};
use anyhow::ensure;
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::{CryptoRng, RngCore};

pub trait SigmaProtocol<E: Pairing> {
    type Statement: Codomain;
    type Witness: Domain<E>;
    type Hom: FixedBaseMSM<
        Domain = Self::Witness,
        Codomain = Self::Statement,
        Scalar = E::ScalarField,
        Base = E::G1Affine,
    >;

    const DST: &'static [u8];
    const DST_VERIFIER: &'static [u8];

    fn homomorphism(&self) -> Self::Hom;

    fn prove<R: RngCore + CryptoRng>(
        &self,
        witness: &Self::Witness,
        transcript: &mut merlin::Transcript,
        rng: &mut R,
    ) -> Proof<E, Self::Hom> {
        prove_homomorphism(
            self.homomorphism(),
            witness,
            transcript,
            true,
            rng,
            Self::DST,
        )
    }

    #[allow(non_snake_case)]
    fn verify(
        &self,
        statement: &Self::Statement,
        proof: &Proof<E, Self::Hom>,
        transcript: &mut merlin::Transcript,
    ) -> anyhow::Result<()> {
        verify_msm_hom(
            self.homomorphism(),
            statement,
            match &proof.first_stored_message {
                FirstStoredMessage::Commitment(A) => A,
                FirstStoredMessage::Challenge(_) => {
                    anyhow::bail!("Missing implementation - expected commitment, not challenge")
                },
            },
            &proof.z,
            transcript,
            Self::DST,
            Self::DST_VERIFIER,
        )
    }
}

pub trait Domain<E: Pairing>:
    CanonicalSerialize + Clone + std::fmt::Debug + PartialEq + Eq
{
    /// The scalar type associated with the domain.
    type Scalar: CanonicalSerialize + CanonicalDeserialize + Copy + std::fmt::Debug + PartialEq + Eq;

    /// Computes a scaled addition: `self + c * other`.
    fn scaled_add(&self, other: &Self, c: E::ScalarField) -> Self;
    // TODO: Maybe implement this directly / obtain it automatically by using Mul, Add, etc?? Seems impractical with arkworks

    /// Samples a random element in the domain.
    fn sample_randomness<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self;
    // TODO: Do this via UniformRand instead?
}

pub trait Codomain: CanonicalSerialize + Clone + std::fmt::Debug + PartialEq + Eq {}
impl<T> Codomain for T where T: CanonicalSerialize + Clone + std::fmt::Debug + PartialEq + Eq {}

/// The “first message” **stored** in a Sigma proof, which is one of:
/// - Commitment from the prover
/// - Challenge from the verifier
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FirstStoredMessage<E: Pairing, H: Map>
where
    H::Domain: Domain<E>,
    H::Codomain: Codomain,
{
    Commitment(H::Codomain),
    Challenge(E::ScalarField),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Proof<E: Pairing, H: Map>
where
    H::Domain: Domain<E>,
    H::Codomain: Codomain,
{
    /// The “first message” stored in the proof: either the prover's commitment (H::Codomain)
    /// or the verifier's challenge (H::Domain::Scalar)
    pub first_stored_message: FirstStoredMessage<E, H>,
    /// Prover's second message (response)
    pub z: H::Domain,
}

/// Computes the Fiat-Shamir challenge for a Sigma protocol.
///
/// # Parameters
/// - `fs_transcript`: the mutable Merlin transcript to update
/// - `prover_first_message`: the first message in the Sigma protocol (the prover's commitment)
///
/// # Returns
/// The Fiat-Shamir challenge scalar, after appending the DST and the first message to the Fiat-Shamir transcript.
pub fn fiat_shamir_challenge_for_sigma_protocol<E: Pairing, H: Map>(
    fs_transcript: &mut merlin::Transcript,
    prover_first_message: &H::Codomain,
    dst: &'static [u8],
) -> E::ScalarField
where
    H::Domain: Domain<E>,
    H::Codomain: Codomain,
{
    // Append the sigma protocol separator to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_sep(
        fs_transcript,
        dst,
    );

    // Add the first prover message (the commitment) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_first_prover_message(
        fs_transcript,
        prover_first_message,
    );

    // Generate the Fiat-Shamir challenge from the updated transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::challenge_for_sigma_protocol(
        fs_transcript,
    )
}

#[allow(non_snake_case)]
pub fn prove_homomorphism<E: Pairing, H: Map, R>(
    homomorphism: H,
    witness: &H::Domain,
    fiat_shamir_transcript: &mut merlin::Transcript,
    store_prover_commitment: bool, // true = store prover's commitment, false = store Fiat-Shamir challenge
    rng: &mut R,
    dst: &'static [u8],
) -> Proof<E, H>
where
    H::Domain: Domain<E>,
    H::Codomain: Codomain,
    R: RngCore + CryptoRng,
{
    // Step 1: Sample randomness
    let r = witness.sample_randomness(rng);

    // Step 2: Compute commitment A = Ψ(r)
    let A = homomorphism.apply(&r);

    // Step 3: Obtain Fiat-Shamir challenge
    let c = fiat_shamir_challenge_for_sigma_protocol::<E, H>(fiat_shamir_transcript, &A, dst);

    // Step 4: Compute prover response
    let z = r.scaled_add(&witness, c);

    // Step 5: Pick first **stored** message
    let first_stored_message = if store_prover_commitment {
        FirstStoredMessage::Commitment(A)
    } else {
        FirstStoredMessage::Challenge(c)
    };

    Proof {
        first_stored_message,
        z,
    }
}

pub fn fiat_shamir_challenge_for_msm_verifier<E: Pairing, H: Map>(
    fs_transcript: &mut merlin::Transcript,
    public_statement: &H::Codomain,
    prover_last_message: &H::Domain,
    dst: &'static [u8],
) -> E::ScalarField
where
    H::Domain: Domain<E>,
    H::Codomain: Codomain,
{
    // Append the sigma protocol separator to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_sep(
        fs_transcript,
        dst,
    );

    // Add the last prover message (the prover's response) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_last_message(
        fs_transcript,
        prover_last_message,
    );

    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_public_statement(
        fs_transcript,
        public_statement,
    );

    // Generate the Fiat-Shamir challenge from the updated transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::challenge_for_sigma_protocol(
        fs_transcript,
    )
}

#[allow(non_snake_case)]
pub fn verify_msm_hom<E: Pairing, H>(
    homomorphism: H,
    public_statement: &H::Codomain,
    prover_first_message: &H::Codomain,
    prover_last_message: &H::Domain,
    fs_transcript: &mut merlin::Transcript,
    dst: &'static [u8],
    dst_verifier: &'static [u8],
) -> anyhow::Result<()>
where
    H: FixedBaseMSM<Scalar = E::ScalarField, Base = E::G1Affine>,
    H::Domain: Domain<E>,
    H::Codomain: Codomain,
{
    // Step 1: Reproduce the prover's Fiat-Shamir challenge
    let c =
        fiat_shamir_challenge_for_sigma_protocol::<E, H>(fs_transcript, &prover_first_message, dst);

    // Step 2: Compute verifier-specific challenge (used for weighted MSM)
    let beta = fiat_shamir_challenge_for_msm_verifier::<E, H>(
        fs_transcript,
        public_statement,
        prover_last_message,
        dst_verifier,
    );

    // Step 3: Flatten and convert part of proof for easier MSM combination
    let prover_first_message_flat = homomorphism.flatten_codomain(prover_first_message);
    let public_statement_flat = homomorphism.flatten_codomain(public_statement);

    // Step 4: Compute MSM components for the rest of the proof
    let mut msm_rows = homomorphism.msm_rows(prover_last_message);

    // Step 5: Build weighted MSM for each row
    let powers_of_beta = utils::powers(beta, msm_rows.len());
    for (i, (bases, scalars)) in msm_rows.iter_mut().enumerate() {
        // Add the prover's commitment and the public statement to the bases
        bases.push(prover_first_message_flat[i]);
        bases.push(public_statement_flat[i]);

        // Scale existing scalars by the appropriate power of the verifier challenge
        let beta_power = powers_of_beta[i];
        for scalar in scalars.iter_mut() {
            *scalar *= beta_power;
        }

        // Add scalars for the prover's commitment and challenge
        scalars.push(-E::ScalarField::ONE * beta_power); // for the prover's commitment
        scalars.push(-c * beta_power); // for challenge
    }

    // Step 6: Flatten all MSM rows into single vectors for final MSM
    let mut all_bases = Vec::new();
    let mut all_scalars = Vec::new();
    for (bases, scalars) in msm_rows {
        all_bases.extend(bases);
        all_scalars.extend(scalars);
    }

    // Step 7: Perform the final MSM check
    let msm_result =
        E::G1::msm(&all_bases, &all_scalars).expect("Could not compute MSM for verifier");
    ensure!(msm_result == E::G1::ZERO);

    Ok(())
}
