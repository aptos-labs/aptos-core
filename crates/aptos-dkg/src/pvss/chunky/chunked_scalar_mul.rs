// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE

use crate::{
    pvss::chunky::chunks::le_chunks_to_scalar,
    sigma_protocol,
    sigma_protocol::{
        homomorphism,
        homomorphism::{fixed_base_msms, fixed_base_msms::Trait, EntrywiseMap},
    },
    Scalar,
};
use aptos_crypto::arkworks::msm::{IsMsmInput, MsmInput};
use aptos_crypto_derive::SigmaProtocolWitness;
use ark_ec::CurveGroup;
use ark_ff::PrimeField;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use std::fmt::Debug;

pub const DST: &[u8; 34] = b"APTOS_CHUNKED_COMMIT_HOM_SIGMA_DST";

// TODO: arrange things by player... eh no did that already???
/// In this file we set up the following "commitment" homomorphism:
/// Commit to chunked scalars by unchunking them and multiplying a base group element (in affine representation)
/// with each unchunked scalar.
///
/// Equivalent to `[base * unchunk(chunk) for chunks in chunked_scalars]`.
#[derive(CanonicalSerialize, Debug, Clone, PartialEq, Eq)]
pub struct Homomorphism<C: CurveGroup> {
    pub base: C::Affine,
    pub ell: u8,
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

    fn map<U, F>(self, mut f: F) -> Self::Output<U>
    where
        F: FnMut(T) -> U,
        U: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq,
    {
        CodomainShape(
            self.0
                .into_iter()
                .map(f)
                .collect(),
        )
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

// A vector over the list of weights, and for each weight a vector of chunks
#[derive(
    SigmaProtocolWitness, CanonicalSerialize, CanonicalDeserialize, Clone, Debug, PartialEq, Eq,
)]
pub struct Witness<F: PrimeField> {
    pub chunked_values: Vec<Vec<Scalar<F>>>,
}

impl<C: CurveGroup> homomorphism::Trait for Homomorphism<C> {
    type Codomain = CodomainShape<C>;
    type Domain = Witness<C::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

impl<C: CurveGroup> fixed_base_msms::Trait for Homomorphism<C> {
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone + Debug + Eq;
    type MsmInput = MsmInput<C::Affine, C::ScalarField>;
    type MsmOutput = C;
    type Scalar = C::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {

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

    fn msm_eval(input: Self::MsmInput) -> Self::MsmOutput {
        C::msm(input.bases(), input.scalars()).expect("MSM failed in Schnorr") // TODO: custom MSM here, because only length 1 MSM except during verification
    }
}

impl<C: CurveGroup> sigma_protocol::Trait<C> for Homomorphism<C> {
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

        let hom = Homomorphism::<G1Projective> { base, ell };

        // Apply the homomorphism
        let CodomainShape(outputs) = hom.apply(&witness);

        // Check correctness:
        // base * unchunk(chunks) == output
        let mut output_iter = outputs.iter();
        for scalar_chunks in chunked_values.iter() {
                let V = output_iter.next().expect("Mismatch in output length");

                let reconstructed =
                    le_chunks_to_scalar(ell, &Scalar::slice_as_inner(scalar_chunks));

                let expected = base * reconstructed;
                assert_eq!(
                    *V, expected,
                    "Homomorphism output does not match expected base * scalar"
                );
            }
    }
}
