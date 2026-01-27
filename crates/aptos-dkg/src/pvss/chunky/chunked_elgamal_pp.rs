// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use aptos_crypto::arkworks::{
    hashing,
    serialization::{ark_de, ark_se},
};
use ark_ec::{scalar_mul::BatchMulPreprocessing, AffineRepr, CurveGroup};
use serde::{Deserialize, Serialize};

pub const DST: &[u8; 35] = b"APTOS_CHUNKED_ELGAMAL_GENERATOR_DST"; // This is used to create public parameters, see `default()` below

#[allow(non_snake_case)]
#[derive(Serialize, Deserialize)]
pub struct PublicParameters<C: CurveGroup> {
    /// A group element $G$ that is raised to the encrypted message
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub G: C::Affine,
    /// A group element $H$ that is used to exponentiate both
    /// (1) the ciphertext randomness and (2) the DSK when computing its EK.
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub H: C::Affine,
    /// Tables for G
    #[serde(skip)] // TODO: not sure skipping this in deserialization is ideal
    pub G_table: Option<BatchMulPreprocessing<C>>,
    /// Tables for H
    #[serde(skip)] // TODO: not sure skipping this in deserialization is ideal
    pub H_table: Option<BatchMulPreprocessing<C>>,
}

// ------------------ PartialEq & Eq ------------------

impl<C: CurveGroup> PartialEq for PublicParameters<C> {
    fn eq(&self, other: &Self) -> bool {
        self.G == other.G && self.H == other.H
        // G_table and H_table are ignored
    }
}

impl<C: CurveGroup> Eq for PublicParameters<C> {}

// ------------------ Clone ------------------

impl<C: CurveGroup> Clone for PublicParameters<C> {
    fn clone(&self) -> Self {
        Self {
            G: self.G.clone(),
            H: self.H.clone(),
            G_table: None, // skip cloning the precomputed table
            H_table: None, // skip cloning the precomputed table
        }
    }
}

// ------------------ Debug ------------------

impl<C: CurveGroup> std::fmt::Debug for PublicParameters<C> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicParameters")
            .field("G", &self.G)
            .field("H", &self.H)
            .field("G_table", &"<skipped>") // it's intentionally ignored
            .field("H_table", &"<skipped>") // it's intentionally ignored
            .finish()
    }
}

#[allow(non_snake_case)]
impl<C: CurveGroup> PublicParameters<C> {
    pub fn new(approximate_num_shares: u32) -> Self {
        let (G, H) = Self::default_parameters();
        let G_table = Some(BatchMulPreprocessing::new(
            G.into(),
            approximate_num_shares.try_into().unwrap(),
        ));
        let H_table = Some(BatchMulPreprocessing::new(
            H.into(),
            approximate_num_shares.try_into().unwrap(),
        ));
        Self {
            G,
            H,
            G_table,
            H_table,
        }
    }

    fn default_parameters() -> (C::Affine, C::Affine) {
        let G = hashing::unsafe_hash_to_affine(b"G", DST);
        // Chunky's encryption pubkey base must match up with the blst base, since validators
        // reuse their consensus keypairs as encryption keypairs
        let H = C::Affine::generator();
        debug_assert_ne!(G, H);
        (G, H)
    }

    pub fn message_base(&self) -> &C::Affine {
        &self.G
    }

    pub fn pubkey_base(&self) -> &C::Affine {
        &self.H
    }
}
