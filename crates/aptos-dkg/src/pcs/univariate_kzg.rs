// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::algebra::msm;
use ark_ec::{pairing::Pairing, CurveGroup, VariableBaseMSM};

pub struct Map<'a, E: Pairing> {
    pub lagr_g1: &'a [E::G1Affine],
}

impl<'a, E: Pairing> msm::Map for Map<'a, E> {
    type Codomain = E::G1;
    type Domain = (E::ScalarField, Vec<E::ScalarField>);

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        let (bases, scalars) = &msm::FixedBaseMSM::msm_rows(self, input)[0];
        E::G1::msm(bases, scalars).expect("Could not compute MSM for univariate KZG")
    }
}

impl<'a, E: Pairing> msm::FixedBaseMSM for Map<'a, E> {
    type Base = E::G1Affine;
    type Scalar = E::ScalarField;

    fn msm_rows(&self, input: &Self::Domain) -> Vec<(Vec<Self::Base>, Vec<Self::Scalar>)> {
        debug_assert!(
            self.lagr_g1.len() > input.1.len(),
            "Not enough lagrange basis elements for univariate KZG"
        );

        let mut scalars = Vec::with_capacity(input.1.len() + 1);
        scalars.push(input.0);
        scalars.extend_from_slice(&input.1);

        vec![(self.lagr_g1[..1 + input.1.len()].to_vec(), scalars)]
    }

    fn flatten_codomain(&self, output: &Self::Codomain) -> Vec<Self::Base> {
        vec![output.into_affine()]
    }
}
