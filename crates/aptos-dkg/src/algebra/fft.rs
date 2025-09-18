// Copyright © Aptos Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::algebra::evaluation_domain::EvaluationDomain;
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::Field;
use std::ops::{AddAssign, MulAssign, SubAssign};

/// Computes a Discrete Fourier Transform (DFT), a.k.a., an FFT, on the polynomial $f(X)$ in `poly`,
/// returning all the $N$ evaluations at the roots of unity: $f(\omega^0), f(\omega^1), \ldots, f(\omega^{N-1})$
/// where `dom.log_N` is $\log_2{N}$ and `dom.omega` is $\omega$.
pub fn fft_assign(poly: &mut Vec<Scalar>, dom: &EvaluationDomain) {
    // Pad with zeros, if necessary
    if poly.len() < dom.N {
        poly.resize(dom.N, Scalar::ZERO);
    }

    serial_fft_assign(poly.as_mut_slice(), &dom.omega, dom.log_N as u32)
}

pub fn fft(poly: &Vec<Scalar>, dom: &EvaluationDomain) -> Vec<Scalar> {
    let mut evals = Vec::with_capacity(dom.N);
    evals.resize(poly.len(), Scalar::ZERO);
    evals.copy_from_slice(&poly);

    fft_assign(&mut evals, dom);

    evals
}

/// Computes the inverse of `fft_assign`.
pub fn ifft_assign(poly: &mut Vec<Scalar>, dom: &EvaluationDomain) {
    serial_fft_assign(poly.as_mut_slice(), &dom.omega_inverse, dom.log_N as u32);

    for coeff in poly {
        coeff.mul_assign(&dom.N_inverse);
    }
}

/// TODO: dedup with macro or something
pub fn ifft_assign_g1(poly: &mut Vec<G1Projective>, dom: &EvaluationDomain) {
    serial_fft_assign_g1(poly.as_mut_slice(), &dom.omega_inverse, dom.log_N as u32);

    for coeff in poly {
        coeff.mul_assign(&dom.N_inverse);
    }
}

/// TODO: dedup with macro or something
pub fn ifft_assign_g2(poly: &mut Vec<G2Projective>, dom: &EvaluationDomain) {
    serial_fft_assign_g2(poly.as_mut_slice(), &dom.omega_inverse, dom.log_N as u32);

    for coeff in poly {
        coeff.mul_assign(&dom.N_inverse);
    }
}

/// `bellman`'s FFT code adapted to `blstrs::Scalar`.
fn serial_fft_assign(a: &mut [Scalar], omega: &Scalar, log_n: u32) {
    fn bitreverse(mut n: u32, l: u32) -> u32 {
        let mut r = 0;
        for _ in 0..l {
            r = (r << 1) | (n & 1);
            n >>= 1;
        }
        r
    }

    let n = a.len() as u32;
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k, log_n);
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }

    let mut m = 1;
    for _ in 0..log_n {
        // TODO(Performance): Could have these precomputed via BatchEvaluationDomain, but need to
        //  update all upstream calls to pass in the `BatchEvaluationDomain`.
        let w_m = omega.pow_vartime([u64::from(n / (2 * m))]);

        let mut k = 0;
        while k < n {
            let mut w = Scalar::ONE;
            for j in 0..m {
                let mut t = a[(k + j + m) as usize];
                t.mul_assign(&w);
                let mut tmp = a[(k + j) as usize];
                tmp.sub_assign(&t);
                a[(k + j + m) as usize] = tmp;
                a[(k + j) as usize].add_assign(&t);
                w.mul_assign(&w_m);
            }

            k += 2 * m;
        }

        m *= 2;
    }
}

/// TODO: dedup with macro or something
pub fn serial_fft_assign_g1(a: &mut [G1Projective], omega: &Scalar, log_n: u32) {
    fn bitreverse(mut n: u32, l: u32) -> u32 {
        let mut r = 0;
        for _ in 0..l {
            r = (r << 1) | (n & 1);
            n >>= 1;
        }
        r
    }

    let n = a.len() as u32;
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k, log_n);
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }

    let mut m = 1;
    for _ in 0..log_n {
        // TODO(Performance): Could have these precomputed via BatchEvaluationDomain, but need to
        //  update all upstream calls to pass in the `BatchEvaluationDomain`.
        let w_m = omega.pow_vartime([u64::from(n / (2 * m))]);

        let mut k = 0;
        while k < n {
            let mut w = Scalar::ONE;
            for j in 0..m {
                let mut t = a[(k + j + m) as usize];
                t.mul_assign(&w);
                let mut tmp = a[(k + j) as usize];
                tmp.sub_assign(&t);
                a[(k + j + m) as usize] = tmp;
                a[(k + j) as usize].add_assign(&t);
                w.mul_assign(&w_m);
            }

            k += 2 * m;
        }

        m *= 2;
    }
}

/// TODO: dedup with macro or something
pub fn serial_fft_assign_g2(a: &mut [G2Projective], omega: &Scalar, log_n: u32) {
    fn bitreverse(mut n: u32, l: u32) -> u32 {
        let mut r = 0;
        for _ in 0..l {
            r = (r << 1) | (n & 1);
            n >>= 1;
        }
        r
    }

    let n = a.len() as u32;
    assert_eq!(n, 1 << log_n);

    for k in 0..n {
        let rk = bitreverse(k, log_n);
        if k < rk {
            a.swap(rk as usize, k as usize);
        }
    }

    let mut m = 1;
    for _ in 0..log_n {
        // TODO(Performance): Could have these precomputed via BatchEvaluationDomain, but need to
        //  update all upstream calls to pass in the `BatchEvaluationDomain`.
        let w_m = omega.pow_vartime([u64::from(n / (2 * m))]);

        let mut k = 0;
        while k < n {
            let mut w = Scalar::ONE;
            for j in 0..m {
                let mut t = a[(k + j + m) as usize];
                t.mul_assign(&w);
                let mut tmp = a[(k + j) as usize];
                tmp.sub_assign(&t);
                a[(k + j + m) as usize] = tmp;
                a[(k + j) as usize].add_assign(&t);
                w.mul_assign(&w_m);
            }

            k += 2 * m;
        }

        m *= 2;
    }
}

#[cfg(test)]
mod test {
    use crate::{
        algebra::{
            evaluation_domain::{smallest_power_of_2_greater_than_or_eq, EvaluationDomain},
            fft::fft_assign,
        },
        utils::random::random_scalars,
    };
    use rand::thread_rng;

    #[test]
    #[allow(non_snake_case)]
    fn fft_assign_full_domain() {
        for n in [1, 2, 3, 5, 7, 8, 10, 16] {
            let dom = EvaluationDomain::new(n).unwrap();
            let (N, _) = smallest_power_of_2_greater_than_or_eq(n);

            let mut rng = thread_rng();
            let mut f = random_scalars(n, &mut rng);

            assert_eq!(f.len(), n);
            fft_assign(&mut f, &dom);
            assert_eq!(f.len(), N);
        }
    }
}
