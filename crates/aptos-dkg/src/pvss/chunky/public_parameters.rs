// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule implements the *public parameters* for this "chunked_elgamal_field" PVSS scheme.

use crate::{
    dlog,
    pvss::{
        chunky::{
            chunked_elgamal::num_chunks_per_scalar, chunked_elgamal_pp, input_secret::InputSecret,
            keys,
        },
        traits,
    },
    range_proofs::{dekart_univariate_v2, traits::BatchedRangeProof},
    traits::transcript::WithMaxNumShares,
};
use aptos_crypto::{
    arkworks::{
        hashing,
        serialization::{ark_de, ark_se},
        GroupGenerators,
    },
    utils, CryptoMaterialError, ValidCryptoMaterial,
};
use ark_ec::{pairing::Pairing, scalar_mul::BatchMulPreprocessing, CurveGroup};
use ark_serialize::{SerializationError, Valid};
use ark_std::log2;
use rand::{thread_rng, CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize};
use std::{collections::HashMap, ops::Mul};

const DST: &[u8] = b"APTOS_CHUNKED_ELGAMAL_FIELD_PVSS_DST"; // This DST will be used in setting up a group generator `G_2`, see below

fn compute_powers_of_radix<E: Pairing>(ell: u8) -> Vec<E::ScalarField> {
    utils::powers(
        E::ScalarField::from(1u64 << ell),
        num_chunks_per_scalar::<E::ScalarField>(ell) as usize,
    )
}

// TODO: If we need it later, let's derive CanonicalSerialize/CanonicalDeserialize from Serialize/Deserialize? E.g. the opposite of ark_se/de...
#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    pub pp_elgamal: chunked_elgamal_pp::PublicParameters<E::G1>,

    #[serde(serialize_with = "ark_se")]
    pub pk_range_proof: dekart_univariate_v2::ProverKey<E>,

    /// Base for the commitments to the polynomial evaluations (and for the dealt public key [shares])
    #[serde(serialize_with = "ark_se")]
    G_2: E::G2Affine,

    pub ell: u8,

    pub max_num_shares: u32,

    // Meaning here it seems, the max number of times `n` that transcripts can be aggregated (which means the number of contained transcripts can be `n + 1`)
    pub max_aggregation: usize,

    #[serde(skip)]
    pub dlog_table: HashMap<Vec<u8>, u64>,

    #[serde(skip)]
    pub G2_table: BatchMulPreprocessing<E::G2>,

    #[serde(skip)]
    pub powers_of_radix: Vec<E::ScalarField>,
}

impl<E: Pairing> Clone for PublicParameters<E> {
    fn clone(&self) -> Self {
        let g: E::G1 = self.pp_elgamal.G.into();
        Self {
            max_num_shares: self.max_num_shares,
            pp_elgamal: self.pp_elgamal.clone(),
            pk_range_proof: self.pk_range_proof.clone(),
            G_2: self.G_2,
            ell: self.ell,
            max_aggregation: self.max_aggregation,
            dlog_table: Self::build_dlog_table(g, self.ell, self.max_aggregation),
            G2_table: BatchMulPreprocessing::new(self.G_2.into(), self.max_num_shares as usize), // Recreate table because it doesn't allow for Copy/Clone? TODO: Fix this
            powers_of_radix: compute_powers_of_radix::<E>(self.ell),
        }
    }
}

impl<E: Pairing> PartialEq for PublicParameters<E> {
    fn eq(&self, other: &Self) -> bool {
        self.pp_elgamal == other.pp_elgamal
            && self.pk_range_proof == other.pk_range_proof
            && self.G_2 == other.G_2
            && self.ell == other.ell
            && self.max_num_shares == other.max_num_shares
            && self.max_aggregation == other.max_aggregation
        // table, G2_table, and powers_of_radix are ignored
    }
}

impl<E: Pairing> Eq for PublicParameters<E> {}

impl<E: Pairing> std::fmt::Debug for PublicParameters<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PublicParameters")
            .field("pp_elgamal", &self.pp_elgamal)
            .field("pk_range_proof", &self.pk_range_proof)
            .field("G_2", &self.G_2)
            .field("ell", &self.ell)
            .field("max_aggregation", &self.max_aggregation)
            .field("table", &"<skipped>")
            .field("G2_table", &"<skipped>")
            .field("powers_of_radix", &"<skipped>")
            .finish()
    }
}

#[allow(non_snake_case)]
impl<'de, E: Pairing> Deserialize<'de> for PublicParameters<E> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the serializable fields directly (pp_elgamal tables are skipped in wire format; we rebuild them from G, H and max_num_shares)
        #[derive(Deserialize)]
        struct PpElgamalBases<C: CurveGroup> {
            #[serde(deserialize_with = "ark_de")]
            G: C::Affine,
            #[serde(deserialize_with = "ark_de")]
            H: C::Affine,
        }
        #[derive(Deserialize)]
        struct SerializedFields<E: Pairing> {
            pp_elgamal: PpElgamalBases<E::G1>,
            #[serde(deserialize_with = "ark_de")]
            pk_range_proof: dekart_univariate_v2::ProverKey<E>,
            #[serde(deserialize_with = "ark_de")]
            G_2: E::G2Affine,
            ell: u8,
            max_num_shares: u32,
            max_aggregation: usize,
        }

        let serialized = SerializedFields::<E>::deserialize(deserializer)?;
        let G: E::G1 = serialized.pp_elgamal.G.into();
        let pp_elgamal = chunked_elgamal_pp::PublicParameters::from_bases(
            serialized.pp_elgamal.G,
            serialized.pp_elgamal.H,
            serialized.max_num_shares,
        );

        Ok(Self {
            max_num_shares: serialized.max_num_shares,
            pp_elgamal,
            pk_range_proof: serialized.pk_range_proof,
            G_2: serialized.G_2,
            ell: serialized.ell,
            max_aggregation: serialized.max_aggregation,
            dlog_table: Self::build_dlog_table(G, serialized.ell, serialized.max_aggregation),
            G2_table: BatchMulPreprocessing::new(
                serialized.G_2.into(),
                serialized.max_num_shares as usize,
            ),
            powers_of_radix: compute_powers_of_radix::<E>(serialized.ell),
        })
    }
}

impl<E: Pairing> PublicParameters<E> {
    pub fn get_commitment_base(&self) -> E::G2Affine {
        self.G_2
    }

    #[allow(non_snake_case)]
    pub(crate) fn build_dlog_table(
        G: E::G1,
        ell: u8,
        max_aggregation: usize,
    ) -> HashMap<Vec<u8>, u64> {
        dlog::table::build::<E::G1>(G, 1u64 << ((ell as u64 + log2(max_aggregation) as u64) / 2))
    }

    pub(crate) fn get_dlog_range_bound(&self) -> u64 {
        1u64 << (self.ell as u64 + log2(self.max_aggregation) as u64)
    }
}

impl<E: Pairing> Valid for PublicParameters<E> {
    fn check(&self) -> Result<(), SerializationError> {
        Ok(())
    }
}

impl<E: Pairing> traits::HasEncryptionPublicParams for PublicParameters<E> {
    type EncryptionPublicParameters = chunked_elgamal_pp::PublicParameters<E::G1>;

    fn get_encryption_public_params(&self) -> &Self::EncryptionPublicParameters {
        &self.pp_elgamal
    }
}

impl<E: Pairing> traits::Convert<keys::DealtPubKey<E>, PublicParameters<E>>
    for InputSecret<E::ScalarField>
{
    /// Computes the public key associated with the given input secret.
    /// NOTE: In the SCRAPE PVSS, a `DealtPublicKey` cannot be computed from a `DealtSecretKey` directly.
    fn to(&self, pp: &PublicParameters<E>) -> keys::DealtPubKey<E> {
        keys::DealtPubKey::new(
            pp.get_commitment_base()
                .mul(self.get_secret_a())
                .into_affine(),
        )
    }
}

#[allow(non_snake_case)]
impl<E: Pairing> TryFrom<&[u8]> for PublicParameters<E> {
    type Error = CryptoMaterialError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        bcs::from_bytes::<PublicParameters<E>>(bytes)
            .map_err(|_| CryptoMaterialError::DeserializationError)
    }
}

#[allow(non_snake_case)]
impl<E: Pairing> PublicParameters<E> {
    /// Verifiably creates Aptos-specific public parameters.
    /// If `g2` is `Some(base)`, that value is used as the commitment base (G₂); otherwise it is derived via hashing.
    pub fn new<R: RngCore + CryptoRng>(
        max_num_shares: u32,
        ell: u8,
        max_aggregation: usize,
        g2: Option<E::G2Affine>,
        rng: &mut R,
    ) -> Self {
        assert!(ell > 0, "ell must be greater than zero");

        let num_chunks = num_chunks_per_scalar::<E::ScalarField>(ell);
        let max_num_chunks_padded = max_num_shares
            .checked_mul(num_chunks)
            .and_then(|v| v.checked_add(1))
            .map(|v| (v as u64).next_power_of_two().saturating_sub(1) as u32)
            .expect("Overflow computing max_num_chunks_padded");

        let group_generators = GroupGenerators::default(); // TODO: At least one of these should come from a powers of tau ceremony?
        let pp_elgamal = chunked_elgamal_pp::PublicParameters::new(max_num_shares);
        let G = *pp_elgamal.message_base();
        let G_2 = g2.unwrap_or_else(|| hashing::unsafe_hash_to_affine(b"G_2", DST));
        let pp = Self {
            max_num_shares,
            pp_elgamal,
            pk_range_proof: dekart_univariate_v2::Proof::setup(
                max_num_chunks_padded.try_into().unwrap(),
                ell,
                group_generators,
                rng,
            )
            .0,
            G_2,
            ell,
            max_aggregation,
            dlog_table: Self::build_dlog_table(G.into(), ell, max_aggregation),
            G2_table: BatchMulPreprocessing::new(G_2.into(), max_num_shares as usize),
            powers_of_radix: compute_powers_of_radix::<E>(ell),
        };

        pp
    }

    /// Creates public parameters with a specified commitment base.
    pub fn new_with_commitment_base<R: RngCore + CryptoRng>(
        n: usize,
        ell: u8,
        max_aggregation: usize,
        commitment_base: E::G2Affine,
        rng: &mut R,
    ) -> Self {
        Self::new(
            n.try_into().unwrap(),
            ell,
            max_aggregation,
            Some(commitment_base),
            rng,
        )
    }
}

impl<E: Pairing> ValidCryptoMaterial for PublicParameters<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during PVSS transcript serialization")
    }
}

pub const DEFAULT_ELL_FOR_TESTING: u8 = 16; // TODO: made this a const to emphasize that the parameter is completely fixed wherever this value used (namely below), might not be ideal

impl<E: Pairing> Default for PublicParameters<E> {
    // This is only used for testing and benchmarking
    fn default() -> Self {
        let mut rng = thread_rng();
        Self::new(1, DEFAULT_ELL_FOR_TESTING, 1, None, &mut rng)
    }
}

impl<E: Pairing> WithMaxNumShares for PublicParameters<E> {
    fn with_max_num_shares(n: u32) -> Self {
        let mut rng = thread_rng();
        Self::new(n, DEFAULT_ELL_FOR_TESTING, 1, None, &mut rng)
    }

    fn with_max_num_shares_and_bit_size(n: u32, ell: u8) -> Self {
        let mut rng = thread_rng();
        Self::new(n, ell, 1, None, &mut rng)
    }

    // The only thing from `pp` that `generate()` uses is `pp.ell`, so make the rest as small as possible.
    fn with_max_num_shares_for_generate(_n: u32) -> Self {
        let mut rng = thread_rng();
        Self::new(1, DEFAULT_ELL_FOR_TESTING, 1, None, &mut rng)
    }
}
