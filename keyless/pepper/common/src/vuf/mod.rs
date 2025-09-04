// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use ark_std::rand::{CryptoRng, RngCore};

/// Implement this to define a VUF (verifiable unpredictable function).
pub trait VUF {
    type PrivateKey;
    type PublicKey;

    fn scheme_name() -> String;

    fn setup<R: CryptoRng + RngCore>(rng: &mut R) -> (Self::PrivateKey, Self::PublicKey);

    fn pk_from_sk(sk: &Self::PrivateKey) -> Result<Self::PublicKey>;

    /// Return `(output, proof)`.
    fn eval(sk: &Self::PrivateKey, input: &[u8]) -> Result<(Vec<u8>, Vec<u8>)>;

    fn verify(pk: &Self::PublicKey, input: &[u8], output: &[u8], proof: &[u8]) -> Result<()>;
}

/// a BLS VUF where:
/// - The underlying curve is BLS12-381.
/// - Input/output is in G1 and public key is in G2.
///
/// TODO: better name?
pub mod bls12381_g1_bls;
pub mod slip_10;
