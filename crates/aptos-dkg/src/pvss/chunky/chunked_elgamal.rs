// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use super::chunked_elgamal_pp::PublicParameters;
use crate::{
    dlog::bsgs,
    pvss::chunky::chunks,
    sigma_protocol,
    sigma_protocol::homomorphism::{self, fixed_base_msms, EntrywiseMap},
    Scalar,
};
use aptos_crypto::arkworks::{self, msm::MsmInput, random::sample_field_element};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Write,
};
use ark_std::fmt::Debug;
use std::{collections::HashMap, ops::Sub};

pub const DST: &[u8; 31] = b"APTOS_CHUNKED_ELGAMAL_SIGMA_DST"; // This is used for the sigma protocol Fiat-Shamir challenges

/// Formally, given:
/// - `G_1, H_1` ∈ G₁ (group generators)
/// - `ek_i` ∈ G₁ (encryption keys)
/// - `z_i,j` ∈ Scalar<E> (from plaintext scalars `z_i`, each chunked into a vector z_i,j)
/// - `r_j` ∈ Scalar<E> (randomness for `j` in a vector of chunks z_i,j)
///
/// The homomorphism maps input `[z_i,j]` and randomness `[r_j]` to
/// the following codomain elements:
///
/// ```text
/// C_i,j = G_1 * z_i,j + ek_i * r_j
/// R_j  = H_1 * r_j
/// ```
///
/// The `C_i,j` represent "chunked" homomorphic encryptions of the plaintexts,
/// and `R_j` carry the corresponding randomness contributions.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(non_snake_case)]
pub struct WeightedHomomorphism<'a, C: CurveGroup> {
    pub pp: &'a PublicParameters<C>, // These are small so no harm in copying them here
    pub eks: &'a [C::Affine],        // TODO: capitalize to EKs ?
}

// Need to manually implement `CanonicalSerialize` because `Homomorphism` has references instead of owned values
impl<'a, C: CurveGroup> CanonicalSerialize for WeightedHomomorphism<'a, C> {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.pp.G.serialize_with_mode(&mut writer, compress)?;
        self.pp.H.serialize_with_mode(&mut writer, compress)?;
        for ek in self.eks {
            ek.serialize_with_mode(&mut writer, compress)?;
        }
        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        self.pp.G.serialized_size(compress)
            + self.pp.H.serialized_size(compress)
            + self
                .eks
                .iter()
                .map(|ek| ek.serialized_size(compress))
                .sum::<usize>()
    }
}

/// This struct is used as `CodomainShape<T>`, but the same layout also applies to the `Witness` type.
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct WeightedCodomainShape<T: CanonicalSerialize + CanonicalDeserialize + Clone> {
    pub chunks: Vec<Vec<Vec<T>>>, // Depending on T these can be chunked ciphertexts, or their MSM representations
    pub randomness: Vec<Vec<T>>,  // Same story, depending on T
}

// Witness shape happens to be identical to CodomainShape, this is mostly coincidental
// Setting `type Witness = CodomainShape<Scalar<E>>` would later require deriving SigmaProtocolWitness for CodomainShape<T>
// (and would be overkill anyway), but this leads to issues as it expects `T` to be a Pairing, so we'll simply redefine it:
#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct WeightedWitness<F: PrimeField> {
    pub plaintext_chunks: Vec<Vec<Vec<Scalar<F>>>>,
    pub plaintext_randomness: Vec<Vec<Scalar<F>>>, // For at most max_weight, there needs to be a vector of randomness to encrypt a vector of chunks
}

#[allow(non_snake_case)]
impl<C: CurveGroup> homomorphism::Trait for WeightedHomomorphism<'_, C> {
    type Codomain = WeightedCodomainShape<C>;
    type CodomainNormalized = WeightedCodomainShape<C::Affine>;
    type Domain = WeightedWitness<C::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        // Get the batch multiplication tables
        let G_table = &*self.pp.G_table;
        let H_table = &*self.pp.H_table;

        // 1. Compute C_{i,j,k} = z_{i,j,k} * G + r_{j,k} * ek_i
        //    where i is player, j is share index (which corresponds to weight level), k is chunk

        // 1a. Batch multiply all z_{i,j,k} values with G
        let all_z_chunks: Vec<C::ScalarField> = input
            .plaintext_chunks
            .iter()
            .flatten()
            .flatten()
            .map(|scalar| scalar.0)
            .collect();
        let G_mults = arkworks::batch_mul(G_table, &all_z_chunks);

        // 1b. For each player i and share j, compute r_{j,k} * ek_i for all chunks k // TODO: might be worthwhile to have tables for each ek_i as well
        //     Share index j directly corresponds to the weight level index in plaintext_randomness
        let mut chunks_result = Vec::new();
        let mut G_idx = 0;

        for (player_idx, player_chunks) in input.plaintext_chunks.iter().enumerate() {
            let mut player_Cs = Vec::new();
            let ek: C = self.eks[player_idx].into();
            for (share_idx, share_chunks) in player_chunks.iter().enumerate() {
                // share_idx corresponds to weight level index in plaintext_randomness
                let r_chunks = &input.plaintext_randomness[share_idx];

                // Compute r_{j,k} * ek_i for each chunk k
                // Since we have one base (ek_i) with multiple scalars, we compute each separately

                // Combine: C_{i,j,k} = z_{i,j,k} * G + r_{j,k} * ek_i
                let mut share_Cs = Vec::new();
                for k in 0..share_chunks.len() {
                    let ek_mult = ek * r_chunks[k].0;
                    share_Cs.push(G_mults[G_idx] + ek_mult);
                    G_idx += 1;
                }
                player_Cs.push(share_Cs);
            }
            chunks_result.push(player_Cs);
        }

        // 2. Compute R_{j,k} = r_{j,k} * H for all weight levels j and chunks k
        let all_r_chunks: Vec<C::ScalarField> = input
            .plaintext_randomness
            .iter()
            .flatten()
            .map(|scalar| scalar.0)
            .collect();
        let H_mults = arkworks::batch_mul(H_table, &all_r_chunks);

        // 3. Gather R results into nested structure matching plaintext_randomness
        let mut randomness_result = Vec::new();
        let mut H_idx = 0;
        for r_row in &input.plaintext_randomness {
            let mut R_row = Vec::new();
            for _ in r_row {
                R_row.push(H_mults[H_idx]);
                H_idx += 1;
            }
            randomness_result.push(R_row);
        }

        WeightedCodomainShape {
            chunks: chunks_result,
            randomness: randomness_result,
        }
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        <WeightedHomomorphism<C> as fixed_base_msms::Trait>::normalize_output(value)
    }
}

impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> EntrywiseMap<T>
    for WeightedCodomainShape<T>
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> =
        WeightedCodomainShape<U>;

    fn map<U, F>(self, mut f: F) -> Self::Output<U>
    where
        F: FnMut(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        let chunks = self
            .chunks
            .into_iter()
            .map(|row| {
                row.into_iter()
                    .map(|inner_row| inner_row.into_iter().map(&mut f).collect::<Vec<_>>())
                    .collect::<Vec<_>>()
            })
            .collect();

        let randomness = self
            .randomness
            .into_iter()
            .map(|inner_vec| inner_vec.into_iter().map(&mut f).collect::<Vec<_>>())
            .collect();

        WeightedCodomainShape { chunks, randomness }
    }
}

impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> IntoIterator
    for WeightedCodomainShape<T>
{
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        let mut combined: Vec<T> = self.chunks.into_iter().flatten().flatten().collect();
        combined.extend(self.randomness.into_iter().flatten());
        combined.into_iter()
    }
}

// Given a chunked scalar [z_j] and vector of randomness [r_j], returns a vector of MSM terms
// of the vector C_j = z_j * G_1 + r_j * ek, so a vector with entries [(G_1, ek), (z_j, r_j)]_j
fn chunks_msm_terms<C: CurveGroup>(
    pp: &PublicParameters<C>,
    ek: C::Affine,
    chunks: &[Scalar<C::ScalarField>],
    correlated_randomness: &[Scalar<C::ScalarField>],
) -> Vec<MsmInput<C::Affine, C::ScalarField>> {
    chunks
        .iter()
        .zip(correlated_randomness.iter())
        .map(|(&z_ij, &r_j)| MsmInput {
            bases: vec![pp.G, ek],
            scalars: vec![z_ij.0, r_j.0],
        })
        .collect()
}

// Given a vector of chunked scalar [[z_j]] and vector of randomness [[r_j]], returns a vector of
// vector of MSM terms. This is used for the weighted PVSS, where each player gets a vector of chunks
pub fn chunks_vec_msm_terms<C: CurveGroup>(
    pp: &PublicParameters<C>,
    ek: C::Affine,
    chunks_vec: &[Vec<Scalar<C::ScalarField>>],
    correlated_randomness_vec: &[Vec<Scalar<C::ScalarField>>],
) -> Vec<Vec<MsmInput<C::Affine, C::ScalarField>>> {
    chunks_vec
        .iter()
        .zip(correlated_randomness_vec.iter())
        .map(|(chunks, correlated_randomness)| {
            chunks_msm_terms::<C>(pp, ek, chunks, correlated_randomness)
        })
        .collect()
}

#[allow(non_snake_case)]
impl<'a, C: CurveGroup> fixed_base_msms::Trait for WeightedHomomorphism<'a, C> {
    type Base = C::Affine;
    type CodomainShape<T>
        = WeightedCodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = C;
    type Scalar = C::ScalarField;

    fn msm_terms(
        &self,
        input: &Self::Domain,
    ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
        // C_{i,j} = z_{i,j} * G_1 + r_j * ek[i]
        let Cs = input
            .plaintext_chunks
            .iter()
            .enumerate()
            .map(|(i, z_i)| {
                // here `i` is the player's id
                chunks_vec_msm_terms::<C>(self.pp, self.eks[i], z_i, &input.plaintext_randomness)
            })
            .collect();

        // R_j = r_j * H_1
        let Rs = input
            .plaintext_randomness
            .iter()
            .map(|inner_vec| {
                inner_vec
                    .iter()
                    .map(|&r_j| MsmInput {
                        bases: vec![self.pp.H],
                        scalars: vec![r_j.0],
                    })
                    .collect()
            })
            .collect();

        WeightedCodomainShape {
            chunks: Cs,
            randomness: Rs,
        }
    }

    fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
        C::msm(input.bases(), input.scalars()).expect("MSM failed in ChunkedElgamal")
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        C::normalize_batch(&msm_output)
    }
}

impl<'a, C: CurveGroup> sigma_protocol::CurveGroupTrait for WeightedHomomorphism<'a, C> {
    type Group = C;

    fn dst(&self) -> Vec<u8> {
        let mut result = b"WEIGHTED_".to_vec();
        result.extend(DST);
        result
    }
}

pub fn correlated_randomness<F, R>(
    rng: &mut R,
    radix: u64,
    num_chunks: u32,
    target_sum: &F,
) -> Vec<F>
where
    F: PrimeField, // need `PrimeField` here because of `sample_field_element()`
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    let mut r_vals = vec![F::zero(); num_chunks as usize];
    let mut remaining = *target_sum;
    let radix_f = F::from(radix);
    let mut cur_base = radix_f;

    for i in 1..(num_chunks as usize) {
        r_vals[i] = sample_field_element(rng);
        remaining -= r_vals[i] * cur_base;
        cur_base *= radix_f;
    }
    r_vals[0] = remaining;

    r_vals
}

pub fn num_chunks_per_scalar<F: PrimeField>(ell: u8) -> u32 {
    F::MODULUS_BIT_SIZE.div_ceil(ell as u32) // Maybe add `as usize` here?
}

/// Decrypt a vector of chunked ciphertexts using the corresponding committed randomness and decryption keys
///
/// # Arguments
/// - `Cs_rows`: slice of vectors, each inner vector contains chunks for one scalar.
/// - `Rs_rows`: slice of vectors, same shape as `Cs_rows`, contains corresponding committed randomness/keys.
/// - `dk`: decryption key for the player.
/// - `pp`: public parameters (provides group generator).
/// - `table`: precomputed BSGS table for discrete log.
/// - `radix_exponent`: exponent used to split/reconstruct chunks.
///
/// # Returns
/// - Vec of decrypted scalars.
#[allow(non_snake_case)]
pub fn decrypt_chunked_scalars<C: CurveGroup>(
    Cs_rows: &[Vec<C::Affine>],
    Rs_rows: &[Vec<C::Affine>],
    dk: &C::ScalarField,
    pp: &PublicParameters<C>,
    table: &HashMap<Vec<u8>, u64>,
    table_dlog_range_bound: u64,
    radix_exponent: u8,
) -> Vec<C::ScalarField> {
    let mut decrypted_scalars = Vec::with_capacity(Cs_rows.len());

    for (row, Rs_row) in Cs_rows.iter().zip(Rs_rows.iter()) {
        // Compute C - d_k * R for each chunk
        let exp_chunks: Vec<C> = row
            .iter()
            .zip(Rs_row.iter())
            .map(|(C_ij, &R_j)| C_ij.sub(R_j * *dk))
            .collect();

        // Recover plaintext chunks
        let chunk_values: Vec<_> = bsgs::dlog_vec(
            pp.G.into_group(),
            &exp_chunks,
            &table,
            table_dlog_range_bound,
        )
        .expect("dlog_vec failed")
        .into_iter()
        .map(|x| C::ScalarField::from(x))
        .collect();

        // Convert chunks back to scalar
        let recovered = chunks::le_chunks_to_scalar(radix_exponent, &chunk_values);

        decrypted_scalars.push(recovered);
    }

    decrypted_scalars
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{dlog, pvss::chunky::chunks, sigma_protocol::homomorphism::Trait as _};
    use aptos_crypto::{
        arkworks::{random::sample_field_elements, shamir::ShamirThresholdConfig},
        weighted_config::WeightedConfig,
    };
    use ark_ec::CurveGroup;
    use rand::thread_rng;

    fn test_correlated_randomness_generic<F: PrimeField>() {
        let mut rng = thread_rng();
        let target_sum = F::one();
        let radix: u64 = 4;
        let num_chunks: u8 = 8;

        let coefs = correlated_randomness(&mut rng, radix, num_chunks as u32, &target_sum);

        // Compute actual sum: Σ coef[i] * radix^i
        let actual_sum: F = (0..num_chunks)
            .map(|i| coefs[i as usize] * F::from(radix.pow(i as u32)))
            .sum();

        assert_eq!(target_sum, actual_sum);
    }

    #[test]
    fn test_correlated_randomness_bn254() {
        use ark_bn254::Fr;
        test_correlated_randomness_generic::<Fr>();
    }

    fn prepare_chunked_witness<F: PrimeField>(
        sc: WeightedConfig<ShamirThresholdConfig<F>>,
        ell: u8,
    ) -> (Vec<F>, WeightedWitness<F>, u8, u32) {
        let mut rng = thread_rng();

        // 1. Generate random values
        let zs = sample_field_elements(sc.get_total_weight(), &mut rng);

        // 2. Compute number of chunks
        let number_of_chunks = num_chunks_per_scalar::<F>(ell);

        // 3. Generate correlated randomness
        let rs: Vec<Vec<F>> = (0..sc.get_max_weight())
            .map(|_| correlated_randomness(&mut rng, 1 << ell, number_of_chunks, &F::ZERO))
            .collect();

        // 4. Convert values into little-endian chunks
        let chunked_values: Vec<Vec<F>> = zs
            .iter()
            .map(|z| chunks::scalar_to_le_chunks(ell, z))
            .collect();

        // 5. Build witness
        let witness = WeightedWitness {
            plaintext_chunks: sc.group_by_player(&Scalar::vecvec_from_inner(chunked_values)),
            plaintext_randomness: Scalar::vecvec_from_inner(rs),
        };

        (zs, witness, ell, number_of_chunks)
    }

    #[allow(non_snake_case)]
    fn test_decrypt_roundtrip<C: CurveGroup>() {
        // 2-out-of-3, weights 2 1
        let sc =
            WeightedConfig::<ShamirThresholdConfig<C::ScalarField>>::new(2, vec![2, 1]).unwrap();

        let (zs, witness, radix_exponent, _num_chunks) =
            prepare_chunked_witness::<C::ScalarField>(sc, 16);

        // 6. Initialize the homomorphism
        let pp: PublicParameters<C> = PublicParameters::new(3);
        let dks: Vec<C::ScalarField> = sample_field_elements(2, &mut thread_rng());

        let hom = WeightedHomomorphism::<C> {
            pp: &pp,
            eks: &C::normalize_batch(&[pp.H * dks[0], pp.H * dks[1]]), // 2 players
        };

        // 7. Apply homomorphism to obtain chunked ciphertexts
        let WeightedCodomainShape::<C> {
            chunks: Cs,
            randomness: Rs,
        } = hom.apply(&witness);

        // 8. Build a baby-step giant-step table for computing discrete logs
        let table = dlog::table::build::<C>(pp.G.into(), 1u64 << (radix_exponent / 2));

        // 9. Perform decryption of each ciphertext and reconstruct plaintexts
        // TODO: call some built-in function for this instead
        let mut decrypted_scalars = Vec::new();
        for player_id in 0..Cs.len() {
            // Convert projective Cs[player_id] to affine
            let Cs_player_affine: Vec<Vec<C::Affine>> = Cs[player_id]
                .iter()
                .map(|row| {
                    let proj_vec: Vec<C> = row.to_vec();
                    C::normalize_batch(&proj_vec)
                })
                .collect();

            // Convert projective Rs to affine
            let Rs_affine: Vec<Vec<C::Affine>> = Rs
                .iter()
                .map(|row| {
                    let proj_vec: Vec<C> = row.to_vec();
                    C::normalize_batch(&proj_vec)
                })
                .collect();

            let decrypted_for_player = decrypt_chunked_scalars(
                &Cs_player_affine,
                &Rs_affine,
                &dks[player_id],
                &pp,
                &table,
                1 << radix_exponent, // we're not aggregating anything in this test. TODO: aggregating-and-decrypting should be tested somewhere in this crate, though currently it's happening in some fptx_smoke test I think
                radix_exponent,
            );

            decrypted_scalars.extend(decrypted_for_player);
        }

        // 10. Compare decrypted scalars to original plaintexts
        for (i, (orig, recovered)) in zs.iter().zip(decrypted_scalars.iter()).enumerate() {
            assert_eq!(
                orig, recovered,
                "Decrypted plaintext {} does not match original",
                i
            );
        }
    }

    #[test]
    fn test_decrypt_roundtrip_bn254() {
        test_decrypt_roundtrip::<ark_bn254::G1Projective>();
    }
}
