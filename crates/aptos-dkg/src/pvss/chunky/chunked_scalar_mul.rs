// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pvss::chunky::chunks::le_chunks_to_scalar,
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, EntrywiseMap},
    },
    Scalar,
};
use aptos_crypto::arkworks::{self, msm::MsmInput};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::{scalar_mul::BatchMulPreprocessing, CurveGroup};
use ark_ff::PrimeField;
use ark_serialize::{
    CanonicalDeserialize, CanonicalSerialize, Compress, SerializationError, Write,
};
use std::fmt::{Debug, Formatter, Result as FmtResult};

pub const DST: &[u8; 34] = b"APTOS_CHUNKED_COMMIT_HOM_SIGMA_DST";

/// In this file we set up the following "commitment" homomorphism:
/// Commit to scalars, which are input as chunks, by unchunking them
/// and multiplying a base group element (in affine representation)
/// with each unchunked scalar.
///
/// Equivalent to `[base * unchunk(chunks) for chunks in chunked_scalars]`.
///
/// This is the only file where we decided against organising things by player, but
/// went with a more flat approach. (Older player version is in the repo.) Doesn't make
/// much of a difference, just slightly less nesting...
pub struct Homomorphism<'a, C: CurveGroup> {
    pub base: C::Affine,
    pub table: &'a BatchMulPreprocessing<C>, // TODO: use Arc instead?
    pub ell: u8,
}

impl<'a, C: CurveGroup> Clone for Homomorphism<'a, C> {
    fn clone(&self) -> Self {
        Self {
            base: self.base.clone(),
            table: self.table, // Just copy the reference
            ell: self.ell,
        }
    }
}

impl<'a, C: CurveGroup> PartialEq for Homomorphism<'a, C> {
    fn eq(&self, other: &Self) -> bool {
        self.base == other.base && self.ell == other.ell
        // table is ignored
    }
}

impl<'a, C: CurveGroup> Eq for Homomorphism<'a, C> {}

impl<'a, C: CurveGroup> Debug for Homomorphism<'a, C> {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        f.debug_struct("Homomorphism")
            .field("base", &self.base)
            .field("ell", &self.ell)
            .field("table", &"<skipped>")
            .finish()
    }
}

impl<'a, C: CurveGroup> CanonicalSerialize for Homomorphism<'a, C> {
    fn serialize_with_mode<W: Write>(
        &self,
        mut writer: W,
        compress: Compress,
    ) -> Result<(), SerializationError> {
        self.base.serialize_with_mode(&mut writer, compress)?;
        self.ell.serialize_with_mode(&mut writer, compress)?;
        Ok(())
    }

    fn serialized_size(&self, compress: Compress) -> usize {
        self.base.serialized_size(compress) + self.ell.serialized_size(compress)
    }
}

// pub type CodomainShape<T> = VectorShape<T>;
#[derive(CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq)]
pub struct CodomainShape<T: CanonicalSerialize + CanonicalDeserialize + Clone>(pub Vec<T>);

impl<T> EntrywiseMap<T> for CodomainShape<T>
where
    T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
{
    type Output<U>
        = CodomainShape<U>
    where
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;

    fn map<U, F>(self, f: F) -> Self::Output<U>
    where
        F: FnMut(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        CodomainShape(self.0.into_iter().map(f).collect())
    }
}

impl<T> IntoIterator for CodomainShape<T>
where
    T: CanonicalSerialize + CanonicalDeserialize + Clone,
{
    type IntoIter = std::vec::IntoIter<T>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

// A vector over the list of scalars, and for each scalar a vector of chunks
#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Witness<F: PrimeField> {
    pub chunked_values: Vec<Vec<Scalar<F>>>,
}

impl<'a, C: CurveGroup> homomorphism::Trait for Homomorphism<'a, C> {
    type Codomain = CodomainShape<C>;
    type CodomainNormalized = CodomainShape<C::Affine>;
    type Domain = Witness<C::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        // Convert each chunked value to a scalar entrywise
        let scalars: Vec<C::ScalarField> = input
            .chunked_values
            .iter()
            .map(|chunks| le_chunks_to_scalar(self.ell, &Scalar::slice_as_inner(chunks)))
            .collect();

        // Batch multiply using the base element
        let outputs = arkworks::batch_mul(&self.table, &scalars);

        CodomainShape(outputs)
    }

    fn normalize(&self, value: Self::Codomain) -> Self::CodomainNormalized {
        <Homomorphism<C> as fixed_base_msms::Trait>::normalize_output(value)
    }
}

impl<'a, C: CurveGroup> fixed_base_msms::Trait for Homomorphism<'a, C> {
    type Base = C::Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmOutput = C;
    type Scalar = C::ScalarField;

    fn msm_terms(
        &self,
        input: &Self::Domain,
    ) -> Self::CodomainShape<MsmInput<Self::Base, Self::Scalar>> {
        let mut terms = Vec::new();

        for chunks in &input.chunked_values {
            terms.push(MsmInput {
                bases: vec![self.base.clone()],
                scalars: vec![le_chunks_to_scalar(
                    self.ell,
                    &Scalar::slice_as_inner(chunks),
                )],
            });
        }

        CodomainShape(terms)
    }

    fn msm_eval(input: MsmInput<Self::Base, Self::Scalar>) -> Self::MsmOutput {
        C::msm(input.bases(), input.scalars()).expect("MSM failed in Schnorr") // TODO: custom MSM here, because only length 1 MSM except during verification
    }

    fn batch_normalize(msm_output: Vec<Self::MsmOutput>) -> Vec<Self::Base> {
        C::normalize_batch(&msm_output)
    }
}

impl<'a, C: CurveGroup> sigma_protocol::CurveGroupTrait for Homomorphism<'a, C> {
    type Group = C;

    fn dst(&self) -> Vec<u8> {
        DST.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        pvss::chunky::chunks::{le_chunks_to_scalar, scalar_to_le_chunks},
        sigma_protocol::homomorphism::Trait as _,
    };
    use aptos_crypto::arkworks::random::{sample_field_elements, unsafe_random_point};
    use ark_bls12_381::G1Projective;
    use rand::thread_rng;

    #[test]
    #[allow(non_snake_case)]
    fn test_chunked_homomorphism_ell_16() {
        let mut rng = thread_rng();

        // Parameters
        let ell: u8 = 16;
        let num_scalars = 8;

        // Random base
        let base = unsafe_random_point::<G1Projective, _>(&mut rng);

        // Create random scalars
        let scalars = sample_field_elements(num_scalars, &mut rng);

        // Chunk each scalar into little-endian chunks of size `ell`
        let chunked_values: Vec<Vec<Scalar<_>>> = scalars
            .iter()
            .map(|s| {
                scalar_to_le_chunks(ell, s)
                    .into_iter()
                    .map(|chunk| Scalar(chunk))
                    .collect::<Vec<_>>()
            })
            .collect();

        let witness = Witness {
            chunked_values: chunked_values.clone(),
        };

        // Create table from projective base (same pattern as chunked_elgamal_pp.rs)
        let table = BatchMulPreprocessing::new(base.into(), num_scalars);
        let hom = Homomorphism::<G1Projective> {
            base,
            table: &table,
            ell,
        };

        // Apply the homomorphism
        let CodomainShape(outputs) = hom.apply(&witness);

        // Check correctness:
        // base * unchunk(chunks) == output
        let mut output_iter = outputs.iter();
        for scalar_chunks in chunked_values.iter() {
            let V = output_iter.next().expect("Mismatch in output length");

            let reconstructed = le_chunks_to_scalar(ell, &Scalar::slice_as_inner(scalar_chunks));

            let expected = base * reconstructed;
            assert_eq!(
                *V, expected,
                "Homomorphism output does not match expected base * scalar"
            );
        }
    }
}
