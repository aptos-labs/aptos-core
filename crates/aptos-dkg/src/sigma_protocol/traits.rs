// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    fiat_shamir,
    sigma_protocol::homomorphism::{FixedBaseMsms},
    utils, Scalar,
};
use anyhow::ensure;
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_ff::{AdditiveGroup, Field};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize, Valid};
use ark_std::{
    rand::{CryptoRng, RngCore},
    UniformRand,
};

// pub trait Trait<E: Pairing> {
//     type Statement: Statement;
//     type Witness: Witness<E>;
//     type Hom: FixedBaseMsms<
//         Domain = Self::Witness,
//         Codomain = Self::Statement,
//         Scalar = E::ScalarField,
//         Base = E::G1Affine,
//         MsmOutput = E::G1,
//     >;

//     const DST: &[u8];
//     const DST_VERIFIER: &[u8];

//     fn homomorphism(&self) -> Self::Hom;

//     fn prove<R: RngCore + CryptoRng>(
//         &self,
//         witness: &Self::Witness,
//         transcript: &mut merlin::Transcript,
//         rng: &mut R,
//     ) -> Proof<E, Self::Hom> {
//         prove_homomorphism(
//             self.homomorphism(),
//             witness,
//             transcript,
//             true,
//             rng,
//             Self::DST,
//         )
//     }

//     #[allow(non_snake_case)]
//     fn verify(
//         &self,
//         statement: &Self::Statement,
//         proof: &Proof<E, Self::Hom>,
//         transcript: &mut merlin::Transcript,
//     ) -> anyhow::Result<()>
//     where
//         Self::Hom: Homomorphism<Codomain = <Self::Hom as FixedBaseMsms>::CodomainShape<<Self::Hom as FixedBaseMsms>::MsmOutput>>,
//     {
//         verify_msm_hom(
//             self.homomorphism(),
//             statement,
//             match &proof.first_proof_item {
//                 FirstProofItem::Commitment(A) => A,
//                 FirstProofItem::Challenge(_) => {
//                     anyhow::bail!("Missing implementation - expected commitment, not challenge")
//                 },
//             },
//             &proof.z,
//             transcript,
//             Self::DST,
//             Self::DST_VERIFIER,
//         )
//     }
// }

pub trait Trait<E: Pairing> {
    // TODO: maybe make this 'pub trait Trait<E: Pairing>: FixedBaseMsms' for simplicity
    type Statement: Statement;
    type Witness: Witness<E>;
    type Hom: FixedBaseMsms<
        Domain = Self::Witness,
        Codomain = Self::Statement,
        Scalar = E::ScalarField, // Hmm nee Scalar<E> denk ik
        Base = E::G1Affine,
        MsmOutput = E::G1,
        CodomainShape<E::G1> = Self::Statement, //
    >;

    const DST: &[u8];
    const DST_VERIFIER: &[u8];

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
        public_statement: &Self::Statement,
        proof: &Proof<E, Self::Hom>,
        transcript: &mut merlin::Transcript,
    ) -> anyhow::Result<()>
//where 
        // Self::Hom: Homomorphism<Codomain = <Self::Hom as FixedBaseMsms>::CodomainShape<<Self::Hom as FixedBaseMsms>::MsmOutput>>,
    {
        // let prover_first_message = match &proof.first_proof_item {
        //             FirstProofItem::Commitment(A) => A,
        //             FirstProofItem::Challenge(_) => {
        //                 anyhow::bail!("Missing implementation - expected commitment, not challenge")
        //             },
        //         };

        // let prover_last_message = &proof.z;

        // // Step 1: Reproduce the prover's Fiat-Shamir challenge
        // let c =
        //     fiat_shamir_challenge_for_sigma_protocol::<E, Self::Hom>(fs_transcript, &prover_first_message, Self::DST);

        // // Step 2: Compute verifier-specific challenge (used for weighted MSM)
        // let beta = fiat_shamir_challenge_for_msm_verifier::<E, Self::Hom>(
        //     fs_transcript,
        //     public_statement,
        //     prover_last_message,
        //     Self::DST_VERIFIER,
        // );

        // let msm_terms = homomorphism.msm_terms(prover_last_message);
        // let powers_of_beta = utils::powers(beta, msm_terms.clone().into_iter().count()); // TODO get rid of clone. is .count() an efficient way to get the length?

        // let terms_iter = msm_terms.clone().into_iter(); // TODO: get rid of these clones
        // let prover_iter = prover_first_message.clone().into_iter();
        // let statement_iter = public_statement.clone().into_iter();

        // let mut final_basis = Vec::new();
        // let mut final_scalars = Vec::new();

        // for (((term, A), P), beta_power) in terms_iter.zip(prover_iter).zip(statement_iter).zip(powers_of_beta) {
        //     // Destructure term and create a new MsmInput
        //     //let homomorphism::MsmInput { bases: term_bases, scalars: term_scalars } = term;
        //     let mut bases = term.bases().to_vec();
        //     let mut scalars = term.scalars().to_vec();

        //     for scalar in scalars.iter_mut() {
        //         *scalar *= beta_power;
        //     }

        //     // Append bases/scalars from prover and statement
        //     // Assuming MsmResult can be cloned to Base/Scalar type
        //     // You may need a conversion function if MsmResult is not exactly Base/Scalar
        //     // Here we just append placeholders (e.g., 1)
        //     bases.push(A.clone().into_affine()); // TODO: do a batch into affine
        //     bases.push(P.clone().into_affine()); // TODO: do a batch into affine

        //     scalars.push(- H::Scalar::from(1u8) * beta_power);
        //     scalars.push(- c * beta_power);

        //     final_basis.extend(bases);
        //     final_scalars.extend(scalars);
        // }

        // // Step 7: Perform the final MSM check
        // let msm_result =
        //     E::G1::msm(&final_basis, &final_scalars).expect("Could not compute MSM for verifier");
        // ensure!(msm_result == E::G1::ZERO);

        // Ok(())

        // }

        verify_msm_hom::<E, Self::Hom>(
            self.homomorphism(),
            public_statement,
            match &proof.first_proof_item {
                FirstProofItem::Commitment(A) => A,
                FirstProofItem::Challenge(_) => {
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

pub trait Witness<E: Pairing>:
    CanonicalSerialize + CanonicalDeserialize + Clone + std::fmt::Debug + PartialEq + Eq
{
    /// The scalar type associated with the domain.
    type Scalar: CanonicalSerialize + CanonicalDeserialize + Copy + std::fmt::Debug + PartialEq + Eq;

    /// Computes a scaled addition: `self + c * other`. Can take ownership because the randomness is discarded by the prover afterwards
    fn scaled_add(self, other: &Self, c: E::ScalarField) -> Self;
    // TODO: Maybe implement this directly / obtain it automatically by using Mul, Add, etc?? Seems impractical with arkworks

    /// Samples a random element in the domain.
    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self;
    // TODO: Do this via UniformRand instead?
}

impl<E: Pairing> Witness<E> for Scalar<E> {
    type Scalar = Scalar<E>;

    fn scaled_add(self, other: &Self, c: E::ScalarField) -> Self {
        Scalar(self.0 + (c) * other.0)
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        Scalar(E::ScalarField::rand(rng))
    }
}

impl<E: Pairing, W: Witness<E>> Witness<E> for Vec<W> {
    type Scalar = W::Scalar;

    fn scaled_add(self, other: &Self, c: E::ScalarField) -> Self {
        self.into_iter()
            .zip(other.iter())
            .map(|(a, b)| a.scaled_add(b, c))
            .collect()
    }

    fn rand<R: RngCore + CryptoRng>(&self, rng: &mut R) -> Self {
        self.iter().map(|elem| elem.rand(rng)).collect()
    }
}

pub trait Statement: CanonicalSerialize + CanonicalDeserialize + Clone + PartialEq + Eq {}
impl<T> Statement for T where T: CanonicalSerialize + CanonicalDeserialize + Clone + PartialEq + Eq {}

/// The “first item” recorded in a Σ-proof, which is one of:
/// - The first message of the protocol, which is the commitment from the prover
/// - The second message of the protocol, which is the challenge from the verifier
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FirstProofItem<E: Pairing, H: homomorphism::Trait>
where
    H::Domain: Witness<E>,
    H::Codomain: Statement,
{
    Commitment(H::Codomain),
    Challenge(E::ScalarField),
}

use ark_serialize::{Compress, SerializationError};
use std::io::Write;

impl<E: Pairing, H: homomorphism::Trait> Valid for FirstProofItem<E, H>
where
    H::Domain: Witness<E>,
    H::Codomain: Statement + Valid,
    E::ScalarField: Valid,
{
    fn check(&self) -> Result<(), SerializationError> {
        match self {
            FirstProofItem::Commitment(c) => c.check(),
            FirstProofItem::Challenge(f) => f.check(),
        }
    }
}

impl<E: Pairing, H: homomorphism::Trait> CanonicalSerialize for FirstProofItem<E, H>
where
    H::Domain: Witness<E>,
    H::Codomain: Statement + CanonicalSerialize,
    E::ScalarField: CanonicalSerialize,
{
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        match self {
            FirstProofItem::Commitment(c) => {
                0u8.serialize_with_mode(writer.by_ref(), compress)?;
                c.serialize_with_mode(writer, compress)
            },
            FirstProofItem::Challenge(f) => {
                1u8.serialize_with_mode(writer.by_ref(), compress)?;
                f.serialize_with_mode(writer, compress)
            },
        }
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        1 + match self {
            FirstProofItem::Commitment(c) => c.serialized_size(compress),
            FirstProofItem::Challenge(f) => f.serialized_size(compress),
        }
    }
}

use ark_serialize::Validate;
pub use ark_std::io::Read;

impl<E: Pairing, H: homomorphism::Trait> CanonicalDeserialize for FirstProofItem<E, H>
where
    H::Domain: Witness<E>,
    H::Codomain: Statement + CanonicalDeserialize + Valid,
    E::ScalarField: CanonicalDeserialize + Valid,
{
    fn deserialize_with_mode<R: Read>(
        mut reader: R,
        compress: Compress,
        validate: Validate,
    ) -> Result<Self, SerializationError> {
        // Read the discriminant tag
        let tag = u8::deserialize_with_mode(&mut reader, compress, validate)?;

        let item = match tag {
            0 => {
                let c = H::Codomain::deserialize_with_mode(reader, compress, validate)?;
                FirstProofItem::Commitment(c)
            },
            1 => {
                let f = E::ScalarField::deserialize_with_mode(reader, compress, validate)?;
                FirstProofItem::Challenge(f)
            },
            _ => return Err(SerializationError::InvalidData),
        };

        // Run validity check if requested
        if validate == Validate::Yes {
            item.check()?;
        }

        Ok(item)
    }
}

#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, PartialEq, Eq)]
pub struct Proof<E: Pairing, H: homomorphism::Trait>
where
    H::Domain: Witness<E>,
    H::Codomain: Statement,
{
    /// The “first item” recorded in the proof: either the prover's commitment (H::Codomain)
    /// or the verifier's challenge (H::Domain::Scalar)
    pub first_proof_item: FirstProofItem<E, H>,
    /// Prover's second message (response)
    pub z: H::Domain,
}

/// Computes the Fiat-Shamir challenge for a Σ-protocol.
///
/// # Parameters
/// - `fs_transcript`: the mutable Merlin transcript to update
/// - `prover_first_message`: the first message in the Σ-protocol (the prover's commitment)
///
/// # Returns
/// The Fiat-Shamir challenge scalar, after appending the DST and the first message to the Fiat-Shamir transcript.
pub fn fiat_shamir_challenge_for_sigma_protocol<E: Pairing, H: homomorphism::Trait>(
    fs_transcript: &mut merlin::Transcript,
    prover_first_message: &H::Codomain,
    dst: &[u8],
) -> E::ScalarField
where
    H::Domain: Witness<E>,
    H::Codomain: Statement,
{
    // Append the Σ-protocol separator to the transcript
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
pub fn prove_homomorphism<E: Pairing, H: homomorphism::Trait, R>(
    homomorphism: H,
    witness: &H::Domain,
    fiat_shamir_transcript: &mut merlin::Transcript,
    store_prover_commitment: bool, // true = store prover's commitment, false = store Fiat-Shamir challenge
    rng: &mut R,
    dst: &[u8],
) -> Proof<E, H>
where
    H::Domain: Witness<E>,
    H::Codomain: Statement,
    R: RngCore + CryptoRng,
{
    // Step 1: Sample randomness
    let r = witness.rand(rng);

    // Step 2: Compute commitment A = Ψ(r)
    let A = homomorphism.apply(&r);

    // Step 3: Obtain Fiat-Shamir challenge
    let c = fiat_shamir_challenge_for_sigma_protocol::<E, H>(fiat_shamir_transcript, &A, dst);

    // Step 4: Compute prover response
    let z = r.scaled_add(&witness, c);

    // Step 5: Pick first **recorded** item
    let first_proof_item = if store_prover_commitment {
        FirstProofItem::Commitment(A)
    } else {
        FirstProofItem::Challenge(c)
    };

    Proof {
        first_proof_item,
        z,
    }
}

pub fn fiat_shamir_challenge_for_msm_verifier<E: Pairing, H: homomorphism::Trait>(
    fs_transcript: &mut merlin::Transcript,
    public_statement: &H::Codomain,
    prover_last_message: &H::Domain,
    dst: &[u8],
) -> E::ScalarField
where
    H::Domain: Witness<E>,
    H::Codomain: Statement,
{
    // Append the Σ-protocol separator to the transcript
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

// // NEE MOVE DEZE CODE NAAR SIGMA PROTOCOL!!!
// fn prepare_sigma_msm(
//     terms: Self::CodomainShape<Self::MsmInput>,
//     prover_first_message: Self::CodomainShape<Self::MsmResult>,
//     statement: Self::CodomainShape<Self::MsmResult>,
//     challenge: Self::Scalar,
// ) -> Vec<Self::MsmInput>
// where
//     Self::Base: Clone,
//     Self::Scalar: Clone + From<u8>,
//     Self::MsmInput: Clone,
//     Self::MsmResult: Clone,
// {
//     let mut ans: Vec<Self::MsmInput> = Vec::new();

//     // Convert iterators
//     let terms_iter = terms.into_iter();
//     let prover_iter = prover_first_message.into_iter();
//     let statement_iter = statement.into_iter();

//     for ((term, prover), st) in terms_iter.zip(prover_iter).zip(statement_iter) {
//         // Destructure term and create a new MsmInput
//         let homomorphism::MsmInput { bases: term_bases, scalars: term_scalars } = term;
//         let mut new_bases = term_bases.clone();
//         let mut new_scalars = term_scalars.clone();

//         // Append bases/scalars from prover and statement
//         // Assuming MsmResult can be cloned to Base/Scalar type
//         // You may need a conversion function if MsmResult is not exactly Base/Scalar
//         // Here we just append placeholders (e.g., 1)
//         new_bases.extend(prover.clone().into_iter());
//         new_bases.extend(st.clone().into_iter());

//         new_scalars.push(Self::Scalar::from(1u8));
//         new_scalars.push(Self::Scalar::from(1u8));

//         ans.push(MsmInput {
//             bases: new_bases,
//             scalars: new_scalars,
//         });
//     }

//     ans
// }

use crate::sigma_protocol::{homomorphism, homomorphism::IsMsmInput};
use ark_ec::CurveGroup;

#[allow(non_snake_case)]
pub fn verify_msm_hom<E: Pairing, H>(
    homomorphism: H,
    public_statement: &H::Codomain,
    prover_first_message: &H::Codomain,
    prover_last_message: &H::Domain,
    fs_transcript: &mut merlin::Transcript,
    dst: &[u8],
    dst_verifier: &[u8],
) -> anyhow::Result<()>
where
    H: FixedBaseMsms<Scalar = E::ScalarField, Base = E::G1Affine, MsmOutput = E::G1>, // TODO: Scalar should probably be Scalar<E>
    H: homomorphism::Trait<Codomain = H::CodomainShape<H::MsmOutput>>,
    H::Domain: Witness<E>,
    H::Codomain: Statement,
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

    let msm_terms = homomorphism.msm_terms(prover_last_message);
    let powers_of_beta = utils::powers(beta, msm_terms.clone().into_iter().count()); // TODO get rid of clone. is .count() an efficient way to get the length?

    let terms_iter = msm_terms.clone().into_iter(); // TODO: get rid of these clones
    let prover_iter = prover_first_message.clone().into_iter();
    let statement_iter = public_statement.clone().into_iter();

    let mut final_basis = Vec::new();
    let mut final_scalars = Vec::new();

    for (((term, A), P), beta_power) in terms_iter
        .zip(prover_iter)
        .zip(statement_iter)
        .zip(powers_of_beta)
    {
        // Destructure term and create a new MsmInput
        //let homomorphism::MsmInput { bases: term_bases, scalars: term_scalars } = term;
        let mut bases = term.bases().to_vec();
        let mut scalars = term.scalars().to_vec();

        for scalar in scalars.iter_mut() {
            *scalar *= beta_power;
        }

        // Append bases/scalars from prover and statement
        // Assuming MsmResult can be cloned to Base/Scalar type
        // You may need a conversion function if MsmResult is not exactly Base/Scalar
        // Here we just append placeholders (e.g., 1)
        bases.push(A.clone().into_affine()); // TODO: do a batch into affine
        bases.push(P.clone().into_affine()); // TODO: do a batch into affine

        scalars.push(-H::Scalar::from(1u8) * beta_power);
        scalars.push(-c * beta_power);

        final_basis.extend(bases);
        final_scalars.extend(scalars);
    }

    // Step 7: Perform the final MSM check
    let msm_result =
        E::G1::msm(&final_basis, &final_scalars).expect("Could not compute MSM for verifier");
    ensure!(msm_result == E::G1::ZERO);

    Ok(())
}
