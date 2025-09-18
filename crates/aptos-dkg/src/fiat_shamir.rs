// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and them implement that trait on `merlin::Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::{
    pvss::{traits::Transcript, ThresholdConfig},
    range_proof,
    utils::random::random_scalar_from_uniform_bytes,
    SCALAR_NUM_BYTES,
};
use aptos_crypto::ValidCryptoMaterial;
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::PrimeField;
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
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<Scalar>;
}

impl ScalarProtocol for merlin::Transcript {
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<Scalar> {
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
    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n: usize) -> Vec<Scalar>;

    /// Returns one or more scalars `r` useful for doing linear combinations (e.g., combining
    /// pairings in the SCRAPE multipairing check using coefficients $1, r, r^2, r^3, \ldots$
    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<Scalar>;
}

pub trait RangeProof {
    fn append_sep(&mut self);

    fn append_vk(&mut self, vk: &(&G1Projective, &G2Projective, &G2Projective, &G2Projective));

    fn append_public_statement(&mut self, public_statement: &(usize, &range_proof::Commitment));

    fn append_bit_commitments(&mut self, bit_commitments: &(&[G1Projective], &[G2Projective]));

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<Scalar>;
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

    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n_plus_1: usize) -> Vec<Scalar> {
        let num_coeffs = n_plus_1 - t;
        <merlin::Transcript as ScalarProtocol>::challenge_scalars(
            self,
            b"challenge_dual_code_word_polynomial",
            num_coeffs,
        )
    }

    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<Scalar> {
        <merlin::Transcript as ScalarProtocol>::challenge_scalars(
            self,
            b"challenge_linear_combination",
            num_scalars,
        )
    }
}

#[allow(non_snake_case)]
impl RangeProof for merlin::Transcript {
    fn append_sep(&mut self) {
        self.append_message(b"dom-sep", range_proof::DST);
    }

    fn append_vk(&mut self, vk: &(&G1Projective, &G2Projective, &G2Projective, &G2Projective)) {
        let vk_bytes = bcs::to_bytes(vk).expect("vk serialization should succeed");
        self.append_message(b"vk", vk_bytes.as_slice());
    }

    fn append_public_statement(&mut self, public_statement: &(usize, &range_proof::Commitment)) {
        let public_statement_bytes =
            bcs::to_bytes(public_statement).expect("public_statement serialization should succeed");
        self.append_message(b"public-statements", public_statement_bytes.as_slice());
    }

    fn append_bit_commitments(&mut self, bit_commitments: &(&[G1Projective], &[G2Projective])) {
        let bit_commitments_bytes =
            bcs::to_bytes(bit_commitments).expect("bit_commitments serialization should succeed");
        self.append_message(b"bit-commitments", bit_commitments_bytes.as_slice());
    }

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<Scalar> {
        let mut buf = vec![0u8; num_scalars * 16];
        self.challenge_bytes(b"challenge_linear_combination", &mut buf);

        let mut v = Vec::with_capacity(num_scalars);

        for chunk in buf.chunks(16) {
            match chunk.try_into() {
                Ok(chunk) => {
                    v.push(Scalar::from_u128(u128::from_le_bytes(chunk)));
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
) -> (Vec<Scalar>, Vec<Scalar>) {
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
