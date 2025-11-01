// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::{
    sigma_protocol::homomorphism::{self, fixed_base_msms, fixed_base_msms::Trait, EntrywiseMap},
    Scalar,
};
use aptos_crypto::arkworks::hashing;
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Write,
};
use ark_std::fmt::Debug;

pub const DST: &[u8; 35] = b"APTOS_CHUNKED_ELGAMAL_GENERATOR_DST";

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
    pub pp: &'a PublicParameters<E>, // This is small so could clone it here, then no custom `CanonicalSerialize` needed
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
/// Hence, for brevity, we reuse this struct for both purposes.
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct ChunksAndRandomness<T: CanonicalSerialize + CanonicalDeserialize + Clone> {
    pub chunks: Vec<Vec<T>>, // Depending on T these can be chunked ciphertexts, or their MSM representations, but also chunked plaintexts (see Witness below)
    pub randomness: Vec<T>,  // Same story, depending on T
}

// Witness shape happens to be identical to CodomainShape, this is mostly coincidental; hence for brevity:
pub type Witness<E> = ChunksAndRandomness<Scalar<E>>;

impl<E: Pairing> homomorphism::Trait for Homomorphism<'_, E> {
    type Codomain = ChunksAndRandomness<E::G1>;
    type Domain = Witness<E>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

// TODO: Can problably do EntrywiseMap with another derive macro
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> EntrywiseMap<T>
    for ChunksAndRandomness<T>
{
    type Output<U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq> =
        ChunksAndRandomness<U>;

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

        ChunksAndRandomness { chunks, randomness }
    }
}

// TODO: Use a derive macro?
impl<T: CanonicalSerialize + CanonicalDeserialize + Clone> IntoIterator for ChunksAndRandomness<T> {
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
        = ChunksAndRandomness<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = fixed_base_msms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = Scalar<E>;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        // C_i,j = G_1 * z_i,j + ek[i] * r_j
        let Cs = input
            .chunks
            .iter()
            .enumerate()
            .map(|(i, z_i)| {
                z_i.iter()
                    .zip(input.randomness.iter())
                    .map(|(&z_ij, &r_j)| fixed_base_msms::MsmInput {
                        bases: vec![self.pp.G, self.eks[i]],
                        scalars: vec![z_ij, r_j],
                    })
                    .collect()
            })
            .collect();

        //  R_j = H_1 * r_j
        let Rs = input
            .randomness
            .iter()
            .map(|&r_j| fixed_base_msms::MsmInput {
                bases: vec![self.pp.H],
                scalars: vec![r_j],
            })
            .collect();

        ChunksAndRandomness {
            chunks: Cs,
            randomness: Rs,
        }
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, &Scalar::slice_as_inner(scalars)).expect("MSM failed in ChunkedElGamal")
    }
}
