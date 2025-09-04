// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

//! This submodule implements the *public parameters* for this PVSS scheme.

use crate::{
    constants::G2_PROJ_NUM_BYTES,
    pvss::{encryption_elgamal, traits},
};
use velor_crypto::{CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use velor_crypto_derive::{DeserializeKey, SerializeKey};
use blstrs::{G1Projective, G2Projective};
use pairing::group::Group;

/// The size, in number of bytes, of a serialized `PublicParameters` struct.
const NUM_BYTES: usize = encryption_elgamal::g1::PUBLIC_PARAMS_NUM_BYTES + G2_PROJ_NUM_BYTES;

/// "Nothing up my sleeve" domain-separator tag (DST) for the hash-to-curve operation used
/// to pick our PVSS public parameters (group elements) as `hash_to_curve(seed, dst, group_element_name)`.
pub const DST_PVSS_PUBLIC_PARAMS: &[u8; 32] = b"VELOR_DISTRIBUTED_RANDOMNESS_DST";
/// "Nothing up my sleeve" seed to deterministically-derive the public parameters.
pub const SEED_PVSS_PUBLIC_PARAMS: &[u8; 33] = b"VELOR_DISTRIBUTED_RANDOMNESS_SEED";

/// The cryptographic *public parameters* needed to run the PVSS protocol.
#[derive(DeserializeKey, Clone, SerializeKey, Debug, PartialEq, Eq)]
pub struct PublicParameters {
    /// Encryption public parameters
    enc: encryption_elgamal::g1::PublicParameters,
    /// Base for the commitments to the polynomial evaluations (and for the dealt public key [shares])
    g_2: G2Projective,
}

impl PublicParameters {
    /// Verifiably creates public parameters from a public sequence of bytes `seed`.
    pub fn new_from_seed(seed: &[u8]) -> Self {
        let g = G1Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"g");
        let h = G1Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"h");
        debug_assert_ne!(g, h);
        PublicParameters {
            enc: encryption_elgamal::g1::PublicParameters::new(g, h),
            g_2: G2Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"g_2"),
        }
    }

    /// Verifiably creates public parameters from `SEED_PVSS_PUBLIC_PARAMS` but sets
    /// the encryption pubkey (and randomness) base $g$ to be the same as the generator in our
    /// BLS12-381 consensus signatures.
    pub fn default_with_bls_base() -> Self {
        let g = G1Projective::generator();
        let h = G1Projective::hash_to_curve(
            SEED_PVSS_PUBLIC_PARAMS,
            DST_PVSS_PUBLIC_PARAMS.as_slice(),
            b"h_with_bls_base",
        );
        debug_assert_ne!(g, h);
        PublicParameters {
            enc: encryption_elgamal::g1::PublicParameters::new(
                // Our BLS signatures over BLS12-381 curves use this generator as the base of their
                // PKs. We plan on (safely) reusing those BLS PKs as encryption PKs.
                g, h,
            ),
            g_2: G2Projective::hash_to_curve(
                SEED_PVSS_PUBLIC_PARAMS,
                DST_PVSS_PUBLIC_PARAMS.as_slice(),
                b"g_2_with_bls_base",
            ),
        }
    }

    /// Returns the base $g_2$ for the commitment to the polynomial.
    pub fn get_commitment_base(&self) -> &G2Projective {
        &self.g_2
    }

    /// Serializes the public parameters.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.enc.to_bytes().to_vec();

        bytes.append(&mut self.g_2.to_compressed().to_vec());

        bytes
    }
}

impl ValidCryptoMaterial for PublicParameters {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl Default for PublicParameters {
    /// Verifiably creates Velor-specific public parameters.
    fn default() -> Self {
        Self::default_with_bls_base()
    }
}

impl traits::HasEncryptionPublicParams for PublicParameters {
    type EncryptionPublicParameters = encryption_elgamal::g1::PublicParameters;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters {
        &self.enc
    }
}

impl TryFrom<&[u8]> for PublicParameters {
    type Error = CryptoMaterialError;

    /// Deserialize a `PublicParameters` struct.
    fn try_from(bytes: &[u8]) -> std::result::Result<PublicParameters, Self::Error> {
        let slice: &[u8; NUM_BYTES] = match <&[u8; NUM_BYTES]>::try_from(bytes) {
            Ok(slice) => slice,
            Err(_) => return Err(CryptoMaterialError::WrongLengthError),
        };

        let pp_bytes: [u8; encryption_elgamal::g1::PUBLIC_PARAMS_NUM_BYTES] = slice
            [0..encryption_elgamal::g1::PUBLIC_PARAMS_NUM_BYTES]
            .try_into()
            .unwrap();
        let g2_bytes = slice[encryption_elgamal::g1::PUBLIC_PARAMS_NUM_BYTES..NUM_BYTES]
            .try_into()
            .unwrap();

        let enc_pp = encryption_elgamal::g1::PublicParameters::try_from(&pp_bytes[..])?;
        let g2_opt = G2Projective::from_compressed(g2_bytes);

        if g2_opt.is_some().unwrap_u8() == 1u8 {
            Ok(PublicParameters {
                enc: enc_pp,
                g_2: g2_opt.unwrap(),
            })
        } else {
            Err(CryptoMaterialError::DeserializationError)
        }
    }
}
