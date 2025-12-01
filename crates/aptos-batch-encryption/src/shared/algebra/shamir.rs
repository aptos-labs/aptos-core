// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use crate::{
    errors::ReconstructError,
    group::{Fr, G1Affine, G1Projective},
    shared::{
        algebra::{differentiate::DifferentiableFn, interpolate::vanishing_poly},
        ark_serialize::*,
    },
    traits::Player,
};
use anyhow::Result;
use ark_ec::VariableBaseMSM as _;
use ark_poly::{EvaluationDomain, Radix2EvaluationDomain};
use ark_std::{rand::RngCore, UniformRand};
use num_traits::{One, Zero};
use rayon::iter::{IndexedParallelIterator as _, IntoParallelIterator, ParallelIterator as _};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub type ShamirShare = (Player, Fr);
pub type ShamirGroupShare = (Player, G1Affine);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub n: usize,
    pub t: usize,
    #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")]
    pub domain: Radix2EvaluationDomain<Fr>,
}

// TODO does this handle values of `n` that are not powers of 2?
impl ThresholdConfig {
    pub fn new(n: usize, t: usize) -> Self {
        let domain = Radix2EvaluationDomain::new(n).unwrap();
        ThresholdConfig { n, t, domain }
    }

    /// for testing
    #[allow(unused)]
    fn lagrange(&self, x: Fr, xs: &HashSet<Fr>) -> Fr {
        let mut prod = Fr::one();

        for xprime in self.domain.elements() {
            if x == xprime {
                continue;
            } else if xs.contains(&xprime) {
                prod *= -xprime;
                prod /= x - xprime;
            }
        }

        prod
    }

    /// Fast lagrange coefficient computation algorithm, taken from the paper
    /// "Towards Scalable Threshold Cryptosystems" by Alin Tomescu, Robert Chen, Yiming Zheng, Ittai
    /// Abraham, Benny Pinkas, Guy Golan Gueta and Srinivas Devadas
    /// (which I think takes it from Modrn Computer Algebra, by von zur Gathen and Gerhard
    pub fn all_lagrange(&self, xs: &HashSet<Fr>) -> HashMap<Fr, Fr> {
        // step 1: compute poly w/ roots at all x in xs, compute eval at 0
        let vanishing_poly = vanishing_poly(&xs.iter().cloned().collect::<Vec<Fr>>());
        let vanishing_poly_eval = vanishing_poly.coeffs[0]; // vanishing_poly(0) = const term

        // step 2  (numerators): for each x in xs, divide poly eval from step 1 by (-x)
        let numerators = self
            .domain
            .elements()
            .collect::<Vec<Fr>>()
            .into_par_iter()
            .filter_map(|x| xs.contains(&x).then_some(vanishing_poly_eval / -x))
            .collect::<Vec<Fr>>();

        // step 3a (denominators): Compute derivative of poly from step 1
        let derivative = vanishing_poly.differentiate();

        // step 3b (denominators): FFT of poly in 3a, keep evals that correspond to the points in
        // question
        let denominators_indexed_by_x = derivative
            .evaluate_over_domain(self.domain)
            .evals
            .into_par_iter()
            .zip(self.domain.elements().collect::<Vec<Fr>>())
            .filter_map(|(y, x)| xs.contains(&x).then_some((x, y)))
            .collect::<Vec<(Fr, Fr)>>();

        // step 4: combine
        numerators
            .into_par_iter()
            .zip(denominators_indexed_by_x)
            .map(|(numerator, (x, denominator))| (x, numerator / denominator))
            .collect()
    }

    pub fn share(&self, val_to_share: Fr, rng: &mut impl RngCore) -> Vec<ShamirShare> {
        let mut coeffs = vec![val_to_share];
        coeffs.extend((0..(self.t - 1)).map(|_| Fr::rand(rng)));
        let y_pts = self.domain.fft(&coeffs);

        y_pts
            .into_iter()
            .enumerate()
            .map(|(i, y)| (Player::new(i), y))
            .take(self.n)
            .collect()
    }

    pub fn reconstruct(&self, shares: &[ShamirShare]) -> Result<Fr> {
        if shares.len() != self.t {
            Err(ReconstructError::ReconstructImproperNumShares)?
        } else {
            let mut sum = Fr::zero();

            let xs: Vec<Fr> = shares
                .iter()
                .map(|(player, _)| self.domain.element(player.id()))
                .collect();
            let lagrange_coeffs = self.all_lagrange(&HashSet::from_iter(xs.iter().cloned()));

            for (x, (_, y)) in xs.iter().zip(shares) {
                sum += lagrange_coeffs[x] * y;
            }

            Ok(sum)
        }
    }

    pub fn reconstruct_in_exponent(&self, shares: &[ShamirGroupShare]) -> Result<G1Affine> {
        if shares.len() != self.t {
            Err(ReconstructError::ReconstructImproperNumShares)?
        } else {
            let xs: Vec<Fr> = shares
                .iter()
                .map(|(player, _)| self.domain.element(player.id()))
                .collect();
            let lagrange_coeffs = self.all_lagrange(&HashSet::from_iter(xs.iter().cloned()));

            let (bases, coeffs): (Vec<G1Affine>, Vec<Fr>) = xs
                .iter()
                .zip(shares)
                .map(|(x, (_, g_y))| (g_y, lagrange_coeffs[x]))
                .collect();

            Ok(G1Projective::msm(&bases, &coeffs)
                .unwrap() // TODO this shouldn't ever panic. Is it ok to leave here?
                .into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_ec::AffineRepr as _;
    use ark_std::rand::thread_rng;
    use itertools::Itertools;

    #[test]
    fn test_all_lagrange() {
        for n in 2..8 {
            for t in 1..=n {
                let params = ThresholdConfig::new(n, t);

                let elements: Vec<Fr> = params.domain.elements().collect();
                for all_elements_vec in elements.into_iter().combinations(t) {
                    let all_elements_iter = all_elements_vec.into_iter();

                    let all_elements = HashSet::from_iter(all_elements_iter.clone());
                    let all_lagrange = params.all_lagrange(&all_elements);

                    for (x, lagrange) in all_lagrange.into_iter() {
                        assert_eq!(lagrange, params.lagrange(x, &all_elements));
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

                let shares = params.share(Fr::from(1u64), &mut rng);

                for reconstruct_shares in shares.into_iter().combinations(t) {
                    assert_eq!(
                        params.reconstruct(&reconstruct_shares).unwrap(),
                        Fr::from(1u64)
                    );
                }
            }
        }
    }

    #[test]
    fn test_reconstruct_in_exponent() {
        let mut rng = thread_rng();
        for n in 2..8 {
            for t in 1..=n {
                let params = ThresholdConfig::new(n, t);

                let shares = params.share(Fr::from(1u64), &mut rng);
                let shares_g1: Vec<ShamirGroupShare> = shares
                    .iter()
                    .map(|(player, y)| (*player, (G1Affine::generator() * y).into()))
                    .collect();

                for reconstruct_shares_g1 in shares_g1.into_iter().combinations(t) {
                    assert_eq!(
                        params
                            .reconstruct_in_exponent(&reconstruct_shares_g1)
                            .unwrap(),
                        G1Affine::generator()
                    );
                }
            }
        }
    }
}
