// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and then implement that trait on `merlin::Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::{
    pvss::{traits::Transcript, ThresholdConfig},
    range_proofs::univariate_range_proof,
    utils::random::random_scalar_from_uniform_bytes,
    SCALAR_NUM_BYTES,
};
use aptos_crypto::ValidCryptoMaterial;
use ark_ec::pairing::Pairing;
use ark_ff::Field;
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
trait ScalarProtocol {
    /// **Auxiliary** function to return random scalars
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<blstrs::Scalar>;
}

impl ScalarProtocol for merlin::Transcript {
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<blstrs::Scalar> {
        let mut buf = vec![0u8; num_scalars * 2 * SCALAR_NUM_BYTES];
        self.challenge_bytes(label, &mut buf);

        let mut result = Vec::with_capacity(num_scalars);
        for chunk in buf.chunks(2 * SCALAR_NUM_BYTES) {
            match chunk.try_into() {
                Ok(chunk) => {
                    result.push(random_scalar_from_uniform_bytes(chunk));
                },
                Err(_) => panic!("Expected a 64-byte slice, but got a different size"),
            }
        }

        debug_assert_eq!(result.len(), num_scalars);
        result
    }
}

#[allow(non_snake_case)]
#[allow(private_bounds)]
pub trait PVSS<T: Transcript>: ScalarProtocol {
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

pub trait RangeProof<E: Pairing> {
    fn append_sep(&mut self);

    fn append_vk(&mut self, vk: &(&E::G1, &E::G2, &E::G2, &E::G2));

    fn append_public_statement(
        &mut self,
        public_statement: &(usize, &univariate_range_proof::Commitment<E>),
    );

    fn append_bit_commitments(&mut self, bit_commitments: &(&[E::G1], &[E::G2]));

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<E::ScalarField>;
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
        <merlin::Transcript as ScalarProtocol>::challenge_scalars(
            self,
            b"challenge_dual_code_word_polynomial",
            num_coeffs,
        )
    }

    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<blstrs::Scalar> {
        <merlin::Transcript as ScalarProtocol>::challenge_scalars(
            self,
            b"challenge_linear_combination",
            num_scalars,
        )
    }
}

use ark_serialize::CanonicalSerialize;

#[allow(non_snake_case)]
impl<E: Pairing> RangeProof<E> for merlin::Transcript {
    fn append_sep(&mut self) {
        self.append_message(b"dom-sep", univariate_range_proof::DST);
    }

    fn append_vk(&mut self, vk: &(&E::G1, &E::G2, &E::G2, &E::G2)) {
        let mut vk_bytes = Vec::new();
        vk.0.serialize_compressed(&mut vk_bytes) // TODO: change this
            .expect("vk0 serialization should succeed");
        vk.1.serialize_compressed(&mut vk_bytes)
            .expect("vk1 serialization should succeed");
        vk.2.serialize_compressed(&mut vk_bytes)
            .expect("vk2 serialization should succeed");
        vk.3.serialize_compressed(&mut vk_bytes)
            .expect("vk3 serialization should succeed");
        self.append_message(b"vk", vk_bytes.as_slice());
    }

    fn append_public_statement(
        &mut self,
        public_statement: &(usize, &univariate_range_proof::Commitment<E>),
    ) {
        let mut public_statement_bytes = Vec::new();
        public_statement
            .0
            .serialize_compressed(&mut public_statement_bytes)
            .expect("public_statement0 serialization should succeed");
        public_statement
            .1
            .serialize_compressed(&mut public_statement_bytes)
            .expect("public_statement1 serialization should succeed");
        // TODO: CHANGE THIS STUFF
        self.append_message(b"public-statements", public_statement_bytes.as_slice());
    }

    fn append_bit_commitments(&mut self, bit_commitments: &(&[E::G1], &[E::G2])) {
        let mut bit_commitments_bytes = Vec::new();
        bit_commitments
            .serialize_compressed(&mut bit_commitments_bytes)
            .expect("bit_commitments serialization should succeed");
        self.append_message(b"bit-commitments", bit_commitments_bytes.as_slice());
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
