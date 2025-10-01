// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and then implement that trait on `merlin::Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::{
    algebra::msm::Map,
    pvss::{traits::Transcript, ThresholdConfig},
    range_proofs::traits::BatchedRangeProof,
    sigma_protocol,
    utils::random::random_scalar_from_uniform_bytes,
};
use aptos_crypto::ValidCryptoMaterial;
use ark_ec::pairing::Pairing;
use ark_ff::{Field, PrimeField};
use ark_serialize::CanonicalSerialize;
use serde::Serialize;

pub const PVSS_DOM_SEP: &[u8; 21] = b"APTOS_SCRAPE_PVSS_DST"; // TODO: Name needs work, but check backwards-compatibility

/// Helper trait for deriving random scalars from a transcript.
///
/// Not every Fiat–Shamir call needs higher-level operations
/// (like appending PVSS information), but most do require scalar
/// derivation. This basic trait provides that functionality.
///
/// ⚠️ This trait is intentionally private: `challenge_scalars`
/// should **only** be used internally to ensure properly
/// labelled scalar generation across protocols.
pub trait ScalarProtocol<S: FromUniformBytes> {
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S>;

    fn challenge_scalar(&mut self, label: &[u8]) -> S {
        self.challenge_scalars(label, 1)[0]
    }
}

pub trait FromUniformBytes: Copy {
    const BYTE_SIZE: usize;
    fn from_uniform_bytes(bytes: &[u8]) -> Self;
}

/// This reason for this wrapper is to avoid "overlapping implementations" for blstrs::Scalar
#[derive(Clone, Copy)]
struct ArkworksScalar<E: Pairing>(E::ScalarField);

impl<E: Pairing> FromUniformBytes for ArkworksScalar<E> {
    const BYTE_SIZE: usize = (E::ScalarField::MODULUS_BIT_SIZE as usize) / 8;

    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        Self(E::ScalarField::from_le_bytes_mod_order(bytes))
    }
}

impl FromUniformBytes for blstrs::Scalar {
    const BYTE_SIZE: usize = crate::SCALAR_NUM_BYTES;

    fn from_uniform_bytes(bytes: &[u8]) -> Self {
        random_scalar_from_uniform_bytes(bytes.try_into().expect("Wrong byte length"))
    }
}

impl<S: FromUniformBytes> ScalarProtocol<S> for merlin::Transcript {
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S> {
        let mut buf = vec![0u8; 2 * num_scalars * S::BYTE_SIZE];
        self.challenge_bytes(label, &mut buf);

        let result = buf
            .chunks(2 * S::BYTE_SIZE)
            .map(S::from_uniform_bytes)
            .collect::<Vec<_>>();

        debug_assert_eq!(result.len(), num_scalars);
        result
    }
}

#[allow(non_snake_case)]
#[allow(private_bounds)]
pub trait PVSS<T: Transcript>: ScalarProtocol<blstrs::Scalar> {
    /// Append a domain separator for the PVSS protocol (in addition to the transcript-level DST used to initialise the FS transcript),
    /// consisting of a sharing configuration `sc`, which locks in the $t$ out of $n$ threshold.
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfig);

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

    fn append_commitments<A: CanonicalSerialize>(&mut self, commitments: &A);

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<E::ScalarField>;
}

#[allow(private_bounds)]
pub trait SigmaProtocol<E: Pairing, H: Map>: ScalarProtocol<ArkworksScalar<E>> {
    fn append_sigma_protocol_sep(&mut self, dst: &'static [u8]);

    /// Append the claim of a sigma protocol.
    fn append_sigma_protocol_public_statement(&mut self, public_statement: &H::Codomain);

    /// Append the first message (the commitment) in a sigma protocol.
    fn append_sigma_protocol_first_prover_message(&mut self, prover_first_message: &H::Codomain);

    /// Append the last message (the masked witness) in a sigma protocol.
    fn append_sigma_protocol_last_message(&mut self, prover_last_message: &H::Domain);

    // Returns a single scalar `r` for use in a Sigma protocol
    fn challenge_for_sigma_protocol(&mut self) -> E::ScalarField;
}

#[allow(non_snake_case)]
impl<T: Transcript> PVSS<T> for merlin::Transcript {
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfig) {
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
        <merlin::Transcript as ScalarProtocol<blstrs::Scalar>>::challenge_scalars(
            self,
            b"challenge_dual_code_word_polynomial",
            num_coeffs,
        )
    }

    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<blstrs::Scalar> {
        <merlin::Transcript as ScalarProtocol<blstrs::Scalar>>::challenge_scalars(
            self,
            b"challenge_linear_combination",
            num_scalars,
        )
    }
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

    fn append_commitments<A: CanonicalSerialize>(&mut self, commitments: &A) {
        let mut commitments_bytes = Vec::new();
        commitments
            .serialize_compressed(&mut commitments_bytes)
            .expect("commitments serialization should succeed");
        self.append_message(b"commitments", commitments_bytes.as_slice());
    }

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<E::ScalarField> {
        let mut buf = vec![0u8; num_scalars * 16];
        self.challenge_bytes(b"challenge_linear_combination", &mut buf);

        let mut v = Vec::with_capacity(num_scalars);

        for chunk in buf.chunks(16) {
            match chunk.try_into() {
                Ok(chunk) => {
                    v.push(
                        E::ScalarField::from_random_bytes(chunk)
                            .expect("Error sampling field elements from bytes"),
                    );
                },
                Err(_) => panic!("Expected a 16-byte slice, but got a different size"),
            }
        }

        assert_eq!(v.len(), num_scalars);

        v
    }
}

impl<E: Pairing, H: Map> SigmaProtocol<E, H> for merlin::Transcript
where
    H::Domain: sigma_protocol::Domain<E>,
    H::Codomain: sigma_protocol::Codomain,
{
    fn append_sigma_protocol_sep(&mut self, dst: &'static [u8]) {
        self.append_message(b"dom-sep", dst);
    }

    fn append_sigma_protocol_public_statement(&mut self, public_statement: &H::Codomain) {
        let mut public_statement_bytes = Vec::new();
        public_statement
            .serialize_compressed(&mut public_statement_bytes)
            .expect("public_statement0 serialization should succeed");
        self.append_message(b"sigma-protocol-claim", public_statement_bytes.as_slice());
    }

    fn append_sigma_protocol_first_prover_message(&mut self, prover_first_message: &H::Codomain) {
        let mut prover_first_message_bytes = Vec::new();
        prover_first_message
            .serialize_compressed(&mut prover_first_message_bytes)
            .expect("public_statement0 serialization should succeed");
        self.append_message(
            b"sigma-protocol-first-message",
            prover_first_message_bytes.as_slice(),
        );
    }

    fn append_sigma_protocol_last_message(&mut self, prover_last_message: &H::Domain) {
        let mut prover_last_message_bytes = Vec::new();
        prover_last_message
            .serialize_compressed(&mut prover_last_message_bytes)
            .expect("public_statement0 serialization should succeed");
        self.append_message(
            b"sigma-protocol-last-message",
            prover_last_message_bytes.as_slice(),
        );
    }

    fn challenge_for_sigma_protocol(&mut self) -> E::ScalarField {
        <merlin::Transcript as ScalarProtocol<ArkworksScalar<E>>>::challenge_scalar(
            self,
            b"challenge_sigma_protocol",
        )
        .0
    }
}

/// Securely derives a Fiat-Shamir challenge via Merlin.
/// Returns (n+1-t) random scalars for the SCRAPE LDT test (i.e., the random polynomial itself).
/// Additionally returns `num_scalars` random scalars for some linear combinations.
pub(crate) fn fiat_shamir_das<T: Transcript, A: Serialize>(
    // TODO: only used for das so might as well specify T
    trx: &T,
    sc: &ThresholdConfig,
    pp: &T::PublicParameters,
    spks: &Vec<T::SigningPubKey>,
    eks: &Vec<T::EncryptPubKey>,
    auxs: &Vec<A>,
    dst: &'static [u8],
    num_scalars: usize,
) -> (Vec<blstrs::Scalar>, Vec<blstrs::Scalar>) {
    let mut fs_t = merlin::Transcript::new(dst);

    <merlin::Transcript as PVSS<T>>::pvss_domain_sep(&mut fs_t, sc);
    <merlin::Transcript as PVSS<T>>::append_public_parameters(&mut fs_t, pp);
    <merlin::Transcript as PVSS<T>>::append_signing_pub_keys(&mut fs_t, spks);
    <merlin::Transcript as PVSS<T>>::append_encryption_keys(&mut fs_t, eks);
    <merlin::Transcript as PVSS<T>>::append_auxs(&mut fs_t, auxs);
    <merlin::Transcript as PVSS<T>>::append_transcript(&mut fs_t, trx);

    (
        <merlin::Transcript as PVSS<T>>::challenge_dual_code_word_polynomial(
            &mut fs_t,
            sc.t,
            sc.n + 1,
        ),
        <merlin::Transcript as PVSS<T>>::challenge_linear_combination_scalars(
            &mut fs_t,
            num_scalars,
        ),
    )
}
