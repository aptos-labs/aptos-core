// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Contains a version of shamir secret sharing and `ThresholdConfig` for arkworks

use crate::arkworks::{
    differentiate::DifferentiableFn,
    serialization::{ark_de, ark_se},
    vanishing_poly,
};
use anyhow::{anyhow, Result};
use ark_ff::{batch_inversion, Field, PrimeField};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::{
    fmt,
    rand::{Rng, RngCore},
};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{HashMap, HashSet};

/// Represents a single share in Shamir's Secret Sharing scheme. Each
/// `ShamirShare` consists of an `(x, y)` point on the secret sharing polynomial.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct ShamirShare<F: PrimeField> {
    /// The interpolation point of the secret sharing polynomial.
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub x: F,
    /// The evaluation of the polynomial at `x`.
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub y: F,
}

/// Configuration for a threshold cryptography scheme. We're restricting F to `Primefield`
/// because Shamir shares are usually defined over such a field. For reconstructing to a group (TODO)
/// we'll use a generic parameter `G: CurveGroup<ScalarField = F>`
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
pub struct ThresholdConfig<F: PrimeField> {
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

impl<F: PrimeField> fmt::Display for ThresholdConfig<F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ThresholdConfig {{ n: {}, t: {} }}", self.n, self.t)
    }
}

impl<'de, F: PrimeField> Deserialize<'de> for ThresholdConfig<F> {
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

        let domain = Radix2EvaluationDomain::new(n) // Note that `new(n)` does `n.next_power_of_two()`
            .ok_or_else(|| serde::de::Error::custom(format!("Invalid domain size: {}", n)))?;

        Ok(ThresholdConfig { n, t, domain })
    }
}

// This one will be used for benchmarks only (TODO)
#[allow(dead_code)]
fn naive_all_lagrange_coefficients<F: Field>(xs: &HashSet<F>) -> Vec<(F, F)> {
    let xs_vec: Vec<F> = xs.iter().cloned().collect();
    let n = xs_vec.len();

    // Step 1: Collect denominators for all i
    let mut denominators = Vec::with_capacity(n);
    for i in 0..n {
        let xi = xs_vec[i];
        let mut denom = F::one();
        for (j, xj) in xs_vec.iter().enumerate() {
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

impl<F: PrimeField> ThresholdConfig<F> {
    /// This initializes a `(t, n)` threshold scheme configuration.
    /// The `domain` is automatically computed as a radix-2 evaluation domain
    /// of size `n.next_power_of_two()` for use in FFT-based polynomial operations.
    pub fn new(n: usize, t: usize) -> Self {
        let domain = Radix2EvaluationDomain::new(n).unwrap();
        ThresholdConfig { n, t, domain }
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
            .map(|inv_neg_x| vanishing_poly_at_0 * *inv_neg_x)
            .collect();

        // Step 3a (denominators): Compute derivative of poly from step 1, and its evaluations
        let derivative = vanishing_poly.differentiate();
        let derivative_evals = derivative.evaluate_over_domain(self.domain).evals; // TODO: with a filter perhaps we don't have to store all evals, but then batch inversion becomes a bit more tedious

        // Step 3b: Only keep the relevant evaluations, then perform a batch inversion
        let domain_vec: Vec<F> = self.domain.elements().collect();
        let derivative_map: HashMap<F, F> = domain_vec
            .into_iter()
            .zip(derivative_evals.into_iter())
            .collect();
        let mut denominators: Vec<F> = xs_vec.iter().map(|x| derivative_map[x]).collect();
        batch_inversion(&mut denominators);

        // Step 4: Compute Lagrange coefficients
        xs_vec
            .into_iter()
            .zip(numerators.into_iter())
            .zip(denominators.into_iter())
            .map(|((x, numerator), denom_inv)| (x, numerator * denom_inv))
            .collect()
    }

    /// This method creates `n` shares of the secret `val_to_share` using
    /// a `(t, n)` Shamir Secret Sharing scheme:
    /// 1. A random polynomial of degree `t-1` is generated with `val_to_share`
    ///    as the constant term.
    /// 2. The polynomial is evaluated over the `domain` using FFT to produce `y` values.
    /// 3. Each share is represented as a `(x, y)` pair (`ShamirShare<F>`).
    pub fn share<R: Rng + RngCore>(&self, val_to_share: F, rng: &mut R) -> Vec<ShamirShare<F>> {
        let mut coeffs = vec![val_to_share]; // constant term of polynomial
        coeffs.extend((0..(self.t - 1)).map(|_| F::rand(rng)));

        self.share_with_coeffs(&coeffs)
    }

    /// `Lightweight` version that only returns the evaluations
    pub fn share_with_coeffs_only_evals(&self, coeffs: &[F]) -> Vec<F> {
        // Evaluate the polynomial over the domain
        let y_pts = self.domain.fft(coeffs);
        y_pts[..self.n].to_vec()
    }

    /// Generates ShamirShares from given polynomial coefficients.
    pub fn share_with_coeffs(&self, coeffs: &[F]) -> Vec<ShamirShare<F>> {
        // Evaluate the polynomial over the domain
        let y_pts = self.share_with_coeffs_only_evals(coeffs);

        self.domain
            .elements()
            .zip(y_pts.iter())
            .map(|(x, &y)| ShamirShare { x, y })
            .collect()
    }

    /// This method uses Lagrange interpolation to recover the original secret
    /// from exactly `t` shares. Each share is an `(x, y)` point on the secret-sharing
    /// polynomial. The interpolation coefficients are computed over the field `F`.
    pub fn reconstruct(&self, shares: &[ShamirShare<F>]) -> Result<F> {
        if shares.len() != self.t {
            Err(anyhow!("Incorrect number of shares provided"))
        } else {
            let mut sum = F::zero();

            let xs = HashSet::from_iter(shares.iter().map(|s| s.x));
            let lagrange_coeffs = self.lagrange_for_subset(&xs);

            for ShamirShare { x, y } in shares {
                sum += lagrange_coeffs[x] * y;
            }

            Ok(sum)
        }
    }
}

#[cfg(test)]
mod shamir_tests {
    use super::*;
    use ark_bn254::Fr;
    use ark_ff::{One, UniformRand};
    use ark_std::rand::thread_rng;
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
                let config = ThresholdConfig::new(n, t);

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

    #[test]
    fn test_reconstruct() {
        for n in 2..8 {
            for t in 1..=n {
                let mut rng = thread_rng();
                let params = ThresholdConfig::new(n, t);

                let val = Fr::rand(&mut rng);
                let shares: Vec<ShamirShare<Fr>> = params.share(val, &mut rng);

                for reconstruct_shares in shares.iter().combinations(t) {
                    let reconstruct_shares_vec: Vec<ShamirShare<Fr>> =
                        reconstruct_shares.into_iter().cloned().collect();

                    assert_eq!(params.reconstruct(&reconstruct_shares_vec).unwrap(), val);
                }
            }
        }
    }
}
