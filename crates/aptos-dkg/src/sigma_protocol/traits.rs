// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    fiat_shamir,
    sigma_protocol::homomorphism::{
        self,
        fixed_base_msms::{self},
    },
    Scalar,
};
use aptos_crypto::arkworks::msm::{IsMsmInput};
use anyhow::ensure;
use aptos_crypto::{arkworks::random::sample_field_element, utils};
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};
use ark_ff::AdditiveGroup;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Valid, Validate,
};
use ark_std::{io::Read, UniformRand};
use serde::Serialize;
use std::{fmt::Debug, io::Write};
use ark_ff::PrimeField;

pub trait Trait<E: Pairing>:
    fixed_base_msms::Trait<
        Domain: Witness<E::ScalarField>,
        MsmOutput = E::G1,
        MsmInput: IsMsmInput<Base = E::G1Affine, Scalar= E::ScalarField>, // need to be a bit specific because this code multiplies scalars and does into_affine(), etc
    > + Sized
    + CanonicalSerialize
{
    /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
    /// transcript operations within the protocol are uniquely namespaced
    fn dst(&self) -> Vec<u8>;

    fn prove<C: Serialize, R: rand_core::RngCore + rand_core::CryptoRng>(
        &self,
        witness: &Self::Domain,
        statement: &Self::Codomain,
        cntxt: &C, // for SoK purposes
        rng: &mut R,
    ) -> Proof<E, Self> {
        prove_homomorphism(self, witness, statement, cntxt, true, rng, &self.dst())
    }

    #[allow(non_snake_case)]
    fn verify<C: Serialize, H>(
        &self,
        public_statement: &Self::Codomain,
        proof: &Proof<E, H>, // Would like to set &Proof<E, Self>, but that ties the lifetime of H to that of Self, but we'd like it to be eg static
        cntxt: &C,
    ) -> anyhow::Result<()>
    where
        H: homomorphism::Trait<Domain = Self::Domain, Codomain = Self::Codomain>,
    {
        let (bases, scalars) = self.msm_terms_for_verify::<_, H>(
            public_statement,
            proof,
            cntxt,
        );

        let msm_result =
            E::G1::msm(&bases, &scalars).expect("Could not compute MSM for verifier");
        ensure!(msm_result == E::G1::ZERO);

        Ok(())
    }

    // Returns the MSM terms that `verify()` needs
    #[allow(non_snake_case)]
    fn msm_terms_for_verify<C: Serialize, H>(
        &self,
        public_statement: &Self::Codomain,
        proof: &Proof<E, H>,
        cntxt: &C,
    ) -> (Vec<<Self::MsmInput as IsMsmInput>::Base>, Vec<<Self::MsmInput as IsMsmInput>::Scalar>)
    where
        H: homomorphism::Trait<Domain = Self::Domain, Codomain = Self::Codomain>,
        {
            let prover_first_message = match &proof.first_proof_item {
                FirstProofItem::Commitment(A) => A,
                FirstProofItem::Challenge(_) => {
                    panic!("Missing implementation - expected commitment, not challenge")
                },
            };
            let c = fiat_shamir_challenge_for_sigma_protocol::<_, E, _>(
                cntxt,
                self,
                public_statement,
                &prover_first_message,
                &self.dst(),
            );

            // **Compute verifier-specific challenge (used for weighted MSM)**
            // While this could be derived deterministically via Fiat–Shamir, doing so would require
            // integrating it into the prover as well for composability; we no longer follow this approach.
            // Instead, we follow the simple approach:
            let mut rng = ark_std::rand::thread_rng(); // TODO: make this part of the function input?
            let beta = E::ScalarField::rand(&mut rng);
            
            let len = public_statement.clone().into_iter().count(); // hmm maybe pass the into_iter version in combine_msm_terms?
            let powers_of_betas = utils::powers(beta, len);

            let msm_terms_of_response = self.msm_terms(&proof.z);

            let (bases, scalars) = combine_msm_terms::<E, Self>(
                msm_terms_of_response.into_iter().collect(),
                prover_first_message,
                public_statement,
                powers_of_betas,
                c,
            );

            (bases, scalars)
        }        
}

// pub trait TraitTwo<E: Pairing>:
//     fixed_base_msms::TraitTwo<
//         Domain: Witness<E>,
//         Scalar = E::ScalarField,
//         FirstBase = E::G1Affine,
//         SecondBase = E::G2Affine,
//         FirstMsmOutput = E::G1,
//         SecondMsmOutput = E::G2,
//     > + Sized
//     + CanonicalSerialize
// {
//     /// Domain-separation tag (DST) used to ensure that all cryptographic hashes and
//     /// transcript operations within the protocol are uniquely namespaced
//     fn dst(&self) -> Vec<u8>;

//     fn prove<C: Serialize, R: rand_core::RngCore + rand_core::CryptoRng>(
//         &self,
//         witness: &Self::Domain,
//         statement: &Self::Codomain,
//         cntxt: &C, // for SoK purposes
//         rng: &mut R,
//     ) -> Proof<E, Self> {
//         prove_homomorphism(self, witness, statement, cntxt, true, rng, &self.dst())
//     }

//     #[allow(non_snake_case)]
//     fn verify<C: Serialize, H>(
//         &self,
//         public_statement: &Self::Codomain,
//         proof: &Proof<E, H>, // Would like to set &Proof<E, Self>, but that ties the lifetime of H to that of Self, but we'd like it to be eg static
//         cntxt: &C,
//     ) -> anyhow::Result<()>
//     where
//         H: homomorphism::Trait<Domain = Self::Domain, Codomain = Self::Codomain>,
//     {
//         let ((first_bases, first_scalars), (second_bases, second_scalars)) = self.msm_terms_for_verify::<_, H>(
//             public_statement,
//             proof,
//             cntxt,
//         );

//         let first_msm_result =
//             E::G1::msm(&first_bases, &first_scalars).expect("Could not compute MSM for verifier");
//         ensure!(first_msm_result == E::G1::ZERO);

//         let second_msm_result =
//             E::G2::msm(&second_bases, &second_scalars).expect("Could not compute MSM for verifier");
//         ensure!(second_msm_result == E::G2::ZERO);

//         Ok(())
//     }

//     // Returns the MSM terms that verify needs
//     #[allow(non_snake_case)]
//     fn msm_terms_for_verify<C: Serialize, H>(
//         &self,
//         public_statement: &Self::Codomain,
//         proof: &Proof<E, H>,
//         cntxt: &C,
//     ) -> ((Vec<Self::FirstBase>, Vec<Self::Scalar>), (Vec<Self::SecondBase>, Vec<Self::Scalar>))
//     where
//         H: homomorphism::Trait<Domain = Self::Domain, Codomain = Self::Codomain>,
//         {
//             let prover_first_message = match &proof.first_proof_item {
//                 FirstProofItem::Commitment(A) => A,
//                 FirstProofItem::Challenge(_) => {
//                     panic!("Missing implementation - expected commitment, not challenge")
//                 },
//             };
//             let c = fiat_shamir_challenge_for_sigma_protocol::<_, E, _>(
//                 cntxt,
//                 self,
//                 public_statement,
//                 &prover_first_message,
//                 &self.dst(),
//             );

//             // Step 2: Compute verifier-specific challenge (used for weighted MSM)
//             // While this could be derived deterministically via Fiat–Shamir, doing so would require
//             // integrating it into the prover as well for composability; we no longer follow this approach.
//             // Instead, we follow the simple approach:
//             let mut rng = ark_std::rand::thread_rng(); // TODO: make this part of the function input?
//             let beta = E::ScalarField::rand(&mut rng);

//             let msm_terms_of_response = self.msm_terms(&proof.z);

//             let (first_bases, first_scalars, _) = combine_msm_terms::<E, Self, <Self as fixed_base_msms::TraitTwo>::FirstMsmInput>(
//                 msm_terms_of_response.0.into_iter().collect(),
//                 &prover_first_message.0,
//                 &public_statement.0,
//                 beta, // TODO: aanpassen dit...
//                 c,
//                 0
//             );

//             ((first_bases, first_scalars), (second_bases, second_scalars))
//         }        
// }

use ark_ff::Field;

pub trait Witness<F: Field>: CanonicalSerialize + CanonicalDeserialize + Clone + Eq {
    /// Computes a scaled addition: `self + c * other`. Can take ownership because the
    /// randomness is discarded by the prover afterwards
    fn scaled_add(self, other: &Self, c: F) -> Self;

    /// Samples a random element in the domain. The prover has a witness `w` and calls `w.rand(rng)` to get
    /// the prover's first nonce (of the same "size" as `w`, hence why this cannot be a static method),
    /// which it then uses to compute the prover's first message in the sigma protocol.
    fn rand<R: rand_core::RngCore + rand_core::CryptoRng>(&self, rng: &mut R) -> Self;
}

// use ark_ff::FpConfig;
// use ark_ff::Fp;

// impl<const N: usize, P: FpConfig<N>> Witness for Fp<P, N> {
//     type Scalar = Fp<P, N>;

//     fn scaled_add(self, other: &Self, c: Fp<P, N>) -> Self {
//         Scalar(self.0 + (c) * other.0)
//     }

//     fn rand<R: rand_core::RngCore + rand_core::CryptoRng>(&self, rng: &mut R) -> Self {
//         Scalar(sample_field_element(rng))
//     }
// }

impl<F: PrimeField> Witness<F> for Scalar<F> {
    fn scaled_add(self, other: &Self, c: F) -> Self {
        Scalar(self.0 + (c) * other.0)
    }

    fn rand<R: rand_core::RngCore + rand_core::CryptoRng>(&self, rng: &mut R) -> Self {
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

    fn rand<R: rand_core::RngCore + rand_core::CryptoRng>(&self, rng: &mut R) -> Self {
        self.iter().map(|elem| elem.rand(rng)).collect()
    }
}

// Standard method to get `trait Statement = Canonical Serialize + ...`, because type aliases are experimental in Rust
pub trait Statement: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq {}
impl<T> Statement for T where T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq {}

/// The “first item” recorded in a Σ-proof, which is one of:
/// - The first message of the protocol, which is the commitment from the prover. This leads to a more compact proof.
/// - The second message of the protocol, which is the challenge from the verifier. This leads to a proof which is amenable to batch verification.
/// TODO: Better name? In https://github.com/sigma-rs/sigma-proofs these would be called "compact" and "batchable" proofs
#[derive(Clone, Debug, Eq)]
pub enum FirstProofItem<E: Pairing, H: homomorphism::Trait>
where
    H::Codomain: Statement,
{
    Commitment(H::Codomain),
    Challenge(E::ScalarField), // In more generality, this should be H::Domain::Scalar
}

// Manual implementation of PartialEq is required here because deriving PartialEq would
// automatically require `H` itself to implement PartialEq, which is undesirable.
impl<E: Pairing, H: homomorphism::Trait> PartialEq for FirstProofItem<E, H>
where
    H::Codomain: Statement,
{
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (FirstProofItem::Commitment(a), FirstProofItem::Commitment(b)) => a == b,
            (FirstProofItem::Challenge(a), FirstProofItem::Challenge(b)) => a == b,
            _ => false,
        }
    }
}

// The natural CanonicalSerialize/Deserialize implementations for `FirstProofItem`; we follow the usual approach for enums.
// CanonicalDeserialize needs Valid.
impl<E: Pairing, H: homomorphism::Trait> Valid for FirstProofItem<E, H>
where
    H::Domain: Witness<E::ScalarField>,
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
    H::Domain: Witness<E::ScalarField>,
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

impl<E: Pairing, H: homomorphism::Trait> CanonicalDeserialize for FirstProofItem<E, H>
where
    H::Domain: Witness<E::ScalarField>,
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

#[derive(CanonicalSerialize, Debug, CanonicalDeserialize, Clone)]
pub struct Proof<E: Pairing, H: homomorphism::Trait>
where
    H::Domain: Witness<E::ScalarField>,
    H::Codomain: Statement,
{
    /// The “first item” recorded in the proof, which can be either:
    /// - the prover's commitment (H::Codomain)
    /// - the verifier's challenge (E::ScalarField)
    pub first_proof_item: FirstProofItem<E, H>,
    /// Prover's second message (response)
    pub z: H::Domain,
}

impl<E: Pairing, H: homomorphism::Trait> Proof<E, H>
where
    H::Domain: Witness<E::ScalarField>,
    H::Codomain: Statement,
{
    /// No-op (semantically): circumvents the fact that proofs inherit the homomorphism’s lifetime. This method should do nothing at runtime.
    #[allow(non_snake_case)]
    pub fn change_lifetime<H2>(self) -> Proof<E, H2>
    where
        H2: homomorphism::Trait<Domain = H::Domain, Codomain = H::Codomain>,
    {
        let first = match self.first_proof_item {
            FirstProofItem::Commitment(A) => FirstProofItem::Commitment(A),
            FirstProofItem::Challenge(c) => FirstProofItem::Challenge(c),
        };

        Proof {
            first_proof_item: first,
            z: self.z,
        }
    }
}

// Manual implementation of PartialEq and Eq is required here because deriving PartialEq/Eq would
// automatically require `H` itself to implement PartialEq and Eq, which is undesirable.
// Workaround would be to make `Proof` generic over `H::Domain` and `H::Codomain` instead of `H`
impl<E: Pairing, H: homomorphism::Trait> PartialEq for Proof<E, H>
where
    H::Domain: Witness<E::ScalarField>,
    H::Codomain: Statement,
{
    fn eq(&self, other: &Self) -> bool {
        self.first_proof_item == other.first_proof_item && self.z == other.z
    }
}

// Empty because it simply asserts reflexivity
impl<E: Pairing, H: homomorphism::Trait> Eq for Proof<E, H>
where
    H::Domain: Witness<E::ScalarField>,
    H::Codomain: Statement,
{
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
    C: Serialize,
    E: Pairing,
    H: homomorphism::Trait + CanonicalSerialize,
>(
    cntxt: &C,
    hom: &H,
    statement: &H::Codomain,
    prover_first_message: &H::Codomain,
    dst: &[u8],
) -> E::ScalarField
where
    H::Domain: Witness<E::ScalarField>,
    H::Codomain: Statement,
{
    // Initialise the transcript
    let mut fs_t = merlin::Transcript::new(dst);

    // Append the "context" to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E::ScalarField, H>>::append_sigma_protocol_ctxt(
        &mut fs_t, cntxt,
    );

    // Append the MSM bases to the transcript. (If the same hom is used for many proofs, maybe use a single transcript + a boolean to prevent it from repeating?)
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E::ScalarField, H>>::append_sigma_protocol_msm_bases(
        &mut fs_t, hom,
    );

    // Append the public statement (the image of the witness) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E::ScalarField, H>>::append_sigma_protocol_public_statement(
        &mut fs_t,
        statement,
    );

    // Add the first prover message (the commitment) to the transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E::ScalarField, H>>::append_sigma_protocol_first_prover_message(
        &mut fs_t,
        prover_first_message,
    );

    // Generate the Fiat-Shamir challenge from the updated transcript
    <merlin::Transcript as fiat_shamir::SigmaProtocol<E::ScalarField, H>>::challenge_for_sigma_protocol(
        &mut fs_t,
    )
}

#[allow(non_snake_case)]
pub fn prove_homomorphism<
    C: Serialize,
    E: Pairing,
    H: homomorphism::Trait,
    R,
>(
    homomorphism: &H,
    witness: &H::Domain,
    statement: &H::Codomain,
    cntxt: &C,
    store_prover_commitment: bool, // true = store prover's commitment, false = store Fiat-Shamir challenge
    rng: &mut R,
    dst: &[u8],
) -> Proof<E, H>
where
    H::Domain: Witness<E::ScalarField>,
    H::Codomain: Statement,
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    // Step 1: Sample randomness. Here the `witness` is only used to make sure that `r` has the right dimensions
    let r = witness.rand(rng);

    // Step 2: Compute commitment A = Ψ(r)
    let A = homomorphism.apply(&r);

    // Step 3: Obtain Fiat-Shamir challenge
    let c =
        fiat_shamir_challenge_for_sigma_protocol::<_, E, H>(cntxt, homomorphism, statement, &A, dst);

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

// This function is currently not used, see comments in `fn verify()`
// pub fn fiat_shamir_challenge_for_msm_verifier<E: Pairing, H: homomorphism::Trait>(
//     fs_transcript: &mut merlin::Transcript,
//     public_statement: &H::Codomain,
//     prover_last_message: &H::Domain,
//     dst: &[u8],
// ) -> E::ScalarField
// where
//     H::Domain: Witness<E>,
//     H::Codomain: Statement,
// {
//     // Append the Σ-protocol separator to the transcript
//     <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_sep(
//         fs_transcript,
//         dst,
//     );

//     // Add the last prover message (the prover's response) to the transcript
//     <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_last_message(
//         fs_transcript,
//         prover_last_message,
//     );

//     // Add the public statment (the image of the prover's witness) to the transcript
//     <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::append_sigma_protocol_public_statement(
//         fs_transcript,
//         public_statement,
//     );

//     // Generate the Fiat-Shamir challenge from the updated transcript
//     <merlin::Transcript as fiat_shamir::SigmaProtocol<E, H>>::challenge_for_sigma_protocol(
//         fs_transcript,
//     )
// }

/// Performs a **batch verification** of multiple Sigma protocol MSM (Multi-Scalar Multiplication) relations.
///
/// ### Overview
/// Suppose we need to verify a family of equations of the form:
///
/// ```text
/// ∑_i g_{i,j} * x_{i,j} = A_j + P_j * c      for each index j.
/// ```
///
/// Instead of checking each equation individually, we batch them using a random challenge \(\beta\).
/// The verifier checks that:
///
/// ```text
/// ∑_j β^j * ( ∑_i g_{i,j} * x_{i,j} - A_j - P_j * c ) = 0
/// ```
///
/// This reduces the verification of multiple MSM-based equations to a single MSM check,
/// significantly improving efficiency.
///
/// ### Generalization
/// This batching technique can be extended to simultaneously verify multiple protocols
/// that involve MSM relations.
///
/// ### Notes
/// - The random challenge \(\beta\) is currently sampled locally by the verifier, for composability with larger protocols. But this could be derived via Fiat–Shamir as well. (TODO: Pending discussion)
///
/// ### TODO
/// - The code is currently set up on the case where the MSM input representation is
///   `Vec<(Scalars, Bases)> = Vec<(E::ScalarField, E::G1Affine)>`. If we want to add a
///   homomorphism whose codomain has components in both G_1 and G_2, we should probably put
///   the `Bases` component and MsmOutput inside of enums.
// #[allow(non_snake_case)]
// pub fn verify_msm_hom<C: Serialize, E: Pairing, H>(
//     homomorphism: &H,
//     statement: &H::Codomain,
//     prover_first_message: &H::Codomain,
//     prover_last_message: &H::Domain,
//     cntxt: &C,
//     dst: &[u8],
// ) -> anyhow::Result<()>
// where
// //     H: fixed_base_msms::Trait<Scalar = E::ScalarField, Base = E::G1Affine, MsmOutput = E::G1, MsmInput = MsmInput<E::G1Affine, E::ScalarField>>
//     H: fixed_base_msms::Trait<Scalar = E::ScalarField, Base = E::G1Affine, MsmOutput = E::G1>
//         + CanonicalSerialize,
//     H::Domain: Witness<E>,
// {
//     // Step 1: Reproduce the prover's Fiat-Shamir challenge
//     let c = fiat_shamir_challenge_for_sigma_protocol::<_, E, H>(
//         cntxt,
//         homomorphism,
//         statement,
//         &prover_first_message,
//         dst,
//     );

//     // Step 2: Compute verifier-specific challenge (used for weighted MSM)
//     // While this could be derived deterministically via Fiat–Shamir, doing so would require
//     // integrating it into the prover as well for composability; we no longer follow this approach.
//     // Instead, we follow the simple approach:
//     let mut rng = ark_std::rand::thread_rng(); // TODO: make this part of the function input?
//     let beta = E::ScalarField::rand(&mut rng);

//     let msm_terms = homomorphism.msm_terms(prover_last_message);
//     // let powers_of_beta = utils::powers(beta, statement.clone().into_iter().count()); // TODO: Maybe get rid of clone? Is .count() an efficient way to get the length?

//     // let terms_iter = msm_terms.clone().into_iter(); // TODO: get rid of these clones?
//     // let prover_iter = prover_first_message.clone().into_iter();
//     // let statement_iter = statement.clone().into_iter();

//     // let mut final_bases = Vec::new();
//     // let mut final_scalars = Vec::new();

//     // for (((term, A), P), beta_power) in terms_iter
//     //     .zip(prover_iter)
//     //     .zip(statement_iter)
//     //     .zip(powers_of_beta)
//     // {
//     //     // Destructure term and create a new MsmInput
//     //     let mut bases = term.bases().to_vec();
//     //     let mut scalars = term.scalars().to_vec();

//     //     for scalar in scalars.iter_mut() {
//     //         *scalar *= beta_power;
//     //     }

//     //     // Append bases/scalars from prover and statement
//     //     bases.push(A.clone().into_affine()); // TODO: do a batch into affine
//     //     bases.push(P.clone().into_affine()); // TODO: do a batch into affine

//     //     scalars.push(-beta_power);
//     //     scalars.push(-c * beta_power);

//     //     final_bases.extend(bases);
//     //     final_scalars.extend(scalars);
//     // }

//     let (final_bases, final_scalars, _) = combine_msm_terms::<E, H, <H as fixed_base_msms::Trait>::MsmInput>(
//         msm_terms.into_iter().collect(),
//         prover_first_message,
//         statement,
//         beta,
//         c,
//         0
//     );

//     // Step 7: Perform the final MSM check. TODO: Could use msm_eval here?
//     let msm_result =
//         E::G1::msm(&final_bases, &final_scalars).expect("Could not compute MSM for verifier");
//     ensure!(msm_result == E::G1::ZERO);

//     Ok(())
// }

/// The MSM terms of the sigma protocol. Instead of computing the answer, returning the terms in thsi form
/// is useful for combining with the MSM terms of other protocols, but note that beta powers are already being
/// added here because it's convenient (and slightly faster) to do that when the c factor is being added
#[allow(non_snake_case)]
fn combine_msm_terms<E, H>( // can't we associate M to H?
    msm_terms: Vec<H::MsmInput>,
    prover_first_message: &H::Codomain,
    statement: &H::Codomain,
    powers_of_beta: Vec<E::ScalarField>,
    c: E::ScalarField,
) -> (Vec<E::G1Affine>, Vec<E::ScalarField>) // change the types here??
where
    E: Pairing,
    H: fixed_base_msms::Trait<
        MsmOutput = E::G1,
    >,
    H::Codomain: Clone + IntoIterator,
    H::MsmInput: IsMsmInput<Base = E::G1Affine, Scalar= E::ScalarField>,
{
    // let len = statement.clone().into_iter().count() + excess_betas;
    // let powers_of_betas = utils::powers(beta, len);
    // let (needed_betas, excess_betas) = powers_of_betas.split_at(excess_betas);

    let mut final_basis = Vec::new();
    let mut final_scalars = Vec::new();

    for (((term, A), P), beta_power) in msm_terms
        .into_iter()
        .zip(prover_first_message.clone().into_iter())
        .zip(statement.clone().into_iter())
        .zip(powers_of_beta)
    {
        let mut bases = term.bases().to_vec();
        let mut scalars = term.scalars().to_vec();

        // multiply scalars by βᶦ
        for scalar in scalars.iter_mut() {
            *scalar *= beta_power;
        }

        // add prover + statement contributions
        bases.push(A.into_affine()); // TODO: batch affine conversion
        bases.push(P.into_affine()); // TODO: batch affine conversion

        scalars.push(-beta_power);
        scalars.push(-c * beta_power);

        final_basis.extend(bases);
        final_scalars.extend(scalars);
    }

    (final_basis, final_scalars)
}