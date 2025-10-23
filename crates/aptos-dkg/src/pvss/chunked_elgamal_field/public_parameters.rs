// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! This submodule implements the *public parameters* for this "chunked_elgamal_field" PVSS scheme.

use crate::{
    algebra::GroupGenerators, pvss::{chunked_elgamal_field::chunked_elgamal::PublicParameters as PublicParametersElgamal, traits}, range_proofs::dekart_univariate_v2, utils
};
use ark_serialize::CanonicalSerialize;
use aptos_crypto::{CryptoMaterialError, ValidCryptoMaterial};
use blstrs::{G2Projective, Scalar};
use ark_ec::pairing::Pairing;
// use traits::transcript::WithMaxNumShares;

impl<E: Pairing> traits::HasEncryptionPublicParams for PublicParameters<E> {
    type EncryptionPublicParameters = PublicParametersElgamal<E>;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters {
        &self.pp_elgamal
    }
}

// use crate::constants::build_constants;

// #[cfg(feature = "kangaroo")]
// use kangaroo_dlog::{Kangaroo, ActiveCurve, presets::Presets};

#[derive(CanonicalSerialize, Clone, Debug, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    pub pp_elgamal: PublicParametersElgamal<E>,
    pub pk_range_proof: dekart_univariate_v2::ProverKey<E>,
    /// Base for the commitments to the polynomial evaluations (and for the dealt public key [shares])
    g_2: E::G2Affine,
    pub powers_of_radix: Vec<E::ScalarField>,
}

// impl<E: Pairing> TryFrom<&[u8]> for PublicParameters<E> {
//     type Error = CryptoMaterialError;

//     fn try_from(_bytes: &[u8]) -> Result<PublicParameters, Self::Error> {
//         todo!("Deserialization of PublicParameters from bytes not yet implemented");
//     }
// }

//use sha3::{Digest, Sha3_256};
//use crate::pvss::traits::transcript::Hashed;

// impl Hashed for PublicParameters {
//     fn hash(&self) -> &[u8; 32] {
//         &self.hash
//     }
// }

use crate::range_proofs::traits::BatchedRangeProof;
use ark_std::{
    rand::{CryptoRng, RngCore}};
use crate::constants::build_constants;
use ark_ec::hashing::HashToCurve;
use ark_ec::hashing::curve_maps::wb::WBMap;
use ark_ec::hashing::map_to_curve_hasher::MapToCurveBasedHasher;
use ark_ff::field_hashers::DefaultFieldHasher;
use ark_ec::hashing::map_to_curve_hasher::MapToCurve;
use ark_ec::hashing::curve_maps::wb::WBConfig;
use sha3::Sha3_256;

impl<E: Pairing> PublicParameters<E> 
where
    E::G2: WBConfig,
    WBMap<E::G2>: MapToCurve<E::G2>,
{
    /// Verifiably creates Aptos-specific public parameters.
    pub fn new<R: RngCore + CryptoRng>(max_num_shares: usize, radix_exponent: usize, rng: &mut R) -> Self {
        // existing initialization
        let num_chunks = max_num_shares * 255usize.div_ceil(radix_exponent);
        let num_chunks_padded = (num_chunks + 1).next_power_of_two() - 1;
        let base = E::ScalarField::from(1u64 << radix_exponent);
        let group_generators = GroupGenerators::sample(rng); // hmm at one of these should come from a powers of tau ceremony

        let hasher = MapToCurveBasedHasher::<
            E::G2,                     
            DefaultFieldHasher<Sha3_256, 128>,  // hash-to-field
            WBMap<E::G2>               // TODO: map-to-curve. might be overkill...
        >::new(build_constants::DST_PVSS_PUBLIC_PARAMS).unwrap();

        let mut pp = Self {
            pp_elgamal: PublicParametersElgamal::default(),
            pk_range_proof: dekart_univariate_v2::Proof::setup(radix_exponent, num_chunks_padded, group_generators, rng).0,
            g_2: hasher.hash(&[&build_constants::SEED_PVSS_PUBLIC_PARAMS[..], b"g_2"].concat()).expect("failed to hash to curve").into(), // TODO: Eh should do this elsewhere
            powers_of_radix: utils::powers(base, num_chunks_padded + 1), // TODO: why the +1?
            #[cfg(feature = "kangaroo")]
            table: Some(Kangaroo::<ActiveCurve>::from_preset(
                match build_constants::CHUNK_SIZE {
                    16 => Presets::Kangaroo16,
                    24 => Presets::Kangaroo24,
                    32 => Presets::Kangaroo32,
                    _ => panic!("Unsupported CHUNK_SIZE: {}", build_constants::CHUNK_SIZE),
                }
            ).unwrap()),
            // #[cfg(not(feature = "kangaroo"))]
            // table: None,
            // hash: [0u8; 32],  // placeholder
        };

        //pp.hash = pp.compute_hash();

        pp
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::new();
        // bytes.extend_from_slice(&self.pp_elgamal.to_bytes()); // has constant size
        // bytes.extend_from_slice(&self.g_2.to_compressed()); // has constant size
        // bytes.extend_from_slice(&self.pp_range_proof.to_bytes());
        // The powers of B need not be serialized, they can just be recomputed during deserialization

        bytes
    }

    // fn compute_hash(&self) -> [u8; 32] {
    //     // Custom serialization for hashing, only real change is pp_range_proof.to_bytes_for_hashing()
    //     let mut bytes = Vec::new();
    //     bytes.extend_from_slice(&self.pp_elgamal.to_bytes());
    //     bytes.extend_from_slice(&self.g_2.to_compressed());
    //     bytes.extend_from_slice(&self.pp_range_proof.to_bytes_for_hashing());

    //     let digest = Sha3_256::digest(&bytes);
    //     digest.into()
    // }

    pub fn get_commitment_base(&self) -> &E::G2Affine {
        &self.g_2
    }
}

// TODO: is this actually meaningful?
// impl ValidCryptoMaterial for PublicParameters {
//     const AIP_80_PREFIX: &'static str = "";

//     fn to_bytes(&self) -> Vec<u8> {
//         self.to_bytes()
//     }
// }

// impl Default for PublicParameters {
//     fn default() -> Self {
//         Self::new(1, build_constants::CHUNK_SIZE as usize)
//     }
// }

// impl WithMaxNumShares for PublicParameters {
//     fn with_max_num_shares(n: usize) -> Self {
//         Self::new(n, build_constants::CHUNK_SIZE as usize)
//     }
// }