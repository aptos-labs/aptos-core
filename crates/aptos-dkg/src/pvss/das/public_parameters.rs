// Copyright © Aptos Foundation

//! This submodule implements the *public parameters* for this PVSS scheme.

use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use blstrs::{G1Projective, G2Projective};
use pairing::group::Group;

use crate::constants::{DST_PVSS_PUBLIC_PARAMS, G2_PROJ_NUM_BYTES, SEED_PVSS_PUBLIC_PARAMS};
use crate::pvss::encryption_elgamal;
use crate::pvss::traits;

/// The size, in number of bytes, of a serialized `PublicParameters` struct.
const NUM_BYTES: usize = encryption_elgamal::g1::PUBLIC_PARAMS_NUM_BYTES + G2_PROJ_NUM_BYTES;

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
        PublicParameters {
            enc: encryption_elgamal::g1::PublicParameters::new(
                G1Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"g1"),
                G1Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"h1"),
            ),
            g_2: G2Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"g2"),
        }
    }
    /// Verifiably creates public parameters from a public sequence of bytes `seed` but sets
    /// the encryption pubkey (and randomness) base $g$ to be `ek_base`.
    pub fn new_from_seed_with_bls_base(seed: &[u8]) -> Self {
        PublicParameters {
            enc: encryption_elgamal::g1::PublicParameters::new(
                // Our BLS signatures over BLS12-381 curves use this generator as the base of their
                // PKs. We plan on (safely) reusing those BLS PKs as encryption PKs.
                G1Projective::generator(),
                G1Projective::hash_to_curve(
                    seed,
                    DST_PVSS_PUBLIC_PARAMS.as_slice(),
                    b"h1_with_ek_base",
                ),
            ),
            g_2: G2Projective::hash_to_curve(
                seed,
                DST_PVSS_PUBLIC_PARAMS.as_slice(),
                b"g2_with_ek_base",
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
    fn to_bytes(&self) -> Vec<u8> {
        self.to_bytes()
    }
}

impl Default for PublicParameters {
    /// Verifiably creates Aptos-specific public parameters.
    fn default() -> Self {
        Self::new_from_seed(SEED_PVSS_PUBLIC_PARAMS)
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
