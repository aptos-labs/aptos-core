// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and then implement that trait on `merlin::Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::{
    pvss::{traits::Transcript, ThresholdConfigBlstrs},
    range_proofs::traits::BatchedRangeProof,
    sigma_protocol,
    sigma_protocol::homomorphism,
    utils::random::random_scalar_from_uniform_bytes,
    Scalar,
};
use aptos_crypto::ValidCryptoMaterial;
use ark_ec::pairing::Pairing;
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use ff::PrimeField as FfPrimeField;
use serde::Serialize;

pub const PVSS_DOM_SEP: &[u8; 21] = b"APTOS_SCRAPE_PVSS_DST"; // TODO: Name needs work, but check backwards-compatibility

/// Helper trait for deriving random scalars from a transcript.
///
/// Not every Fiat–Shamir call needs higher-level operations
/// (like appending PVSS information), but most do require scalar
/// derivation. This basic trait provides that functionality.
///
/// ⚠️ This trait is intentionally private: functions like `challenge_scalars`
/// should **only** be used internally to ensure properly
/// labelled scalar generation across protocols.
trait ScalarProtocol<S: FromBytes> {
    fn challenge_full_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S>;

    fn challenge_full_scalar(&mut self, label: &[u8]) -> S {
        self.challenge_full_scalars(label, 1)[0]
    }

    fn challenge_128bit_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S>;
}

/// Trait for types that can be constructed from uniform bytes or 128-bit random bytes
trait FromBytes: Copy {
    const BYTE_SIZE: usize;

    /// Construct scalars, each from a uniform byte slice (usually larger than 16 bytes)
    fn from_uniform_bytes(bytes: &[u8]) -> Self;

    /// Construct scalars, each from exactly 16 bytes (128-bit randomness)
    fn from_16_random_bytes(bytes: &[u8; 16]) -> Self;
}

impl<E: Pairing> FromBytes for Scalar<E> {
    const BYTE_SIZE: usize = (E::ScalarField::MODULUS_BIT_SIZE as usize) / 8;

    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        assert_eq!(bytes.len(), 2 * Self::BYTE_SIZE);
        Self(E::ScalarField::from_le_bytes_mod_order(bytes))
    }

    fn from_16_random_bytes(bytes: &[u8; 16]) -> Self {
        Self(E::ScalarField::from_le_bytes_mod_order(bytes))
    }
}

impl FromBytes for blstrs::Scalar {
    const BYTE_SIZE: usize = crate::SCALAR_NUM_BYTES;

    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        // No assert_eq needed here because it is enforced by the function below
        random_scalar_from_uniform_bytes(bytes.try_into().expect("Wrong byte length"))
    }

    fn from_16_random_bytes(bytes: &[u8; 16]) -> Self {
        blstrs::Scalar::from_u128(u128::from_le_bytes(*bytes))
    }
}

impl<S: FromBytes> ScalarProtocol<S> for merlin::Transcript {
    fn challenge_full_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S> {
        let mut buf = vec![0u8; 2 * num_scalars * S::BYTE_SIZE];
        self.challenge_bytes(label, &mut buf); // Label is also appended here

        let result = buf
            .chunks(2 * S::BYTE_SIZE)
            .map(S::from_uniform_bytes)
            .collect::<Vec<_>>();

        debug_assert_eq!(result.len(), num_scalars);
        result
    }

    fn challenge_128bit_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S> {
        // Allocate 16 bytes (128 bits) per scalar
        let mut buf = vec![0u8; num_scalars * 16];
        self.challenge_bytes(label, &mut buf);

        let mut scalars = Vec::with_capacity(num_scalars);

        for chunk in buf.chunks(16) {
            scalars.push(S::from_16_random_bytes(chunk.try_into().unwrap()));
        }

        debug_assert_eq!(scalars.len(), num_scalars);
        scalars
    }
}

#[allow(non_snake_case)]
#[allow(private_bounds)]
pub trait PVSS<T: Transcript>: ScalarProtocol<blstrs::Scalar> {
    /// Append a domain separator for the PVSS protocol (in addition to the transcript-level DST used to initialise the FS transcript),
    /// consisting of a sharing configuration `sc`, which locks in the $t$ out of $n$ threshold.
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfigBlstrs);

    /// Append the public parameters `pp`.
    fn append_public_parameters(&mut self, pp: &T::PublicParameters);

    /// Append the signing pub keys.
    fn append_signing_pub_keys(&mut self, spks: &[T::SigningPubKey]);

    /// Append the encryption keys `eks`.
    fn append_encryption_keys(&mut self, eks: &[T::EncryptPubKey]);

    /// Append the aux data.
    fn append_auxs<A: Serialize>(&mut self, aux: &[A]);
    fn append_aux<A: Serialize>(&mut self, aux: &A);

    /// Appends the transcript
    fn append_transcript(&mut self, trx: &T);

    /// Returns a random dual-code word check polynomial for the SCRAPE LDT test.
    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n: usize) -> Vec<blstrs::Scalar>;

    /// Returns one or more scalars `r` useful for doing linear combinations (e.g., combining
    /// pairings in the SCRAPE multipairing check using coefficients $1, r, r^2, r^3, \ldots$
    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<blstrs::Scalar>;
}

pub trait RangeProof<E: Pairing, B: BatchedRangeProof<E>> {
    fn append_sep(&mut self, dst: &[u8]);

    fn append_vk(&mut self, vk: &B::VerificationKey);

    fn append_public_statement(&mut self, public_statement: B::PublicStatement);

    fn append_hat_f_commitment<A: CanonicalSerialize>(&mut self, commitment: &A);

    fn append_sigma_proof<A: CanonicalSerialize>(&mut self, sigma_proof: &A);

    fn append_f_j_commitments<A: CanonicalSerialize>(&mut self, f_j_commitments: &A);

    fn append_h_commitment<A: CanonicalSerialize>(&mut self, commitment: &A);

    fn challenges_for_quotient_polynomials(&mut self, ell: usize) -> Vec<E::ScalarField>;

    fn challenges_for_linear_combination(&mut self, ell: usize) -> Vec<E::ScalarField>;

    fn challenge_from_verifier(&mut self) -> E::ScalarField;
}

#[allow(private_bounds)]
pub trait SigmaProtocol<E: Pairing, H: homomorphism::Trait>: ScalarProtocol<Scalar<E>> {
    fn append_sigma_protocol_sep(&mut self, dst: &[u8]);

    /// Append the MSM bases of a sigma protocol.
    fn append_sigma_protocol_msm_bases(&mut self, hom: &H);

    /// Append the claim of a sigma protocol.
    fn append_sigma_protocol_public_statement(&mut self, public_statement: &H::Codomain);

    /// Append the first message (the commitment) in a sigma protocol.
    fn append_sigma_protocol_first_prover_message(&mut self, prover_first_message: &H::Codomain);

    /// Append the last message (the masked witness) in a sigma protocol.
    #[allow(dead_code)] // We ought to be using this, but are serializing the entire sigma proof
                        // because our security proofs like using fresh transcripts...
    fn append_sigma_protocol_last_message(&mut self, prover_last_message: &H::Domain);

    // Returns a single scalar `r` for use in a Sigma protocol
    fn challenge_for_sigma_protocol(&mut self) -> E::ScalarField;
}

#[allow(non_snake_case)]
impl<T: Transcript> PVSS<T> for merlin::Transcript {
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfigBlstrs) {
        self.append_message(b"dom-sep", PVSS_DOM_SEP);
        self.append_message(b"scheme-name", T::scheme_name().as_bytes());
        self.append_u64(b"t", sc.t as u64);
        self.append_u64(b"n", sc.n as u64);
    }

    fn append_public_parameters(&mut self, pp: &T::PublicParameters) {
        self.append_message(b"pp", pp.to_bytes().as_slice());
    }

    fn append_signing_pub_keys(&mut self, spks: &[T::SigningPubKey]) {
        self.append_u64(b"signing-pub-keys", spks.len() as u64);

        for spk in spks {
            self.append_message(b"spk", spk.to_bytes().as_slice())
        }
    }

    fn append_encryption_keys(&mut self, eks: &[T::EncryptPubKey]) {
        self.append_u64(b"encryption-keys", eks.len() as u64);

        for ek in eks {
            self.append_message(b"ek", ek.to_bytes().as_slice())
        }
    }

    fn append_auxs<A: Serialize>(&mut self, auxs: &[A]) {
        self.append_u64(b"auxs", auxs.len() as u64);
        for aux in auxs {
            <merlin::Transcript as PVSS<T>>::append_aux::<A>(self, aux);
        }
    }

    fn append_aux<A: Serialize>(&mut self, aux: &A) {
        let aux_bytes = bcs::to_bytes(aux).expect("aux data serialization should succeed");
        self.append_message(b"aux", aux_bytes.as_slice());
    }

    fn append_transcript(&mut self, trx: &T) {
        self.append_message(b"transcript", trx.to_bytes().as_slice());
    }

    fn challenge_dual_code_word_polynomial(
        &mut self,
        t: usize,
        n_plus_1: usize,
    ) -> Vec<blstrs::Scalar> {
        let num_coeffs = n_plus_1 - t;
        <merlin::Transcript as ScalarProtocol<blstrs::Scalar>>::challenge_full_scalars(
            self,
            b"challenge_dual_code_word_polynomial",
            num_coeffs,
        )
    }

    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<blstrs::Scalar> {
        <merlin::Transcript as ScalarProtocol<blstrs::Scalar>>::challenge_full_scalars(
            self,
            b"challenge_linear_combination",
            num_scalars,
        )
    }
}

pub fn initialize_pvss_transcript<T: Transcript>(
    sc: &ThresholdConfigBlstrs,
    pp: &T::PublicParameters,
    eks: &[T::EncryptPubKey],
    dst: &[u8],
) -> merlin::Transcript {
    let mut fs_t = merlin::Transcript::new(dst);

    <merlin::Transcript as PVSS<T>>::pvss_domain_sep(&mut fs_t, sc);
    <merlin::Transcript as PVSS<T>>::append_public_parameters(&mut fs_t, pp);
    <merlin::Transcript as PVSS<T>>::append_encryption_keys(&mut fs_t, eks);

    fs_t
}

#[allow(non_snake_case)]
impl<E: Pairing, B: BatchedRangeProof<E>> RangeProof<E, B> for merlin::Transcript {
    fn append_sep(&mut self, dst: &[u8]) {
        self.append_message(b"dom-sep", dst);
    }

    fn append_vk(&mut self, vk: &B::VerificationKey) {
        let mut vk_bytes = Vec::new();
        vk.serialize_compressed(&mut vk_bytes)
            .expect("vk serialization should succeed");
        self.append_message(b"vk", vk_bytes.as_slice());
    }

    fn append_public_statement(&mut self, public_statement: B::PublicStatement) {
        let mut public_statement_bytes = Vec::new();
        public_statement
            .serialize_compressed(&mut public_statement_bytes)
            .expect("public_statement0 serialization should succeed");
        self.append_message(b"public-statements", public_statement_bytes.as_slice());
    }

    fn append_hat_f_commitment<A: CanonicalSerialize>(&mut self, commitment: &A) {
        let mut commitment_bytes = Vec::new();
        commitment
            .serialize_compressed(&mut commitment_bytes)
            .expect("hat_f_commitment serialization should succeed");
        self.append_message(b"hat_f_commitment", commitment_bytes.as_slice());
    }

    fn append_sigma_proof<A: CanonicalSerialize>(&mut self, sigma_proof: &A) {
        let mut sigma_proof_bytes = Vec::new();
        sigma_proof
            .serialize_compressed(&mut sigma_proof_bytes)
            .expect("sigma proof serialization should succeed");
        self.append_message(b"sigma_proof_commitment", sigma_proof_bytes.as_slice());
    }

    fn append_f_j_commitments<A: CanonicalSerialize>(&mut self, f_j_commitments: &A) {
        let mut f_j_commitments_bytes = Vec::new();
        f_j_commitments
            .serialize_compressed(&mut f_j_commitments_bytes)
            .expect("f_j_commitments serialization should succeed");
        self.append_message(b"f_j_commitments", f_j_commitments_bytes.as_slice());
    }

    fn append_h_commitment<A: CanonicalSerialize>(&mut self, commitment: &A) {
        let mut commitment_bytes = Vec::new();
        commitment
            .serialize_compressed(&mut commitment_bytes)
            .expect("h_commitment serialization should succeed");
        self.append_message(b"h_commitment", commitment_bytes.as_slice());
    }

    fn challenges_for_quotient_polynomials(&mut self, ell: usize) -> Vec<E::ScalarField> {
        let challenges =
            <merlin::Transcript as ScalarProtocol<Scalar<E>>>::challenge_128bit_scalars(
                self,
                b"challenge_for_quotient_polynomials",
                ell + 1,
            );

        Scalar::<E>::vec_into_inner(challenges)
    }

    fn challenges_for_linear_combination(&mut self, num: usize) -> Vec<E::ScalarField> {
        let challenges =
            <merlin::Transcript as ScalarProtocol<Scalar<E>>>::challenge_128bit_scalars(
                self,
                b"challenge_for_linear_combination",
                num,
            );

        Scalar::<E>::vec_into_inner(challenges)
    }

    fn challenge_from_verifier(&mut self) -> E::ScalarField {
        <merlin::Transcript as ScalarProtocol<Scalar<E>>>::challenge_full_scalar(
            self,
            b"verifier_challenge_for_linear_combination",
        )
        .0
    }
}

impl<E: Pairing, H: homomorphism::Trait + CanonicalSerialize> SigmaProtocol<E, H>
    for merlin::Transcript
where
    H::Domain: sigma_protocol::Witness<E>,
    H::Codomain: sigma_protocol::Statement,
{
    fn append_sigma_protocol_sep(&mut self, dst: &[u8]) {
        self.append_message(b"dom-sep", dst);
    }

    fn append_sigma_protocol_msm_bases(&mut self, hom: &H) {
        let mut hom_bytes = Vec::new();
        hom.serialize_compressed(&mut hom_bytes)
            .expect("hom MSM bases serialization should succeed");
        self.append_message(b"hom-msm-bases", hom_bytes.as_slice());
    }

    fn append_sigma_protocol_public_statement(&mut self, public_statement: &H::Codomain) {
        let mut public_statement_bytes = Vec::new();
        public_statement
            .serialize_compressed(&mut public_statement_bytes)
            .expect("public_statement serialization should succeed");
        self.append_message(b"sigma-protocol-claim", public_statement_bytes.as_slice());
    }

    fn append_sigma_protocol_first_prover_message(&mut self, prover_first_message: &H::Codomain) {
        let mut prover_first_message_bytes = Vec::new();
        prover_first_message
            .serialize_compressed(&mut prover_first_message_bytes)
            .expect("sigma protocol first message  serialization should succeed");
        self.append_message(
            b"sigma-protocol-first-message",
            prover_first_message_bytes.as_slice(),
        );
    }

    fn append_sigma_protocol_last_message(&mut self, prover_last_message: &H::Domain) {
        let mut prover_last_message_bytes = Vec::new();
        prover_last_message
            .serialize_compressed(&mut prover_last_message_bytes)
            .expect("sigma protocol last message serialization should succeed");
        self.append_message(
            b"sigma-protocol-last-message",
            prover_last_message_bytes.as_slice(),
        );
    }

    fn challenge_for_sigma_protocol(&mut self) -> E::ScalarField {
        <merlin::Transcript as ScalarProtocol<Scalar<E>>>::challenge_full_scalar(
            self,
            b"challenge_sigma_protocol",
        )
        .0
    }
}
