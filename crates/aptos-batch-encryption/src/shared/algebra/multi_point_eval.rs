// Copyright (c) Aptos Foundation
// Licensed pursuant to the Innovation-Enabling Source Code License, available at https://github.com/aptos-labs/aptos-core/blob/main/LICENSE
use super::mult_tree::compute_mult_tree;
use ark_ec::VariableBaseMSM;
use ark_ff::FftField;
use ark_poly::{
    domain::DomainCoeff, univariate::DensePolynomial, DenseUVPolynomial, EvaluationDomain as _,
    Polynomial as _, Radix2EvaluationDomain,
};
use ark_std::log2;
use rayon::iter::{IndexedParallelIterator as _, IntoParallelIterator as _, ParallelIterator as _};
use std::ops::{Deref, Mul};

fn inv_mod_xpow<F: FftField>(h: &DensePolynomial<F>, x_power: usize) -> DensePolynomial<F> {
    let log_xpower = log2(x_power);
    let mut g: DensePolynomial<F> = DenseUVPolynomial::from_coefficients_vec(vec![F::one()]);
    for i in 1..=log_xpower {
        let g_squared = g.clone() * &g;
        let g_times_2 = g.clone() + &g;
        g = g_times_2 - &g_squared * h;
        g.coeffs.truncate(2usize.pow(i));
    }
    g
}

trait Remainder<F: FftField> {
    fn remainder(&self, divisor: &DensePolynomial<F>) -> impl Deref<Target = Self>;
}

impl<F: FftField, T: DomainCoeff<F> + Mul<F, Output = T>> Remainder<F> for [T] {
    fn remainder(&self, divisor: &DensePolynomial<F>) -> Vec<T> {
        let n = self.len() - 1;
        let m = divisor.degree();

        if n < m {
            Vec::from(self)
        } else {
            let domain = Radix2EvaluationDomain::new(2 * (n - m + 1))
                .expect("Should never panic unless the size is ridiculously large");
            let domain2 = Radix2EvaluationDomain::new(n + 1)
                .expect("Should never panic unless the size is ridiculously large");

            let mut f_rev = Vec::from(self);
            f_rev.reverse();
            f_rev.truncate(n - m + 1);

            let mut divisor_rev = divisor.clone();
            divisor_rev.reverse();
            let divisor_rev_inv = inv_mod_xpow(&divisor_rev, n - m + 1);

            let f_rev_evals = domain.fft(&f_rev);
            let divisor_rev_inv_evals = domain.fft(&divisor_rev_inv);

            let quotient_rev_evals: Vec<T> = f_rev_evals
                .into_par_iter()
                .zip(divisor_rev_inv_evals)
                .map(|(x, y)| x * y)
                .collect();
            let mut quotient_rev = domain.ifft(&quotient_rev_evals);
            quotient_rev.truncate(n - m + 1);
            let mut quotient = quotient_rev;
            quotient.reverse();

            let quotient_evals = domain2.fft(&quotient);
            let divisor_evals = domain2.fft(divisor);

            let product_evals: Vec<T> = quotient_evals
                .into_par_iter()
                .zip(divisor_evals.into_par_iter())
                .map(|(x, y)| x * y)
                .collect();
            let product = domain2.ifft(&product_evals);

            let mut result: Vec<T> = product.into_iter().zip(self).map(|(x, y)| *y - x).collect();

            let mut i = result.len();
            while i > 0 && result[i - 1] == T::zero() {
                i -= 1;
            }
            result.truncate(i);

            result
        }
    }
}

fn recurse<F: FftField, T: DomainCoeff<F> + Mul<F, Output = T>>(
    f: &[T],
    mult_tree: &Vec<Vec<DensePolynomial<F>>>,
    level: usize,
    pos: usize,
) -> Vec<T> {
    if f.is_empty() {
        vec![T::zero()]
    } else if f.len() == 1 {
        vec![f[0]]
    } else {
        debug_assert!(mult_tree[level - 1].len() == mult_tree[level].len() * 2);
        debug_assert!(mult_tree[level - 1].len() > 2 * pos + 1);
        let (left, right) = rayon::join(
            || f.remainder(&mult_tree[level - 1][2 * pos]),
            || f.remainder(&mult_tree[level - 1][2 * pos + 1]),
        );

        let (result_left, result_right) = rayon::join(
            || recurse(&left, mult_tree, level - 1, 2 * pos),
            || recurse(&right, mult_tree, level - 1, 2 * pos + 1),
        );
        result_left.into_iter().chain(result_right).collect()
    }
}

pub fn multi_point_eval<F: FftField, T: DomainCoeff<F> + Mul<F, Output = T>>(
    f: &[T],
    x_coords: &[F],
) -> Vec<T> {
    // The way it is written right now, this only supports
    // evaluating a poly on a number of x coords greater than deg(f) + 1
    assert!(x_coords.len() >= f.len());
    let mult_tree = compute_mult_tree(x_coords);
    recurse(f, &mult_tree, mult_tree.len() - 1, 0)
}

pub fn multi_point_eval_naive<
    F: FftField,
    T: DomainCoeff<F> + Mul<F, Output = T> + VariableBaseMSM<ScalarField = F>,
>(
    f: &[T::MulBase],
    x_coords: &[F],
) -> Vec<T> {
    // Note: unlike the non-naive algorithm, this supports an arbitrary
    // number of x coords
    let powers = x_coords
        .into_par_iter()
        .map(|x| {
            let mut result = Vec::new();
            let mut x_power = F::one();
            for _i in 0..f.len() {
                result.push(x_power);
                x_power *= x;
            }
            result
        })
        .collect::<Vec<Vec<F>>>();

    powers
        .into_par_iter()
        .map(|p| T::msm(f, &p).expect("Sizes should always agree by p's construction"))
        .collect()
}

pub fn multi_point_eval_with_mult_tree<F: FftField, T: DomainCoeff<F> + Mul<F, Output = T>>(
    f: &[T],
    mult_tree: &Vec<Vec<DensePolynomial<F>>>,
) -> Vec<T> {
    recurse(f, mult_tree, mult_tree.len() - 1, 0)
}

#[cfg(test)]
mod tests {
    use super::{
        compute_mult_tree, inv_mod_xpow, multi_point_eval, multi_point_eval_naive, Remainder,
    };
    use crate::group::{Fr, G1Affine, G1Projective};
    use ark_poly::{univariate::DensePolynomial, DenseUVPolynomial};
    use ark_std::{rand::thread_rng, One, UniformRand, Zero};

    #[test]
    fn test_multi_point_eval_naive() {
        let poly_size = 2;
        let mut rng = thread_rng();

        let poly: Vec<G1Affine> = (0..poly_size).map(|_| G1Affine::rand(&mut rng)).collect();
        let poly_proj: Vec<G1Projective> = poly.iter().map(|g| G1Projective::from(*g)).collect();
        let x_coords = vec![Fr::one() + Fr::one(); poly_size];

        let evals1 = multi_point_eval(&poly_proj, &x_coords);
        let evals2: Vec<G1Projective> = multi_point_eval_naive(&poly, &x_coords);

        for i in 0..poly_size {
            println!("{}", i);
            assert_eq!(evals1[i], evals2[i]);
        }
    }

    #[test]
    fn test_inv_mod_xpow() {
        let mut rng = thread_rng();

        for i in 0..5 {
            for j in 1..(1 << i) + 3 {
                let mut f: DensePolynomial<Fr> =
                    DenseUVPolynomial::from_coefficients_vec(vec![Fr::one()]);
                f.coeffs.extend_from_slice(
                    &(0..(j - 1))
                        .map(|_| Fr::rand(&mut rng))
                        .collect::<Vec<Fr>>(),
                );
                let finv = inv_mod_xpow(&f, 1 << i);
                let mut prod = f.clone() * &finv;
                prod.coeffs.truncate(1 << i);
                println!("{:?}", finv);
                println!("{:?}", prod);

                assert_eq!(prod[0], Fr::one());
                for coeff in &prod[1..] {
                    assert_eq!(*coeff, Fr::zero());
                }
            }
        }
    }

    #[test]
    fn test_remainder() {
        println!(" -1: {}", -Fr::one());
        println!(" -2: {}", -(Fr::one() + Fr::one()));

        let cone: DensePolynomial<Fr> = DenseUVPolynomial::from_coefficients_vec(vec![Fr::one()]);
        let one: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![-Fr::one(), Fr::one()]);
        let two: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![-(Fr::one() + Fr::one()), Fr::one()]);
        let quadratic = one.clone() * &two;
        println!("{:?}", quadratic);

        let r1 = quadratic.remainder(&one);

        let r2 = (quadratic + &cone).remainder(&one);

        assert_eq!(r1, &[]);
        assert_eq!(r2, cone.coeffs())
    }

    #[test]
    fn test_remainder_2() {
        println!(" -1: {}", -Fr::one());
        println!(" -2: {}", -(Fr::one() + Fr::one()));

        let cone: DensePolynomial<Fr> = DenseUVPolynomial::from_coefficients_vec(vec![-Fr::one()]);
        let one: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![Fr::one(), Fr::one()]);
        let two: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![(Fr::one() + Fr::one()), Fr::one()]);

        let r1 = one.remainder(&one);

        let r2 = one.remainder(&two);

        assert_eq!(r1, &[]);
        assert_eq!(r2, cone.coeffs())
    }

    #[test]
    fn test_remainder_3() {
        println!(" -1: {}", -Fr::one());
        println!(" -2: {}", -(Fr::one() + Fr::one()));

        let x_squared: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![Fr::zero(), Fr::zero(), Fr::one()]);
        let x_squared_minus_x: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![Fr::zero(), -Fr::one(), Fr::one()]);

        let r = x_squared.remainder(&x_squared_minus_x);

        assert_eq!(r, &[Fr::zero(), Fr::one()]);
    }

    #[test]
    fn test_multi_point_eval() {
        let f = [Fr::one(); 2];
        let x_coords = [-Fr::one(), -(Fr::one() + Fr::one())];

        let evals = multi_point_eval(&f, &x_coords);

        assert_eq!(evals.len(), 2);
        assert_eq!(evals[0], f[0] - f[1]);
        assert_eq!(evals[1], f[0] - f[1] - f[1]);
    }

    #[test]
    fn test_multi_point_eval_2() {
        let mut rng = thread_rng();

        let f = [Fr::rand(&mut rng); 2];
        let x_coords = [-Fr::one(), -(Fr::one() + Fr::one())];

        let evals = multi_point_eval(&f, &x_coords);

        assert_eq!(evals.len(), 2);
        assert_eq!(evals[0], f[0] - f[1]);
        assert_eq!(evals[1], f[0] - f[1] - f[1]);
    }

    #[test]
    fn test_multi_point_eval_3() {
        let mut rng = thread_rng();
        let one = Fr::one();
        let two = one + one;
        let three = two + one;
        let four = two + two;

        let f = [Fr::rand(&mut rng); 4];
        let x_coords = [one, two, three, four];

        let evals = multi_point_eval(&f, &x_coords);

        assert_eq!(evals.len(), 4);
        assert_eq!(evals[0], f[0] + f[1] + f[2] + f[3]);
        assert_eq!(
            evals[1],
            f[0] + two * f[1] + two * two * f[2] + two * two * two * f[3]
        );
        assert_eq!(
            evals[2],
            f[0] + three * f[1] + three * three * f[2] + three * three * three * f[3]
        );
        assert_eq!(
            evals[3],
            f[0] + four * f[1] + four * four * f[2] + four * four * four * f[3]
        );
    }

    #[test]
    fn test_multi_point_eval_4() {
        let mut rng = thread_rng();
        let one = Fr::one();
        let two = one + one;
        let three = two + one;
        let four = two + two;

        let f = [G1Projective::rand(&mut rng); 4];
        let x_coords = [one, two, three, four];

        let evals = multi_point_eval(&f, &x_coords);

        assert_eq!(evals.len(), 4);
        assert_eq!(evals[0], f[0] + f[1] + f[2] + f[3]);
        assert_eq!(
            evals[1],
            f[0] + f[1] * two + f[2] * two * two + f[3] * two * two * two
        );
        assert_eq!(
            evals[2],
            f[0] + f[1] * three + f[2] * three * three + f[3] * three * three * three
        );
        assert_eq!(
            evals[3],
            f[0] + f[1] * four + f[2] * four * four + f[3] * four * four * four
        );
    }

    #[test]
    fn test_compute_mult_tree() {
        let roots = [Fr::one(); 4];
        let mult_tree = compute_mult_tree(&roots);
        let p: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![-Fr::one(), Fr::one()]);
        let depth_1_p = p.clone() * &p;
        let depth_2_p = depth_1_p.clone() * &depth_1_p;

        assert_eq!(mult_tree[0], vec![p; 4]);
        assert_eq!(mult_tree[1], vec![depth_1_p; 2]);
        assert_eq!(mult_tree[2], vec![depth_2_p; 1]);
    }

    #[test]
    fn test_compute_mult_tree_2() {
        let one = Fr::one();
        let two = Fr::one() + Fr::one();
        let three = two + Fr::one();
        let four = two + two;

        let pone: DensePolynomial<Fr> = DenseUVPolynomial::from_coefficients_vec(vec![-one, one]);
        let ptwo: DensePolynomial<Fr> = DenseUVPolynomial::from_coefficients_vec(vec![-two, one]);
        let pthree: DensePolynomial<Fr> =
            DenseUVPolynomial::from_coefficients_vec(vec![-three, one]);
        let pfour: DensePolynomial<Fr> = DenseUVPolynomial::from_coefficients_vec(vec![-four, one]);

        let tree = compute_mult_tree(&[one, two, three, four]);
        assert_eq!(tree[0], &[
            pone.clone(),
            ptwo.clone(),
            pthree.clone(),
            pfour.clone()
        ]);
        assert_eq!(tree[1], &[pone.clone() * &ptwo, pthree.clone() * &pfour]);
        assert_eq!(tree[2], &[pone.clone() * ptwo * pthree * pfour]);
    }
}
