// Copyright (c) Aptos Foundation
// SPDX-License-Identifier: Apache-2.0
use super::mult_tree::compute_mult_tree;
use crate::group::Fr;
use ark_ff::Field;
use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
use ark_std::One;
use rayon::iter::{IndexedParallelIterator as _, IntoParallelIterator, ParallelIterator as _};

pub fn vanishing_poly(xs: &[Fr]) -> DensePolynomial<Fr> {
    compute_mult_tree(xs).last().unwrap()[0].clone()
}

fn lagrange(x: Fr, other_xs: &[Fr], vanishing_poly: &DensePolynomial<Fr>) -> DensePolynomial<Fr> {
    let mut result =
        vanishing_poly / &DenseUVPolynomial::from_coefficients_vec(vec![-x, Fr::one()]);

    let denominator: Fr = other_xs
        .into_par_iter()
        .map(|other_x| (x - other_x).inverse().unwrap())
        .reduce(Fr::one, |a, b| a * b);

    result = result * denominator;

    result
}

// TODO I'm not sure this is used anywhere?
pub fn interpolate(xs: &[Fr], ys: &[Fr]) -> DensePolynomial<Fr> {
    let vanishing_poly = vanishing_poly(xs);

    xs.into_par_iter()
        .zip(ys.into_par_iter())
        .enumerate()
        .map(|(i, (x, y))| {
            let other_xs = [&xs[..i], &xs[i + 1..]].concat();
            lagrange(*x, &other_xs, &vanishing_poly) * *y
        })
        .reduce(
            || DensePolynomial::from_coefficients_vec(vec![]),
            |a, b| a + b,
        )
}

#[cfg(test)]
mod tests {
    use super::vanishing_poly;
    use crate::group::Fr;
    use ark_ff::UniformRand as _;
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial, Polynomial};
    use ark_std::{rand::thread_rng, One, Zero};

    #[test]
    fn test_vanishing_poly() {
        let mut rng = thread_rng();
        let xs = vec![Fr::rand(&mut rng); 8];

        let vanishing = vanishing_poly(&xs);
        println!("{:?}", vanishing);

        for x in xs {
            assert_eq!(vanishing.evaluate(&x), Fr::zero());
        }
        assert!(vanishing.evaluate(&Fr::rand(&mut rng)) != Fr::zero());
    }

    #[test]
    fn test_divide() {
        let mut f = Fr::zero();
        let mut xs = vec![];
        for _i in 0..2 {
            f = f - Fr::one();
            xs.push(f);
        }

        println!("{:?}", xs);
        let vanishing = vanishing_poly(&xs);
        println!("{:?}", vanishing);
        let divisor = DensePolynomial::from_coefficients_vec(vec![-xs[0], Fr::one()]);
        println!("{:?}", divisor);
        let l = vanishing / divisor;
        println!("{:?}", l);

        for x in &xs[1..] {
            assert_eq!(l.evaluate(&x), Fr::zero());
        }
        assert!(l.evaluate(&xs[0]) != Fr::zero());
    }
}
