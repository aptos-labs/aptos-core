// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Contains a version of shamir secret sharing and `ThresholdConfig` for arkworks

use crate::{
    arkworks::{differentiate::DifferentiableFn, vanishing_poly},
    player::Player,
    traits,
};
use anyhow::{anyhow, Result};
use ark_ec::CurveGroup;
use ark_ff::{batch_inversion, FftField, Field, PrimeField};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::fmt;
use rand::{seq::IteratorRandom, Rng};
use rand_core::{CryptoRng, RngCore};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

/// A pair corresponding to a player (i.e., an index into the roots of unity) and a field element
/// evaluation
#[allow(type_alias_bounds)]
pub type ShamirShare<F: PrimeField> = (Player, F);
/// A pair corresponding to a player (i.e., an index into the roots of unity) and a group element
/// whose exponent is an evaluation
#[allow(type_alias_bounds)]
pub type ShamirGroupShare<G: CurveGroup> = (Player, G::Affine);

/// Configuration for a threshold cryptography scheme. Usually one restricts `F` to `Primefield`
/// but any field is theoretically possible.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ShamirSharingScheme<F: FftField> {
    /// Total number of participants (shares)
    pub n: usize,
    /// Threshold number of shares required to reconstruct the secret. Note that in
    /// MPC literature `t` usually denotes the maximal adversary threshold, so `t + 1`
    /// shares would be required to reconstruct the secret
    pub t: usize,
    /// Used for FFT-based polynomial operations. Recomputed from `n` on deserialize
    #[serde(skip)]
    pub domain: Radix2EvaluationDomain<F>,
}

impl<F: PrimeField> traits::SecretSharingConfig for ShamirSharingScheme<F> {
    /// For testing only.
    fn get_random_player<R>(&self, rng: &mut R) -> Player
    where
        R: RngCore + CryptoRng,
    {
        Player {
            id: rng.gen_range(0, self.n),
        }
    }

    /// For testing only.
    fn get_random_eligible_subset_of_players<R>(&self, mut rng: &mut R) -> Vec<Player>
    where
        R: RngCore,
    {
        (0..self.get_total_num_shares())
            .choose_multiple(&mut rng, self.t)
            .into_iter()
            .map(|i| self.get_player(i))
            .collect::<Vec<Player>>()
    }

    fn get_total_num_players(&self) -> usize {
        self.n
    }

    fn get_total_num_shares(&self) -> usize {
        self.n
    }
}

impl<F: PrimeField> traits::ThresholdConfig for ShamirSharingScheme<F> {
    fn new(t: usize, n: usize) -> Result<Self> {
        let domain = Radix2EvaluationDomain::new(n) // Note that `new(n)` internally does `n.next_power_of_two()`
            .expect("Invalid domain size: {}");
        Ok(Self { n, t, domain })
    }

    fn get_threshold(&self) -> usize {
        self.t
    }
}

impl<F: PrimeField> fmt::Display for ShamirSharingScheme<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThresholdConfig {{ n: {}, t: {} }}", self.n, self.t)
    }
}

impl<'de, F: PrimeField> Deserialize<'de> for ShamirSharingScheme<F> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct BasicFields {
            n: usize,
            t: usize,
        }

        let BasicFields { n, t } = BasicFields::deserialize(deserializer)?;

        let domain = Radix2EvaluationDomain::new(n) // Note that `new(n)` internally does `n.next_power_of_two()`
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid domain size: {}", n)))?;

        Ok(ShamirSharingScheme { n, t, domain })
    }
}

// This one will be used for benchmarks only (TODO)
#[allow(dead_code)]
fn naive_all_lagrange_coefficients<F: Field>(xs: &HashSet<F>) -> Vec<(F, F)> {
    let xs_vec: Vec<F> = xs.iter().cloned().collect();
    let n = xs_vec.len();

    // Step 1: Collect denominators for all i
    let mut denominators = Vec::with_capacity(n);

    for (i, &xi) in xs_vec.iter().enumerate() {
        let mut denom = F::one();
        for (j, &xj) in xs_vec.iter().enumerate() {
            if i == j {
                continue;
            }
            denom *= xi - xj;
        }
        denominators.push(denom);
    }
    // Step 2: Invert all denominators at once
    let mut denom_invs = denominators.clone();
    batch_inversion(&mut denom_invs);

    // Step 3: Compute numerators (product of -x_j for j != i)
    let mut results = Vec::with_capacity(n);
    for i in 0..n {
        let xi = xs_vec[i];
        let mut num = F::one();
        for (j, &xj) in xs_vec.iter().enumerate() {
            if i == j {
                continue;
            }
            num *= -xj;
        }

        let li = num * denom_invs[i];
        results.push((xi, li));
    }

    results
}

/// Computes the Lagrange denominators for a set of evaluation points in a Radix-2 FFT domain.
///
/// Specifically, for a polynomial evaluated at points `\omega^0, \dots, \omega^{n-1}` in `dom`,
/// the Lagrange denominators are given by:
///
/// ```text
/// v_i = 1 / \prod_{j \ne i} (\omega^i - \omega^j)
/// ```
#[allow(non_snake_case)]
pub fn all_lagrange_denominators<F: FftField>(
    dom: &Radix2EvaluationDomain<F>,
    n: usize,
    include_zero: bool,
) -> Vec<F> {
    // A(X) = \prod_{i \in [0, n-1]} (X - \omega^i)
    let omegas: Vec<F> = dom.elements().take(n).collect();
    debug_assert_eq!(F::ONE, omegas[0]);
    for i in 1..n {
        debug_assert_eq!(
            omegas[i - 1] * omegas[1],
            omegas[i],
            "omegas are not in sequence at index {}",
            i
        );
    }

    // This is **not** X^n - 1, because the \omega^i are not n-th roots of unity, they are N-th roots of unity where N is some power of 2
    let mut A = vanishing_poly::from_roots(&omegas);

    // A'(X) = \sum_{i \in [0, n-1]} \prod_{j \ne i, j \in [0, n-1]} (X - \omega^j)
    A.differentiate_in_place();
    let A_prime = A;

    // A'(\omega^i) = \prod_{j\ne i, j \in [n] } (\omega^i - \omega^j)
    let mut denoms = dom.fft(&A_prime);
    denoms.truncate(n);

    // If `include_zero`, need to:
    if include_zero {
        // 1. Augment A'(\omega_i) = A'(\omega_i) * \omega^i, for all i\ in [0, n)
        for (i, denom) in denoms.iter_mut().enumerate().take(n) {
            *denom *= omegas[i];
        }

        // 2. Compute A'(0) = \prod_{j \in [0, n)} (0 - \omega^j)
        denoms.push((0..n).map(|i| -omegas[i]).product());
    }

    batch_inversion(&mut denoms);

    denoms
}

impl<F: PrimeField> ShamirSharingScheme<F> {
    /// This initializes a `(t, n)` threshold scheme configuration.
    /// The `domain` is automatically computed as a radix-2 evaluation domain
    /// of size `n.next_power_of_two()` for use in FFT-based polynomial operations.
    pub fn new(t: usize, n: usize) -> Self {
        debug_assert!(t <= n, "Expected t <= n, but t = {} and n = {}", t, n);
        let domain = Radix2EvaluationDomain::new(n).unwrap();
        ShamirSharingScheme { n, t, domain }
    }

    /// Returns the threshold `t` for this `(t, n)` Shamir secret sharing scheme.
    pub fn get_threshold(&self) -> usize {
        self.t
    }

    /// Returns the total number of players `n` in this `(t, n)` Shamir secret sharing
    pub fn get_total_num_players(&self) -> usize {
        self.n
    }

    /// Fast lagrange coefficient computation algorithm, taken from the paper
    /// "Towards Scalable Threshold Cryptosystems" by Alin Tomescu, Robert Chen, Yiming Zheng, Ittai
    /// Abraham, Benny Pinkas, Guy Golan Gueta and Srinivas Devadas
    /// (which I think takes it from Modern Computer Algebra, by von zur Gathen and Gerhard
    pub fn lagrange_for_subset(&self, xs: &HashSet<F>) -> HashMap<F, F> {
        // Step 0: check that subset is large enough
        assert!(
            xs.len() >= self.t,
            "subset size {} is smaller than threshold t={}",
            xs.len(),
            self.t
        );

        let xs_vec: Vec<F> = xs.iter().cloned().collect();

        // Step 1: compute poly w/ roots at all x in xs, compute eval at 0
        let vanishing_poly = vanishing_poly::from_roots(&xs_vec);
        let vanishing_poly_at_0 = vanishing_poly.coeffs[0]; // vanishing_poly(0) = const term

        // Step 2 (numerators): for each x in xs, divide poly eval from step 1 by (-x) using batch inversion
        let mut neg_xs: Vec<F> = xs_vec.iter().map(|&x| -x).collect();
        batch_inversion(&mut neg_xs);
        let numerators: Vec<F> = neg_xs
            .iter()
            .map(|&inv_neg_x| vanishing_poly_at_0 * inv_neg_x)
            .collect();

        // Step 3a (denominators): Compute derivative of poly from step 1, and its evaluations
        let derivative = vanishing_poly.differentiate();
        let derivative_evals = derivative.evaluate_over_domain(self.domain).evals; // TODO: with a filter perhaps we don't have to store all evals, but then batch inversion becomes a bit more tedious

        // Step 3b: Only keep the relevant evaluations, then perform a batch inversion
        let domain_vec: Vec<F> = self.domain.elements().collect();
        let derivative_map: HashMap<F, F> = domain_vec.into_iter().zip(derivative_evals).collect();
        let mut denominators: Vec<F> = xs_vec.iter().map(|x| derivative_map[x]).collect();
        batch_inversion(&mut denominators);

        // Step 4: compute Lagrange coefficients
        xs_vec
            .into_iter()
            .zip(numerators)
            .zip(denominators)
            .map(|((x, numerator), denom_inv)| (x, numerator * denom_inv))
            .collect()
    }

    /// This method creates `n` shares of a secret using
    /// a `(t, n)` Shamir Secret Sharing scheme:
    /// 1. A random polynomial of degree `t-1` is given as input. We are deliberately generating
    /// it outside of this file so it won't depend on the specific version of the `rand` crate.
    /// 2. The polynomial is evaluated over the `domain` using FFT to produce all evaluations,
    ///    which are subsequently trunked.
    pub fn share(&self, coeffs: &[F]) -> Vec<ShamirShare<F>> {
        debug_assert_eq!(coeffs.len(), self.t);
        let evals = self.domain.fft(coeffs);
        (0..self.n).map(Player::new).zip(evals).collect()
    }

    /// This method uses Lagrange interpolation to recover the original secret
    /// from exactly `t` shares. Each share is an `(x, y)` point on the secret-sharing
    /// polynomial. The interpolation coefficients are computed over the field `F`.
    pub fn reconstruct(&self, shares: &[ShamirShare<F>]) -> Result<F> {
        if shares.len() != self.t {
            Err(anyhow!("Incorrect number of shares provided"))
        } else {
            let mut sum = F::zero();

            let xs: Vec<F> = shares
                .iter()
                .map(|(player, _)| self.domain.element(player.get_id()))
                .collect();
            let lagrange_coeffs = self.lagrange_for_subset(&HashSet::from_iter(xs.iter().cloned()));

            for (x, (_, y)) in xs.iter().zip(shares) {
                sum += lagrange_coeffs[x] * y;
            }

            Ok(sum)
        }
    }

    /// Same as above, but in the exponent.
    pub fn reconstruct_in_exponent<G: CurveGroup<ScalarField = F>>(
        &self,
        shares: &[ShamirGroupShare<G>],
    ) -> Result<G::Affine> {
        if shares.len() != self.t {
            Err(anyhow!("Incorrect number of shares provided"))
        } else {
            let xs: Vec<F> = shares
                .iter()
                .map(|(player, _)| self.domain.element(player.get_id()))
                .collect();
            let lagrange_coeffs = self.lagrange_for_subset(&HashSet::from_iter(xs.iter().cloned()));

            let (bases, coeffs): (Vec<G::Affine>, Vec<F>) = xs
                .iter()
                .zip(shares)
                .map(|(x, (_, g_y))| (g_y, lagrange_coeffs[x]))
                .collect();

            Ok(G::msm(&bases, &coeffs)
                .expect("MSM failed during reconstruct_in_exponent()")
                .into())
        }
    }
}

#[cfg(test)]
mod shamir_tests {
    use super::*;
    use crate::arkworks::random::sample_field_elements;
    use ark_bn254::{Fr, G1Affine, G1Projective};
    use ark_ec::AffineRepr as _;
    use ark_ff::One;
    use itertools::Itertools;

    fn single_lagrange(x: Fr, xs: &HashSet<Fr>, omegas: &[Fr]) -> Fr {
        let mut prod = Fr::one();

        for &xprime in omegas {
            if x == xprime {
                continue;
            } else if xs.contains(&xprime) {
                prod *= -xprime;
                prod /= x - xprime;
            }
        }

        prod
    }

    #[test]
    fn test_all_lagrange() {
        use itertools::Itertools;

        for n in 2..8 {
            for t in 1..=n {
                let config = ShamirSharingScheme::new(t, n);

                let elements: Vec<Fr> = config.domain.elements().collect();

                for subset_vec in elements.iter().cloned().combinations(t) {
                    let subset: HashSet<Fr> = subset_vec.iter().cloned().collect();

                    let lagrange_for_subset = config.lagrange_for_subset(&subset);

                    for (x, lagrange) in lagrange_for_subset {
                        let expected = single_lagrange(x, &subset, &elements);
                        assert_eq!(
                            lagrange, expected,
                            "Mismatch at x={:?}, subset={:?}",
                            x, subset
                        );
                    }
                }
            }
        }
    }

    fn sample_shares(
        rng: &mut impl rand::RngCore,
        t: usize,
        n: usize,
    ) -> (ShamirSharingScheme<Fr>, Fr, Vec<ShamirShare<Fr>>) {
        let sharing_scheme = ShamirSharingScheme::new(t, n);

        let coeffs = sample_field_elements(t, rng);

        let shares = sharing_scheme.share(&coeffs);
        (sharing_scheme, coeffs[0], shares)
    }

    #[test]
    fn test_reconstruct() {
        let mut rng = rand::thread_rng();
        for n in 2..8 {
            for t in 1..=n {
                let (sharing_scheme, secret, shares) = sample_shares(&mut rng, t, n);

                for reconstruct_shares in shares.iter().combinations(t) {
                    let reconstruct_shares_vec: Vec<ShamirShare<Fr>> =
                        reconstruct_shares.into_iter().cloned().collect();

                    assert_eq!(
                        sharing_scheme.reconstruct(&reconstruct_shares_vec).unwrap(),
                        secret
                    );
                }
            }
        }
    }

    #[test]
    fn test_reconstruct_in_exponent() {
        let mut rng = rand::thread_rng();
        for n in 2..8 {
            for t in 1..=n {
                let (sharing_scheme, secret, shares) = sample_shares(&mut rng, t, n);

                let shares_g1: Vec<ShamirGroupShare<G1Projective>> = shares
                    .into_iter()
                    .map(|(player, y)| (player, (G1Affine::generator() * y).into()))
                    .collect();

                for reconstruct_shares_g1 in shares_g1.into_iter().combinations(t) {
                    assert_eq!(
                        sharing_scheme
                            .reconstruct_in_exponent::<G1Projective>(&reconstruct_shares_g1)
                            .unwrap(),
                        G1Affine::generator() * secret
                    );
                }
            }
        }
    }

    #[test]
    fn test_all_lagrange_denominators_no_zero() {
        for n in [4, 8, 16] {
            let dom = Radix2EvaluationDomain::<Fr>::new(n).unwrap();

            // Compute denominators
            let denoms = all_lagrange_denominators(&dom, n, false);
            assert_eq!(denoms.len(), n);

            // Manual check: for small n, v_i = 1 / Π_{j≠i} (ω^i - ω^j)
            let omegas: Vec<Fr> = dom.elements().collect();
            for i in 0..n {
                let mut expected = Fr::one();
                for j in 0..n {
                    if i != j {
                        expected *= omegas[i] - omegas[j];
                    }
                }
                expected = expected.inverse().unwrap();
                assert_eq!(denoms[i], expected, "Mismatch at index {}", i);
            }
        }
    }

    #[test]
    fn test_all_lagrange_denominators_with_zero() {
        for n in [4, 8, 16] {
            let dom = Radix2EvaluationDomain::<Fr>::new(n).unwrap();

            let denoms = all_lagrange_denominators(&dom, n, true);
            assert_eq!(denoms.len(), n + 1); // should include the zero term

            let omegas: Vec<Fr> = dom.elements().collect();

            // Check last element corresponds to point at zero
            let expected_zero_term_inv: Fr = (0..n).map(|i| -omegas[i]).product();
            let expected_zero_term = expected_zero_term_inv.inverse().unwrap();
            assert_eq!(denoms[n], expected_zero_term);
        }
    }
}
