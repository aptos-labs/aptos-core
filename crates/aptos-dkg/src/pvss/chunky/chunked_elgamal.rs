// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sigma_protocol,
    sigma_protocol::homomorphism::{self, fixed_base_msms, fixed_base_msms::Trait, EntrywiseMap},
    Scalar,
};
use aptos_crypto::arkworks::{hashing, random::sample_field_element};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Write,
};
use ark_std::fmt::Debug;

pub const DST: &[u8; 35] = b"APTOS_CHUNKED_ELGAMAL_GENERATOR_DST"; // This is used to create public parameters, see `default()` below

// TODO: Change this to PublicParameters<E: CurveGroup>. Would first require changing Scalar<E: Pairing> to Scalar<F: PrimeField>, which would be a bit of work
#[derive(CanonicalSerialize, CanonicalDeserialize, PartialEq, Clone, Eq, Debug)]
#[allow(non_snake_case)]
pub struct PublicParameters<E: Pairing> {
    /// A group element $G$ that is raised to the encrypted message
    pub G: E::G1Affine,
    /// A group element $H$ that is used to exponentiate both
    /// (1) the ciphertext randomness and (2) the DSK when computing its EK.
    pub H: E::G1Affine,
}

#[allow(non_snake_case)]
impl<E: Pairing> PublicParameters<E> {
    pub fn new(G: E::G1Affine, H: E::G1Affine) -> Self {
        Self { G, H }
    }

    pub fn message_base(&self) -> &E::G1Affine {
        &self.G
    }

    pub fn pubkey_base(&self) -> &E::G1Affine {
        &self.H
    }

    pub fn default() -> Self {
        let G = hashing::unsafe_hash_to_affine(b"G", DST);
        let H = hashing::unsafe_hash_to_affine(b"H", DST);
        debug_assert_ne!(G, H);
        Self { G, H }
    }
}

/// Formally, given:
/// - `G_1, H_1` ∈ G₁ (group generators)
/// - `ek_i` ∈ G₁ (encryption keys)
/// - `z_i,j` ∈ Scalar<E> (plaintext scalars z_i, chunked into z_i,j)
/// - `r_j` ∈ Scalar<E> (randomness for each `column` of chunks z_i,j)
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
pub struct Homomorphism<'a, E: Pairing> {
    pub pp: &'a PublicParameters<E>, // This is small so could clone it here, then no custom `CanonicalSerialize` is needed
    pub eks: &'a [E::G1Affine],
}

// Need to manually implement `CanonicalSerialize` because `Homomorphism` has references instead of owned values
impl<'a, E: Pairing> CanonicalSerialize for Homomorphism<'a, E> {
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
pub struct CodomainShape<T: CanonicalSerialize + CanonicalDeserialize + Clone> {
    pub chunks: Vec<Vec<T>>, // Depending on T these can be chunked ciphertexts, or their MSM representations
    pub randomness: Vec<T>,  // Same story, depending on T
}

// Witness shape happens to be identical to CodomainShape, this is mostly coincidental
// Setting `type Witness = CodomainShape<Scalar<E>>` would later require deriving SigmaProtocolWitness for CodomainShape<T>
// (and would be overkill anyway), but this leads to issues as it expects T to be a Pairing, so we'll simply redefine it:
#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Witness<E: Pairing> {
    pub plaintext_chunks: Vec<Vec<Scalar<E>>>,
    pub plaintext_randomness: Vec<Scalar<E>>,
}

impl<E: Pairing> homomorphism::Trait for Homomorphism<'_, E> {
    type Codomain = CodomainShape<E::G1>;
    type Domain = Witness<E>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

// TODO: Can problably do EntrywiseMap with another derive macro
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> EntrywiseMap<T>
    for CodomainShape<T>
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> =
        CodomainShape<U>;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: Fn(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        let chunks = self
            .chunks
            .into_iter()
            .map(|row| row.into_iter().map(&f).collect())
            .collect();

        let randomness = self.randomness.into_iter().map(f).collect();

        CodomainShape { chunks, randomness }
    }
}

// TODO: Use a derive macro?
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> IntoIterator for CodomainShape<T> {
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        let mut combined: Vec<T> = self.chunks.into_iter().flatten().collect(); // Temporary Vec can probably be avoided, but might require unstable Rust or a lot of lines
        combined.extend(self.randomness);
        combined.into_iter()
    }
}

#[allow(non_snake_case)]
impl<'a, E: Pairing> fixed_base_msms::Trait for Homomorphism<'a, E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        // C_{i,j} = z_{i,j} * G_1 + r_j * ek[i]
        let Cs = input
            .plaintext_chunks
            .iter()
            .enumerate()
            .map(|(i, z_i)| {
                z_i.iter()
                    .zip(input.plaintext_randomness.iter())
                    .map(|(&z_ij, &r_j)| fixed_base_msms::MsmInput {
                        bases: vec![self.pp.G, self.eks[i]],
                        scalars: vec![z_ij.0, r_j.0],
                    })
                    .collect()
            })
            .collect();

        // R_j = r_j * H_1
        let Rs = input
            .plaintext_randomness
            .iter()
            .map(|&r_j| fixed_base_msms::MsmInput {
                bases: vec![self.pp.H],
                scalars: vec![r_j.0],
            })
            .collect();

        CodomainShape {
            chunks: Cs,
            randomness: Rs,
        }
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, scalars).expect("MSM failed in ChunkedElgamal")
    }
}

impl<'a, E: Pairing> sigma_protocol::Trait<E> for Homomorphism<'a, E> {
    fn dst(&self) -> Vec<u8> {
        b"APTOS_CHUNKED_ELGAMAL_SIGMA_PROTOCOL_DST".to_vec()
    }
}

pub(crate) fn correlated_randomness<F, R>(rng: &mut R, radix: u64, num_chunks: usize) -> Vec<F>
where
    F: ark_ff::PrimeField,
    R: rand_core::RngCore + rand_core::CryptoRng,
{
    let mut r_vals = Vec::with_capacity(num_chunks);
    r_vals.push(F::zero()); // placeholder for r_0
    let mut remainder = F::zero();

    // Precompute radix as F once
    let radix_f = F::from(radix);
    let mut cur_base = radix_f;

    // Fill r_1 .. r_{num_chunks-1} randomly
    for _ in 1..num_chunks {
        let r = sample_field_element(rng);
        r_vals.push(r);
        remainder -= r * cur_base;
        cur_base *= radix_f;
    }

    r_vals[0] = remainder;

    r_vals
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{pvss::chunky::chunks, sigma_protocol::homomorphism::Trait as _};
    use aptos_crypto::arkworks::random::{sample_field_elements, unsafe_random_points};
    use ark_ec::CurveGroup;
    use ark_ff::{Field, PrimeField};
    use rand::thread_rng;

    #[allow(non_snake_case)]
    fn test_reconstruct_ciphertexts<E: Pairing>() {
        let mut rng = thread_rng();

        // 1. Generate two random "shares"
        let zs = sample_field_elements(2, &mut rng);

        // 2. Choose a radix and compute number_of_chunks
        let radix_exponent = 16; // Making this smaller would probably make the test slower
        let number_of_chunks = E::ScalarField::MODULUS_BIT_SIZE.div_ceil(radix_exponent) as usize;

        // 3. Generate correlated randomness
        let rs: Vec<E::ScalarField> =
            correlated_randomness(&mut rng, 1 << radix_exponent, number_of_chunks);

        // 4. Convert the two values into little-endian chunks
        let chunked_values: Vec<Vec<E::ScalarField>> = zs
            .iter()
            .map(|v| chunks::scalar_to_le_chunks(radix_exponent as usize, v))
            .collect();

        // 5. Build a witness for the homomorphism
        let witness = Witness {
            plaintext_chunks: Scalar::<E>::vecvec_from_inner(chunked_values),
            plaintext_randomness: Scalar::vec_from_inner(rs),
        };

        // 6. Initialize a homomorphism (mock params or reuse pp, eks as needed)
        let eks_inner: Vec<_> = E::G1::normalize_batch(&unsafe_random_points(2, &mut rng));
        let pp = PublicParameters::default();

        let hom = Homomorphism {
            pp: &pp,
            eks: &eks_inner,
        };

        // 7. Apply homomorphism to obtain chunked ciphertexts
        let CodomainShape {
            chunks: Cs,
            randomness: _Rs,
        } = hom.apply(&witness);

        // 8. Reconstruct original values from the chunked ciphertexts
        for (i, &orig_val) in zs.iter().enumerate() {
            // compute powers of the radix for this chunk vector safely in the field
            let radix_f = E::ScalarField::from(1u64 << radix_exponent); // radix as field element
            let mut cur_power = E::ScalarField::ONE;
            let powers_of_radix: Vec<E::ScalarField> = (0..Cs[i].len())
                .map(|_| {
                    let p = cur_power;
                    cur_power *= radix_f;
                    p
                })
                .collect();

            // perform the MSM to reconstruct the "plaintext group element"
            let reconstructed = E::G1::msm(&E::G1::normalize_batch(&Cs[i]), &powers_of_radix)
                .expect("MSM reconstruction failed");

            // multiply the original scalar by the group generator (or message base)
            let expected = *pp.message_base() * orig_val;

            assert_eq!(
                reconstructed, expected,
                "Reconstructed value {} does not match original",
                i
            );
        }
    }

    #[test]
    fn test_reconstruct_ciphertexts_bn254() {
        test_reconstruct_ciphertexts::<ark_bn254::Bn254>();
    }
}
