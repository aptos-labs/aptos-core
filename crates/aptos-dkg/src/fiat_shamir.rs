// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and then implement that trait on `Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::{
    range_proofs::traits::BatchedRangeProof, sigma_protocol, sigma_protocol::homomorphism,
};
use ark_ec::{pairing::Pairing, AffineRepr};
use ark_ff::PrimeField;
use ark_serialize::CanonicalSerialize;
use merlin::Transcript;
use serde::Serialize;

/// Helper trait for deriving random scalars from a transcript.
///
/// Not every Fiat–Shamir call needs higher-level operations
/// (like appending PVSS information), but most do require scalar
/// derivation. This basic trait provides that functionality.
///
/// ⚠️ This trait is intentionally private: functions like `challenge_scalars`
/// should **only** be used internally to ensure properly
/// labelled scalar generation across Fiat-Shamir protocols.
trait ScalarProtocol<F: PrimeField> {
    fn challenge_full_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<F>;

    fn challenge_full_scalar(&mut self, label: &[u8]) -> F {
        self.challenge_full_scalars(label, 1)[0]
    }

    fn challenge_128bit_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<F>;
}

impl<F: PrimeField> ScalarProtocol<F> for Transcript {
    fn challenge_full_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<F> {
        let byte_size = (F::MODULUS_BIT_SIZE as usize) / 8;
        let mut buf = vec![0u8; 2 * num_scalars * byte_size];
        self.challenge_bytes(label, &mut buf);

        buf.chunks(2 * byte_size)
            .map(|chunk| F::from_le_bytes_mod_order(chunk))
            .collect()
    }

    fn challenge_128bit_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<F> {
        let mut buf = vec![0u8; num_scalars * 16];
        self.challenge_bytes(label, &mut buf);

        buf.chunks(16)
            .map(|chunk| F::from_le_bytes_mod_order(chunk.try_into().unwrap()))
            .collect()
    }
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
pub trait SigmaProtocol<F: PrimeField, H: homomorphism::Trait>: ScalarProtocol<F> {
    /// Append the "context" of a sigma protocol, e.g. session identifiers
    fn append_sigma_protocol_cntxt<Ct: Serialize>(&mut self, cntxt: &Ct);

    /// Append the MSM bases of a sigma protocol.
    fn append_sigma_protocol_msm_bases(&mut self, hom: &H);

    /// Append the claim of a sigma protocol.
    fn append_sigma_protocol_public_statement(&mut self, public_statement: &H::CodomainNormalized);

    /// Append the first message (the commitment) in a sigma protocol.
    fn append_sigma_protocol_first_prover_message(
        &mut self,
        prover_first_message: &H::CodomainNormalized,
    );

    // Returns a single scalar `r` for use in a Sigma protocol
    fn challenge_for_sigma_protocol(&mut self) -> F;
}

// These may or may not need a pairing, so for we're moving the generic parameters to the methods
pub trait PolynomialCommitmentScheme {
    fn append_sep(&mut self, dst: &[u8]);

    fn append_point<C: AffineRepr>(&mut self, point: &C);

    fn challenge_scalar<F: PrimeField>(&mut self) -> F;
}

#[allow(non_snake_case)]
impl<E: Pairing, B: BatchedRangeProof<E>> RangeProof<E, B> for Transcript {
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
        self.append_message(b"hat-f-commitment", commitment_bytes.as_slice());
    }

    fn append_sigma_proof<A: CanonicalSerialize>(&mut self, sigma_proof: &A) {
        let mut sigma_proof_bytes = Vec::new();
        sigma_proof
            .serialize_compressed(&mut sigma_proof_bytes)
            .expect("sigma proof serialization should succeed");
        self.append_message(b"sigma-proof-commitment", sigma_proof_bytes.as_slice());
    }

    fn append_f_j_commitments<A: CanonicalSerialize>(&mut self, f_j_commitments: &A) {
        let mut f_j_commitments_bytes = Vec::new();
        f_j_commitments
            .serialize_compressed(&mut f_j_commitments_bytes)
            .expect("f_j_commitments serialization should succeed");
        self.append_message(b"f-j-commitments", f_j_commitments_bytes.as_slice());
    }

    fn append_h_commitment<A: CanonicalSerialize>(&mut self, commitment: &A) {
        let mut commitment_bytes = Vec::new();
        commitment
            .serialize_compressed(&mut commitment_bytes)
            .expect("h_commitment serialization should succeed");
        self.append_message(b"h-commitment", commitment_bytes.as_slice());
    }

    fn challenges_for_quotient_polynomials(&mut self, ell: usize) -> Vec<E::ScalarField> {
        <Transcript as ScalarProtocol<E::ScalarField>>::challenge_128bit_scalars(
            self,
            b"challenge-for-quotient-polynomials",
            ell + 1,
        )
    }

    fn challenges_for_linear_combination(&mut self, num: usize) -> Vec<E::ScalarField> {
        <Transcript as ScalarProtocol<E::ScalarField>>::challenge_128bit_scalars(
            self,
            b"challenge-for-linear-combination",
            num,
        )
    }

    fn challenge_from_verifier(&mut self) -> E::ScalarField {
        <Transcript as ScalarProtocol<E::ScalarField>>::challenge_full_scalar(
            self,
            b"verifier-challenge-for-linear-combination",
        )
    }
}

impl<F: PrimeField, H: homomorphism::Trait + CanonicalSerialize> SigmaProtocol<F, H> for Transcript
where
    H::Domain: sigma_protocol::Witness<F>,
    H::CodomainNormalized: sigma_protocol::Statement,
{
    fn append_sigma_protocol_cntxt<Ct: Serialize>(&mut self, cntxt: &Ct) {
        let cntxt_bytes = bcs::to_bytes(cntxt).expect("cntxt data serialization should succeed");
        self.append_message(b"cntxt", cntxt_bytes.as_slice());
    }

    fn append_sigma_protocol_msm_bases(&mut self, hom: &H) {
        let mut hom_bytes = Vec::new();
        hom.serialize_compressed(&mut hom_bytes)
            .expect("hom MSM bases serialization should succeed");
        self.append_message(b"hom-msm-bases", hom_bytes.as_slice());
    }

    fn append_sigma_protocol_public_statement(&mut self, public_statement: &H::CodomainNormalized) {
        let mut public_statement_bytes = Vec::new();
        public_statement
            .serialize_compressed(&mut public_statement_bytes)
            .expect("public_statement serialization should succeed");
        self.append_message(b"sigma-protocol-claim", public_statement_bytes.as_slice());
    }

    fn append_sigma_protocol_first_prover_message(
        &mut self,
        prover_first_message: &H::CodomainNormalized,
    ) {
        let mut prover_first_message_bytes = Vec::new();
        prover_first_message
            .serialize_compressed(&mut prover_first_message_bytes)
            .expect("sigma protocol first message  serialization should succeed");
        self.append_message(
            b"sigma-protocol-first-message",
            prover_first_message_bytes.as_slice(),
        );
    }

    fn challenge_for_sigma_protocol(&mut self) -> F {
        <Transcript as ScalarProtocol<F>>::challenge_full_scalar(
            self,
            b"challenge-for-sigma-protocol",
        )
    }
}

impl PolynomialCommitmentScheme for Transcript {
    fn append_sep(&mut self, dst: &[u8]) {
        self.append_message(b"dom-sep", dst);
    }

    fn append_point<C: AffineRepr>(&mut self, point: &C) {
        let mut buf = Vec::new();
        point
            .serialize_compressed(&mut buf)
            .expect("Point serialization failed");
        self.append_message(b"point", &buf);
    }

    fn challenge_scalar<F: PrimeField>(&mut self) -> F {
        <Transcript as ScalarProtocol<F>>::challenge_full_scalar(self, b"challenge-for-pcs")
    }
}
