// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! For what it's worth, I don't understand why the `merlin` library wants the user to first define
//! a trait with their 'append' operations and them implement that trait on `merlin::Transcript`.
//! I also don't understand how that doesn't break the orphan rule in Rust.
//! I suspect the reason they want the developer to do things these ways is to force them to cleanly
//! define all the things that are appended to the transcript.

use crate::{
    pvss::{threshold_config::ThresholdConfig, traits::Transcript},
    utils::random::random_scalar_from_uniform_bytes,
    SCALAR_NUM_BYTES,
};
use velor_crypto::ValidCryptoMaterial;
use blstrs::Scalar;
use ff::PrimeField;
use serde::Serialize;

pub const PVSS_DOM_SEP: &[u8; 21] = b"VELOR_SCRAPE_PVSS_DST";

#[allow(non_snake_case)]
pub trait FiatShamirProtocol<T: Transcript> {
    /// Append a domain separator for the PVSS protocol, consisting of a sharing configuration `sc`,
    /// which locks in the $t$ out of $n$ threshold.
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfig);

    /// Append the public parameters `pp`.
    fn append_public_parameters(&mut self, pp: &T::PublicParameters);

    /// Append the signing pub keys.
    fn append_signing_pub_keys(&mut self, spks: &Vec<T::SigningPubKey>);

    /// Append the encryption keys `eks`.
    fn append_encryption_keys(&mut self, eks: &Vec<T::EncryptPubKey>);

    /// Append the aux data.
    fn append_auxs<A: Serialize>(&mut self, aux: &Vec<A>);

    /// Appends the transcript
    fn append_transcript(&mut self, trx: &T);

    /// Returns a random dual-code word check polynomial for the SCRAPE LDT test.
    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n: usize) -> Vec<Scalar>;

    /// Returns one or more scalars `r` useful for doing linear combinations (e.g., combining
    /// pairings in the SCRAPE multipairing check using coefficients $1, r, r^2, r^3, \ldots$
    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<Scalar>;

    #[allow(dead_code)]
    fn challenge_linear_combination_128bit(&mut self, num_scalars: usize) -> Vec<Scalar>;
}

#[allow(non_snake_case)]
impl<T: Transcript> FiatShamirProtocol<T> for merlin::Transcript {
    fn pvss_domain_sep(&mut self, sc: &ThresholdConfig) {
        self.append_message(b"dom-sep", PVSS_DOM_SEP);
        self.append_message(b"scheme-name", T::scheme_name().as_bytes());
        self.append_u64(b"t", sc.t as u64);
        self.append_u64(b"n", sc.n as u64);
    }

    fn append_public_parameters(&mut self, pp: &T::PublicParameters) {
        self.append_message(b"pp", pp.to_bytes().as_slice());
    }

    fn append_signing_pub_keys(&mut self, spks: &Vec<T::SigningPubKey>) {
        self.append_u64(b"signing-pub-keys", spks.len() as u64);

        for spk in spks {
            self.append_message(b"spk", spk.to_bytes().as_slice())
        }
    }

    fn append_encryption_keys(&mut self, eks: &Vec<T::EncryptPubKey>) {
        self.append_u64(b"encryption-keys", eks.len() as u64);

        for ek in eks {
            self.append_message(b"ek", ek.to_bytes().as_slice())
        }
    }

    fn append_auxs<A: Serialize>(&mut self, auxs: &Vec<A>) {
        self.append_u64(b"auxs", auxs.len() as u64);

        for aux in auxs {
            let aux_bytes = bcs::to_bytes(aux).expect("aux data serialization should succeed");
            self.append_message(b"aux", aux_bytes.as_slice())
        }
    }

    fn append_transcript(&mut self, trx: &T) {
        self.append_message(b"transcript", trx.to_bytes().as_slice());
    }

    fn challenge_dual_code_word_polynomial(&mut self, t: usize, n_plus_1: usize) -> Vec<Scalar> {
        let num_coeffs = n_plus_1 - t;

        let num_bytes = num_coeffs * 2 * SCALAR_NUM_BYTES;
        let mut buf = vec![0u8; num_bytes];

        self.challenge_bytes(b"challenge_dual_code_word_polynomial", &mut buf);

        let mut f = Vec::with_capacity(num_coeffs);

        for chunk in buf.chunks(2 * SCALAR_NUM_BYTES) {
            match chunk.try_into() {
                Ok(chunk) => {
                    f.push(random_scalar_from_uniform_bytes(chunk));
                },
                Err(_) => panic!("Expected a slice of size 64, but got a different size"),
            }
        }

        assert_eq!(f.len(), num_coeffs);

        f
    }

    fn challenge_linear_combination_scalars(&mut self, num_scalars: usize) -> Vec<Scalar> {
        let mut buf = vec![0u8; num_scalars * 2 * SCALAR_NUM_BYTES];
        self.challenge_bytes(b"challenge_linear_combination", &mut buf);

        let mut v = Vec::with_capacity(num_scalars);

        // To ensure we pick a uniform Scalar, we sample twice the number of bytes in a scalar and
        // reduce those bytes modulo the order of the scalar field.
        for chunk in buf.chunks(2 * SCALAR_NUM_BYTES) {
            match chunk.try_into() {
                Ok(chunk) => {
                    v.push(random_scalar_from_uniform_bytes(chunk));
                },
                Err(_) => panic!("Expected a 64-byte slice, but got a different size"),
            }
        }

        assert_eq!(v.len(), num_scalars);

        v
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
pub(crate) fn fiat_shamir<T: Transcript, A: Serialize>(
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

    <merlin::Transcript as FiatShamirProtocol<T>>::pvss_domain_sep(&mut fs_t, sc);
    <merlin::Transcript as FiatShamirProtocol<T>>::append_public_parameters(&mut fs_t, pp);
    <merlin::Transcript as FiatShamirProtocol<T>>::append_signing_pub_keys(&mut fs_t, spks);
    <merlin::Transcript as FiatShamirProtocol<T>>::append_encryption_keys(&mut fs_t, eks);
    <merlin::Transcript as FiatShamirProtocol<T>>::append_auxs(&mut fs_t, auxs);
    <merlin::Transcript as FiatShamirProtocol<T>>::append_transcript(&mut fs_t, trx);

    (
        <merlin::Transcript as FiatShamirProtocol<T>>::challenge_dual_code_word_polynomial(
            &mut fs_t,
            sc.t,
            sc.n + 1,
        ),
        <merlin::Transcript as FiatShamirProtocol<T>>::challenge_linear_combination_scalars(
            &mut fs_t,
            num_scalars,
        ),
    )
}
