// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

//! Contains a version of shamir secret sharing and `ThresholdConfig` for arkworks

use crate::arkworks::{
    differentiate::DifferentiableFn,
    mult_tree::vanishing_poly,
    serialization::{ark_de, ark_se},
};
use anyhow::{anyhow, Result};
use ark_ff::{batch_inversion, Field, PrimeField};
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::rand::{Rng, RngCore};
use rayon::iter::{IndexedParallelIterator, IntoParallelIterator, ParallelIterator};
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
/// we'll use a parameter G: CurveGroup<ScalarField = F>
#[derive(Debug, Clone, Copy, Serialize)]
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
        for j in 0..n {
            if i == j {
                continue;
            }
            let xj = xs_vec[j];
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
        for j in 0..n {
            if i == j {
                continue;
            }
            num *= -xs_vec[j];
        }

        let li = num * denom_invs[i];
        results.push((xi, li));
    }

    results
}

impl<'de, F: PrimeField> ThresholdConfig<F> {
    /// This initializes a `(t, n)` threshold scheme configuration.
    /// The `domain` is automatically computed as a radix-2 evaluation domain
    /// of size `n.next_power_of_two()` for use in FFT-based polynomial operations.
    pub fn new(n: usize, t: usize) -> Self {
        let domain = Radix2EvaluationDomain::new(n).unwrap();
        ThresholdConfig { n, t, domain }
    }

    /// Fast lagrange coefficient computation algorithm, taken from the paper
    /// "Towards Scalable Threshold Cryptosystems" by Alin Tomescu, Robert Chen, Yiming Zheng, Ittai
    /// Abraham, Benny Pinkas, Guy Golan Gueta and Srinivas Devadas
    /// (which I think takes it from Modern Computer Algebra, by von zur Gathen and Gerhard
    pub fn all_lagrange(&self, xs: &HashSet<F>) -> HashMap<F, F> {
        // let coeffs_vec = naive_all_lagrange_coefficients(xs);
        // coeffs_vec.into_iter().collect()

        // Step 1: compute poly w/ roots at all x in xs, compute eval at 0
        let vanishing_poly = vanishing_poly(&xs.into_iter().cloned().collect::<Vec<F>>());
        let vanishing_poly_eval = vanishing_poly.coeffs[0]; // vanishing_poly(0) = const term

        // Step 2 (numerators): for each x in xs, divide poly eval from step 1 by (-x)
        let numerators = self
            .domain
            .elements()
            .collect::<Vec<F>>()
            .into_par_iter()
            .filter_map(|x| xs.contains(&x).then_some(vanishing_poly_eval / -x))
            .collect::<Vec<F>>();

        // Step 3a (denominators): Compute derivative of poly from step 1
        let derivative = vanishing_poly.differentiate();

        // Step 3b (denominators): FFT of poly in 3a, keep evals that correspond to the points in
        // question
        let denominators_indexed_by_x = derivative
            .evaluate_over_domain(self.domain)
            .evals
            .into_par_iter()
            .zip(self.domain.elements().collect::<Vec<F>>())
            .filter_map(|(y, x)| xs.contains(&x).then_some((x, y)))
            .collect::<Vec<(F, F)>>();

        // step 4: combine
        numerators
            .into_par_iter()
            .zip(denominators_indexed_by_x)
            .map(|(numerator, (x, denominator))| (x, numerator / denominator))
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
        let y_pts = self.domain.fft(&coeffs);

        self.domain
            .elements()
            .zip(y_pts.iter())
            .map(|(x, &y)| ShamirShare { x, y })
            .take(self.n)
            .collect()
    }

    /// This method uses Lagrange interpolation to recover the original secret
    /// from exactly `t` shares. Each share is an `(x, y)` point on the secret-sharing
    /// polynomial. The interpolation coefficients are computed over the field `F`.
    pub fn reconstruct(&self, shares: &[ShamirShare<F>]) -> Result<F> {
        if shares.len() != self.t {
            return Err(anyhow!("Incorrect number of shares provided"));
        } else {
            let mut sum = F::zero();

            let xs = HashSet::from_iter(shares.iter().map(|s| s.x));
            let lagrange_coeffs = self.all_lagrange(&xs);

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

    fn single_lagrange(x: Fr, xs: &HashSet<Fr>, omegas: Vec<Fr>) -> Fr {
        let mut prod = Fr::one();

        for xprime in omegas {
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
        for n in 2..8 {
            for t in 1..=n {
                let params = ThresholdConfig::new(n, t);

                let elements: Vec<Fr> = params.domain.elements().collect();
                let omegas = elements.clone();
                for all_elements_vec in elements.into_iter().combinations(t) {
                    let all_elements_iter = all_elements_vec.into_iter();

                    let all_elements = HashSet::from_iter(all_elements_iter.clone());
                    let all_lagrange = params.all_lagrange(&all_elements);

                    for (x, lagrange) in all_lagrange.into_iter() {
                        assert_eq!(lagrange, single_lagrange(x, &all_elements, omegas.clone()));
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
