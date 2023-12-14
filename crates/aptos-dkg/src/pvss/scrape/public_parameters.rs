// Copyright © Aptos Foundation

//! This submodule implements the *public parameters* for the SCRAPE PVSS scheme.

use crate::{
    constants::{DST_PVSS_PUBLIC_PARAMS, G2_PROJ_NUM_BYTES, SEED_PVSS_PUBLIC_PARAMS},
    pvss::{encryption_dlog, traits},
};
use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial, ValidCryptoMaterialStringExt};
use aptos_crypto_derive::{DeserializeKey, SerializeKey};
use blstrs::{G1Projective, G2Projective};

/// The size, in number of bytes, of a serialized `PublicParameters` struct.
const NUM_BYTES: usize = encryption_dlog::g2::PUBLIC_PARAMS_NUM_BYTES + 2 * G2_PROJ_NUM_BYTES;

/// The cryptographic *public parameters* needed to run the SCRAPE PVSS protocol.
#[derive(DeserializeKey, Clone, SerializeKey, Debug, PartialEq, Eq)]
pub struct PublicParameters {
    enc: encryption_dlog::g2::PublicParameters,
    /// Base for committing to the secret $a \in F$ as a group element $\hat{u}_1^a \in G_2$.
    u1_hat: G2Projective,
    /// Base for the Feldman commitment to the polynomial (and for the dealt public key [shares])
    g1: G1Projective,
}

impl PublicParameters {
    /// Verifiably creates public parameters from a public sequence of bytes `seed`.
    pub fn new_from_seed(seed: &[u8]) -> Self {
        PublicParameters {
            enc: encryption_dlog::g2::PublicParameters::new(G2Projective::hash_to_curve(
                seed,
                DST_PVSS_PUBLIC_PARAMS.as_slice(),
                b"h1_hat",
            )),
            u1_hat: G2Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"u1_hat"),
            g1: G1Projective::hash_to_curve(seed, DST_PVSS_PUBLIC_PARAMS.as_slice(), b"g1"),
        }
    }

    /// Returns the base $\hat{h}_1$ used for computing an encryption key $\hat{h}_1^{dk^{-1}}$.
    pub fn get_encryption_key_base(&self) -> &G2Projective {
        &self.enc.as_group_element()
    }

    /// Returns the base $\hat{u}_1$ used for computing the dealt public key $\hat{u}_1^a$ and shares of it.
    pub fn get_public_key_base(&self) -> &G2Projective {
        &self.u1_hat
    }

    /// Returns the base $g_1$ for the Feldman commitment to the polynomial.
    pub fn get_commitment_base(&self) -> &G1Projective {
        &self.g1
    }

    /// Serializes the public parameters.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = self.enc.as_group_element().to_compressed().to_vec();

        bytes.append(&mut self.u1_hat.to_compressed().to_vec());
        bytes.append(&mut self.g1.to_compressed().to_vec());

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
    type EncryptionPublicParameters = encryption_dlog::g2::PublicParameters;

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

        let h1_hat_bytes = slice[0..G2_PROJ_NUM_BYTES].try_into().unwrap();
        let u1_hat_bytes = slice[G2_PROJ_NUM_BYTES..2 * G2_PROJ_NUM_BYTES]
            .try_into()
            .unwrap();
        let g1_bytes = slice[2 * G2_PROJ_NUM_BYTES..NUM_BYTES].try_into().unwrap();

        let h1_hat_opt = G2Projective::from_compressed(h1_hat_bytes);
        let u1_hat_opt = G2Projective::from_compressed(u1_hat_bytes);
        let g1_opt = G1Projective::from_compressed(g1_bytes);

        if h1_hat_opt.is_some().unwrap_u8() == 1u8
            && u1_hat_opt.is_some().unwrap_u8() == 1u8
            && g1_opt.is_some().unwrap_u8() == 1u8
        {
            Ok(PublicParameters {
                enc: encryption_dlog::g2::PublicParameters::new(h1_hat_opt.unwrap()),
                u1_hat: u1_hat_opt.unwrap(),
                g1: g1_opt.unwrap(),
            })
        } else {
            Err(CryptoMaterialError::DeserializationError)
        }
    }
}
