// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

//! This submodule implements the *public parameters* for this "chunked_elgamal_field" PVSS scheme.

use crate::{
    dlog::BabyStepTable,
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
use std::ops::Mul;

pub const DEFAULT_ELL_FOR_TESTING: u8 = 16; // TODO: made this a const to emphasize that the parameter is completely fixed wherever this value used (namely below), might not be ideal
pub const DEFAULT_ELL_FOR_DEPLOYMENT: u8 = 32; // TODO: made this a const to emphasize that the parameter is completely fixed wherever this value used (namely below), might not be ideal
const DEFAULT_MAX_AGGREGATION: usize = 166;
const DLOG_EXTRA_BITS: u8 = 4;

/// Default extra bits for the dlog table when deserializing legacy PublicParameters that did not store this field.
fn default_dlog_extra_bits() -> u8 {
    DLOG_EXTRA_BITS
}

fn compute_powers_of_radix<E: Pairing>(ell: u8) -> Vec<E::ScalarField> {
    utils::powers(
        E::ScalarField::from(1u64 << ell),
        num_chunks_per_scalar::<E::ScalarField>(ell),
    )
}

#[derive(Serialize)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    pub pp_elgamal: chunked_elgamal_pp::PublicParameters<E::G1>,

    #[serde(serialize_with = "ark_se")]
    pub pk_range_proof: dekart_univariate_v2::ProverKey<E>,

    /// Base for the commitments to the polynomial evaluations (and for the dealt public key [shares])
    #[serde(serialize_with = "ark_se")]
    G_2: E::G2Affine,

    pub ell: u8, // Should be below 64 to prevent overflows etc

    pub max_num_shares: usize,

    // Max number of transcripts that can be aggregated. Used to determine the BSGS dlog table
    // size, since aggregation doubles the max possible exponent size that needs to be decrypted.
    pub max_aggregation: usize,

    /// Extra bits for the dlog baby-step table size. Must be serialized so clone/deserialize rebuild the same table.
    /// TODO(Rex): Check if we actually still need this.
    #[serde(default = "default_dlog_extra_bits")]
    pub dlog_extra_bits: u8,

    pub dlog_table: BabyStepTable<E::G1Affine>,

    #[serde(skip)]
    pub G2_table: BatchMulPreprocessing<E::G2>,

    #[serde(skip)]
    pub powers_of_radix: Vec<E::ScalarField>,
}

impl<E: Pairing> Clone for PublicParameters<E> {
    fn clone(&self) -> Self {
        Self {
            max_num_shares: self.max_num_shares,
            pp_elgamal: self.pp_elgamal.clone(),
            pk_range_proof: self.pk_range_proof.clone(),
            G_2: self.G_2,
            ell: self.ell,
            max_aggregation: self.max_aggregation,
            dlog_extra_bits: self.dlog_extra_bits,
            dlog_table: self.dlog_table.clone(),
            G2_table: BatchMulPreprocessing::new(self.G_2.into(), self.max_num_shares), // Recreate table because it doesn't allow for Copy/Clone? TODO: Fix this
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
            && self.dlog_extra_bits == other.dlog_extra_bits
            && self.dlog_table == other.dlog_table
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
            .field("dlog_extra_bits", &self.dlog_extra_bits)
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
            max_num_shares: usize,
            max_aggregation: usize,
            #[serde(default = "default_dlog_extra_bits")]
            dlog_extra_bits: u8,
            dlog_table: BabyStepTable<E::G1Affine>,
        }

        let serialized = SerializedFields::<E>::deserialize(deserializer)?;
        let pp_elgamal = chunked_elgamal_pp::PublicParameters::from_bases(
            serialized.pp_elgamal.G,
            serialized.pp_elgamal.H,
            serialized
                .max_num_shares
                .try_into()
                .expect("Should always fit in u32"),
        );

        Ok(Self {
            max_num_shares: serialized.max_num_shares,
            pp_elgamal,
            pk_range_proof: serialized.pk_range_proof,
            G_2: serialized.G_2,
            ell: serialized.ell,
            max_aggregation: serialized.max_aggregation,
            dlog_extra_bits: serialized.dlog_extra_bits,
            G2_table: BatchMulPreprocessing::new(serialized.G_2.into(), serialized.max_num_shares),
            powers_of_radix: compute_powers_of_radix::<E>(serialized.ell),
            dlog_table: serialized.dlog_table,
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
        extra_bits: u8,
    ) -> BabyStepTable<E::G1Affine> {
        let table_size_exp: u8 = extra_bits + ((ell + log2(max_aggregation) as u8) / 2); // TODO: I think we need the floor of log_2 here, not the ceiling?
        eprintln!(
            "[build_dlog_table] table_size = {} (ell={}, max_aggregation={}, extra_bits={})",
            table_size_exp, ell, max_aggregation, extra_bits
        );
        let tbl = BabyStepTable::new(G.into_affine(), 1u32 << table_size_exp);
        eprintln!("[build_dlog_table] table_size = {}", tbl.table_size);
        tbl
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
    /// Creates public parameters for chunky, with the underlying DeKart prover key generated
    /// insecurely. Should be used only for testing.
    pub fn new_for_testing<R: RngCore + CryptoRng>(
        max_num_shares: usize,
        ell: u8,
        max_aggregation: usize,
        commitment_base: E::G2Affine,
        rng: &mut R,
    ) -> Self {
        Self::new_internal(
            max_num_shares,
            ell,
            max_aggregation,
            commitment_base,
            None,
            rng,
        )
    }

    /// Creates public parameters for chunky, with the provided DeKart prover key.
    pub fn new<R: RngCore + CryptoRng>(
        max_num_shares: usize,
        ell: u8,
        max_aggregation: usize,
        commitment_base: E::G2Affine,
        dekart_prover_key: dekart_univariate_v2::ProverKey<E>,
        rng: &mut R,
    ) -> Self {
        Self::new_internal(
            max_num_shares,
            ell,
            max_aggregation,
            commitment_base,
            Some(dekart_prover_key),
            rng,
        )
    }

    fn new_internal<R: RngCore + CryptoRng>(
        max_num_shares: usize,
        ell: u8,
        max_aggregation: usize,
        commitment_base: E::G2Affine,
        maybe_dekart_prover_key: Option<dekart_univariate_v2::ProverKey<E>>,
        rng: &mut R,
    ) -> Self {
        assert!(ell > 0, "ell must be greater than zero");

        let num_chunks = num_chunks_per_scalar::<E::ScalarField>(ell);
        let max_num_chunks_padded = max_num_shares
            .checked_mul(num_chunks)
            .and_then(|v| v.checked_add(1))
            .map(|v| (v as u64).next_power_of_two().saturating_sub(1) as u32)
            .expect("Overflow computing max_num_chunks_padded");

        let group_generators = GroupGenerators::default();
        let pp_elgamal = chunked_elgamal_pp::PublicParameters::new(
            max_num_shares
                .try_into()
                .expect("should always fit into u32"),
        );
        let G_1 = *pp_elgamal.message_base();
        let pk_range_proof = maybe_dekart_prover_key.unwrap_or_else(|| {
            dekart_univariate_v2::Proof::setup(
                max_num_chunks_padded.try_into().unwrap(),
                ell,
                group_generators,
                rng,
            )
            .0
        });

        let pp = Self {
            max_num_shares,
            pp_elgamal,
            pk_range_proof,
            G_2: commitment_base,
            ell,
            max_aggregation,
            dlog_extra_bits: DLOG_EXTRA_BITS,
            dlog_table: Self::build_dlog_table(G_1.into(), ell, max_aggregation, DLOG_EXTRA_BITS),
            G2_table: BatchMulPreprocessing::new(commitment_base.into(), max_num_shares),
            powers_of_radix: compute_powers_of_radix::<E>(ell),
        };

        pp
    }
}

impl<E: Pairing> ValidCryptoMaterial for PublicParameters<E> {
    const AIP_80_PREFIX: &'static str = "";

    fn to_bytes(&self) -> Vec<u8> {
        bcs::to_bytes(&self).expect("unexpected error during PVSS transcript serialization")
    }
}

impl<E: Pairing> Default for PublicParameters<E> {
    // This is only used for testing and benchmarking
    fn default() -> Self {
        use ark_ec::AffineRepr;
        let mut rng = thread_rng();
        Self::new_for_testing(
            1,
            DEFAULT_ELL_FOR_TESTING,
            DEFAULT_MAX_AGGREGATION,
            E::G2Affine::generator(),
            &mut rng,
        )
    }
}

impl<E: Pairing> WithMaxNumShares for PublicParameters<E> {
    fn with_max_num_shares(n: usize) -> Self {
        use ark_ec::AffineRepr;
        let mut rng = thread_rng();
        Self::new_for_testing(
            n,
            DEFAULT_ELL_FOR_TESTING,
            DEFAULT_MAX_AGGREGATION,
            E::G2Affine::generator(),
            &mut rng,
        )
    }

    fn with_max_num_shares_and_bit_size(n: usize, ell: u8) -> Self {
        use ark_ec::AffineRepr;
        let mut rng = thread_rng();
        Self::new_for_testing(
            n,
            ell,
            DEFAULT_MAX_AGGREGATION,
            E::G2Affine::generator(),
            &mut rng,
        )
    }

    // The only thing from `pp` that `generate()` uses is `pp.ell`, so make the rest as small as possible.
    fn with_max_num_shares_for_generate(_n: usize) -> Self {
        use ark_ec::AffineRepr;
        let mut rng = thread_rng();
        Self::new_for_testing(
            1,
            DEFAULT_ELL_FOR_TESTING,
            1,
            E::G2Affine::generator(),
            &mut rng,
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::pvss::chunky::PublicParameters;
    use ark_bls12_381::G2Affine;
    use ark_ec::AffineRepr;
    use rand::thread_rng;
    use std::time::Instant;

    #[test]
    #[ignore]
    fn test_realistic_serialize_deserialize() {
        let mut rng = thread_rng();

        let start = Instant::now();
        println!("{}: Generating pp", chrono::Local::now());
        let pp: PublicParameters<ark_bls12_381::Bls12_381> =
            PublicParameters::new_for_testing(256, 32, 256, G2Affine::generator(), &mut rng);
        println!(
            "{}: time taken: {:?}",
            chrono::Local::now(),
            start.elapsed()
        );

        let start = Instant::now();
        println!("{}: Serializing pp", chrono::Local::now());
        let bytes = bcs::to_bytes(&pp).unwrap();
        println!(
            "{}: time taken: {:?}",
            chrono::Local::now(),
            start.elapsed()
        );
        println!(
            "{}: pp serialized size: {} MB",
            chrono::Local::now(),
            bytes.len() / 1000 / 1000
        );

        let start = Instant::now();
        println!("{}: Deserializing pp", chrono::Local::now());
        let pp_deserialized: PublicParameters<ark_bls12_381::Bls12_381> =
            bcs::from_bytes(&bytes).unwrap();
        println!(
            "{}: time taken: {:?}",
            chrono::Local::now(),
            start.elapsed()
        );

        assert_eq!(pp, pp_deserialized);
    }

    #[test]
    fn test_serialize_deserialize() {
        let pp: PublicParameters<ark_bls12_381::Bls12_381> = PublicParameters::default();

        let bytes = bcs::to_bytes(&pp).unwrap();
        let pp_deserialized: PublicParameters<ark_bls12_381::Bls12_381> =
            bcs::from_bytes(&bytes).unwrap();

        assert_eq!(pp, pp_deserialized);
    }
}
