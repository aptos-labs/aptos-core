// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::algebra::homomorphism::{self, FixedBaseMSM};
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};

#[allow(non_snake_case)]
pub struct Map<'a, E: Pairing> {
    pub g_1: &'a E::G1Affine,
    pub h_1: &'a E::G1Affine,
    pub ek: &'a [E::G1Affine],
}

type MatrixVectorPair<T> = (Vec<Vec<T>>, Vec<T>);
type MSMInputVec<'a, E> = MatrixVectorPair<(
    Vec<<Map<'a, E> as FixedBaseMSM>::Base>,
    Vec<<Map<'a, E> as FixedBaseMSM>::Scalar>,
)>;

impl<'a, E: Pairing> Map<'a, E> {
    pub fn prep_msms(&self, input: &<Self as homomorphism::Map>::Domain) -> MSMInputVec<'a, E> {
        let first_elgamal_comp = input
            .0
            .iter()
            .enumerate()
            .map(|(i, row)| {
                row.iter()
                    .zip(input.1.iter())
                    .map(|(&z_ij, &r_j)| (vec![*self.g_1, self.ek[i]], vec![z_ij, r_j]))
                    .collect()
            })
            .collect();

        let second_elgamal_comp = input
            .1
            .iter()
            .map(|&r_j| (vec![*self.h_1], vec![r_j]))
            .collect();

        (first_elgamal_comp, second_elgamal_comp)
    }

    fn msm_eval(
        bases: &[<Self as FixedBaseMSM>::Base],
        scalars: &[<Self as FixedBaseMSM>::Scalar],
    ) -> E::G1 {
        E::G1::msm(bases, scalars).expect("MSM failed in ChunkedElGamal")
    }
}

impl<'a, E: Pairing> homomorphism::Map for Map<'a, E> {
    type Codomain = MatrixVectorPair<E::G1>;
    type Domain = MatrixVectorPair<E::ScalarField>;

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let (first_elgamal_comp, second_elgamal_comp) = self.prep_msms(input);

        (
            first_elgamal_comp
                .into_iter()
                .map(|row| {
                    row.into_iter()
                        .map(|(b, s)| Self::msm_eval(&b, &s))
                        .collect()
                })
                .collect(),
            second_elgamal_comp
                .into_iter()
                .map(|(b, s)| Self::msm_eval(&b, &s))
                .collect(),
        )
    }
}

fn flatten<T>(input: MatrixVectorPair<T>) -> Vec<T> {
    let (mat, vec) = input;
    mat.into_iter().flatten().chain(vec).collect()
}

impl<'a, E: Pairing> FixedBaseMSM for Map<'a, E> {
    type Base = E::G1Affine;
    type Scalar = E::ScalarField;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        let res = Map::<'a, E>::prep_msms(self, input);
        flatten(res)
    }

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        E::G1::normalize_batch(&flatten(output.clone()))
    }
}
