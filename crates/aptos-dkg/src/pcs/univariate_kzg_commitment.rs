// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::sigma_protocol::{
    homomorphism,
    homomorphism::{fixedbasemsms, fixedbasemsms::Trait, TrivialShape as CodomainShape},
};
use ark_ec::{pairing::Pairing, VariableBaseMSM};
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};

/// Homomorphism for univariate KZG commitments using a Lagrange basis.
///
/// # Description
/// - Maps values (domain), combined with randomness, to a G1 group element (codomain).
/// - Input domain:
///   - `E::ScalarField`: blinding factor, thought of as f(omega^0)
///   - `Vec<E::ScalarField>`: the remaining values, which will correspond to f(omega^i) for i > 0
/// - Uses `lagr_g1` because the input represents **evaluations**, not coefficients.
///
/// For the sake of modularity, we might refactor this in the future to have this homomorphism feed into a homomorphism whose input are only values (by concatenating the inputs into one Vec).
pub struct Homomorphism<'a, E: Pairing> {
    pub lagr_g1: &'a [E::G1Affine],
}

impl<'a, E: Pairing> homomorphism::Trait for Homomorphism<'a, E> {
    type Codomain = CodomainShape<E::G1>;
    /// Input domain: (blinding factor, remaining values)
    type Domain = (E::ScalarField, Vec<E::ScalarField>);

    fn apply(&self, input: &Self::Domain) -> Self::Codomain {
        self.apply_msm(self.msm_terms(input))
    }
}

impl<'a, E: Pairing> fixedbasemsms::Trait for Homomorphism<'a, E> {
    type Base = E::G1Affine;
    type CodomainShape<T>
        = CodomainShape<T>
    where
        T: CanonicalSerialize + CanonicalDeserialize + Clone;
    type MsmInput = fixedbasemsms::MsmInput<Self::Base, Self::Scalar>;
    type MsmOutput = E::G1;
    type Scalar = E::ScalarField;

    fn msm_terms(&self, input: &Self::Domain) -> Self::CodomainShape<Self::MsmInput> {
        debug_assert!(
            self.lagr_g1.len() > input.1.len(),
            "Not enough Lagrange basis elements for univariate KZG: required {}, got {}",
            input.1.len() + 1,
            self.lagr_g1.len()
        );

        let mut scalars = Vec::with_capacity(input.1.len() + 1);
        scalars.push(input.0);
        scalars.extend_from_slice(&input.1);

        CodomainShape(fixedbasemsms::MsmInput {
            bases: self.lagr_g1[..1 + input.1.len()].to_vec(),
            scalars,
        })
    }

    fn msm_eval(bases: &[Self::Base], scalars: &[Self::Scalar]) -> Self::MsmOutput {
        E::G1::msm(bases, &scalars).expect("MSM failed in univariate KZG")
    }
}
