// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::needless_range_loop)]

use velor_dkg::{
    algebra::{
        evaluation_domain::{BatchEvaluationDomain, EvaluationDomain},
        fft::{fft_assign, ifft_assign},
        polynomials::poly_eval,
    },
    utils::random::random_scalars,
};
use blstrs::Scalar;
use ff::Field;
use rand::thread_rng;

#[test]
#[allow(non_snake_case)]
fn test_fft_batch_evaluation_domain() {
    for N in 1..=16 {
        let batch_dom = BatchEvaluationDomain::new(N);

        for k in 1..=N {
            let dom1 = EvaluationDomain::new(k).unwrap();
            let dom2 = batch_dom.get_subdomain(k);
            assert_eq!(dom1, dom2);
        }
    }
}

#[test]
#[allow(non_snake_case)]
fn test_fft_assign() {
    let mut rng = thread_rng();
    for n in [3, 5, 7, 8, 10, 16] {
        let dom = EvaluationDomain::new(n).unwrap();

        let mut f = random_scalars(n, &mut rng);
        //println!("f(X): {:#?}", f);
        let f_orig = f.clone();

        // Computes $f(\omega^i)$ for all $i \in n$, where $\omega$ is an $n$th root of unity and
        // $N$ is the smallest power of two larger than $n$ (or $n$ itself if $n=2^k$).
        fft_assign(&mut f, &dom);
        //println!("FFT(f(X)): {:#?}", f);

        // Correctness test #1: Test against inverse FFT.
        let mut f_inv = f.clone();
        ifft_assign(&mut f_inv, &dom);
        //println!("FFT^{{-1}}(FFT(f(X))): {:#?}", f_inv);

        if f_inv.len() > f_orig.len() {
            let mut i = f_orig.len();
            while i < f_inv.len() {
                assert_eq!(f_inv[i], Scalar::ZERO);
                i += 1;
            }
            f_inv.truncate(f_orig.len());
        }
        assert_eq!(f_inv, f_orig);

        // Correctness test #2: Test by re-evaluating naively at the roots of unity
        let mut omega = Scalar::ONE;
        for i in 0..n {
            let y_i = poly_eval(&f_orig, &omega);

            //println!("y[{i}]: {}", y_i);
            assert_eq!(y_i, f[i]);

            omega *= dom.get_primitive_root_of_unity();
        }
    }
}
