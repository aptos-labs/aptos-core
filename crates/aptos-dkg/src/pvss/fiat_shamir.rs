// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and them implement that trait on `merlin::Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::range_proof;
use crate::{
    pvss::{threshold_config::ThresholdConfig, traits::Transcript},
    // utils::random::random_scalar_from_uniform_bytes,
};
use aptos_crypto::ValidCryptoMaterial;
use blstrs::G1Projective;
use serde::Serialize;

pub const PVSS_DOM_SEP: &[u8; 21] = b"APTOS_SCRAPE_PVSS_DST"; // TODO: Both names seem poorly chosen

/// Helper trait for deriving random scalars from a transcript.
///
/// Not every Fiat–Shamir call needs higher-level operations
/// (like appending PVSS information), but most do require scalar
/// derivation. This basic trait provides that functionality.
///
/// ⚠️ This trait is intentionally private: `challenge_scalars`
/// and `challenge_scalar` should **only** be used internally
/// to ensure consistent scalar generation across protocols.
trait ScalarProtocol<S: Scalar> {
    // **Auxiliary** function to return random scalars
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S>;
    // fn challenge_scalar(&mut self, label: &[u8]) -> S { // Not used yet
    //     self.challenge_scalars(label, 1)[0]
    // }
}

impl<S: Scalar> ScalarProtocol<S> for merlin::Transcript {
    fn challenge_scalars(&mut self, label: &[u8], num_scalars: usize) -> Vec<S> {
        let mut buf = vec![0u8; num_scalars * 2 * S::SCALAR_NUM_BYTES];
        self.challenge_bytes(label, &mut buf);

        let mut result = Vec::with_capacity(num_scalars);
        for chunk in buf.chunks_exact(2 * S::SCALAR_NUM_BYTES) {
            debug_assert_eq!(chunk.len(), 2 * S::SCALAR_NUM_BYTES);
            result.push(S::random_scalar_from_uniform_bytes(chunk));
        }

        assert_eq!(result.len(), num_scalars);
        result
    }
}

/// Abstraction over prime field scalars used in protocols. Will probably modify or remove this for arkworks
pub trait Scalar: Copy {
    const SCALAR_NUM_BYTES: usize;

    fn from_u128(v: u128) -> Self;
    fn random_scalar_from_uniform_bytes(bytes: &[u8]) -> Self; // Would like to enforce [u8; 2 * S_NUM_BYTES] here but that would need const generics
}

impl Scalar for blstrs::Scalar {
    const SCALAR_NUM_BYTES: usize = crate::SCALAR_NUM_BYTES;

    fn random_scalar_from_uniform_bytes(bytes: &[u8]) -> Self {
        debug_assert_eq!(bytes.len(), 2 * Self::SCALAR_NUM_BYTES);
        crate::utils::random::random_scalar_from_uniform_bytes(bytes.try_into().unwrap())
    }

    fn from_u128(v: u128) -> Self {
        <Self as ff::PrimeField>::from_u128(v)
    }
}

#[allow(non_snake_case)]
#[allow(private_bounds)]
pub trait PVSS<S: Scalar, T: Transcript>: ScalarProtocol<S> {
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

    // fn append_share_commitments<A: Serialize>(&mut self, commitments: &[A]); // Not used yet

    /// Appends the transcript
    fn append_transcript(&mut self, trx: &T);

    /// Returns a random dual-code word check polynomial for the SCRAPE LDT test.
    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n: usize) -> Vec<S>;

    /// Returns one or more scalars `r` useful for doing linear combinations (e.g., combining
    /// pairings in the SCRAPE multipairing check using coefficients $1, r, r^2, r^3, \ldots$
    /// TODO: maybe rename this / rewrite comment to mention the Schwartz-Zippel lemma? "challenge_for_schwartz_zippel"
    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<S>;
    // fn challenge_linear_combination_scalar(&mut self) -> S { // Not used yet
    //     self.challenge_linear_combination_scalars(1)[0]
    // }
}

pub trait RangeProof<S: Scalar> {
    fn append_range_proof_sep(&mut self);

    /// Append the claim of a range proof
    fn append_range_proof_claim(&mut self, claim: &(&range_proof::Commitment, &[G1Projective]));

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<S>;
}

#[allow(non_snake_case)]
impl<S: Scalar, T: Transcript> PVSS<S, T> for merlin::Transcript {
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfig) {
        self.append_message(b"dom-sep", PVSS_DOM_SEP);
        self.append_message(b"scheme-name", T::scheme_name().as_bytes());
        self.append_u64(b"t", sc.t as u64);
        self.append_u64(b"n", sc.n as u64);
    }

    fn append_public_parameters(&mut self, pp: &T::PublicParameters) {
        self.append_message(b"pp", pp.to_bytes().as_slice()); // TODO: Change to self.append_message(b"pp-hash", pp.hash()); ?
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
            <merlin::Transcript as PVSS<S, T>>::append_aux::<A>(self, aux);
        }
    }

    fn append_aux<A: Serialize>(&mut self, aux: &A) {
        let aux_bytes = bcs::to_bytes(aux).expect("aux data serialization should succeed");
        self.append_message(b"aux", aux_bytes.as_slice());
    }

    // fn append_share_commitments<A: Serialize>(&mut self, commitments: &[A]) {
    //     let commitments_bytes = bcs::to_bytes(commitments).expect("share commitment serialization should succeed");
    //     self.append_message(b"share_commitments", commitments_bytes.as_slice());
    // }

    fn append_transcript(&mut self, trx: &T) {
        self.append_message(b"transcript", trx.to_bytes().as_slice());
    }

    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n_plus_1: usize) -> Vec<S> {
        let num_coeffs = n_plus_1 - t;
        <merlin::Transcript as ScalarProtocol<S>>::challenge_scalars(
            self,
            b"challenge_dual_code_word_polynomial",
            num_coeffs,
        )
    }

    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<S> {
        <merlin::Transcript as ScalarProtocol<S>>::challenge_scalars(
            self,
            b"challenge_linear_combination",
            num_scalars,
        )
    }
}

#[allow(non_snake_case)]
impl<S: Scalar> RangeProof<S> for merlin::Transcript {
    fn append_range_proof_sep(&mut self) {
        self.append_message(b"dom-sep", range_proof::DST);
    }

    fn append_range_proof_claim(&mut self, claim: &(&range_proof::Commitment, &[G1Projective])) {
        let claim_bytes = bcs::to_bytes(claim).expect("claim data serialization should succeed");
        self.append_message(b"range-proof-claim", claim_bytes.as_slice());
    }

    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<S> {
        let mut buf = vec![0u8; num_scalars * 16];
        self.challenge_bytes(b"challenge_linear_combination", &mut buf);

        let mut v = Vec::with_capacity(num_scalars);

        for chunk in buf.chunks(16) {
            match chunk.try_into() {
                Ok(chunk) => {
                    v.push(S::from_u128(u128::from_le_bytes(chunk)));
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
pub(crate) fn fiat_shamir_das<S: Scalar, T: Transcript, A: Serialize>(
    // TODO: only used for das so might as well specify S and T
    trx: &T,
    sc: &ThresholdConfig,
    pp: &T::PublicParameters,
    spks: &Vec<T::SigningPubKey>,
    eks: &Vec<T::EncryptPubKey>,
    auxs: &Vec<A>,
    dst: &'static [u8],
    num_scalars: usize,
) -> (Vec<S>, Vec<S>) {
    let mut fs_t = merlin::Transcript::new(dst);

    <merlin::Transcript as PVSS<S, T>>::pvss_domain_sep(&mut fs_t, sc);
    <merlin::Transcript as PVSS<S, T>>::append_public_parameters(&mut fs_t, pp);
    <merlin::Transcript as PVSS<S, T>>::append_signing_pub_keys(&mut fs_t, spks);
    <merlin::Transcript as PVSS<S, T>>::append_encryption_keys(&mut fs_t, eks);
    <merlin::Transcript as PVSS<S, T>>::append_auxs(&mut fs_t, auxs);
    <merlin::Transcript as PVSS<S, T>>::append_transcript(&mut fs_t, trx);

    (
        <merlin::Transcript as PVSS<S, T>>::challenge_dual_code_word_polynomial(
            &mut fs_t,
            sc.t,
            sc.n + 1,
        ),
        <merlin::Transcript as PVSS<S, T>>::challenge_linear_combination_scalars(
            &mut fs_t,
            num_scalars,
        ),
    )
}
