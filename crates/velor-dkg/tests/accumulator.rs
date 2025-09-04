// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_dkg::{
    algebra::{
        evaluation_domain::BatchEvaluationDomain,
        polynomials::{
            accumulator_poly, accumulator_poly_scheduled, accumulator_poly_slow, poly_eval,
        },
    },
    utils::random::{random_scalar, random_scalars},
};
use blstrs::Scalar;
use ff::Field;
use rand::thread_rng;
use std::ops::{MulAssign, Sub};

#[test]
#[allow(non_snake_case)]
fn test_accumulator_poly_scheduled() {
    let mut rng = thread_rng();

    let set_size = 3_300;
    let batch_dom = BatchEvaluationDomain::new(set_size);

    let naive_thresh = 128;
    let fft_thresh = 256;

    let S = random_scalars(set_size, &mut rng);
    let _ = accumulator_poly_scheduled(S.as_slice(), &batch_dom, naive_thresh, fft_thresh);
}

#[test]
#[allow(non_snake_case)]
fn test_accumulator_poly() {
    let mut rng = thread_rng();
    let max_N = 128;
    let batch_dom = BatchEvaluationDomain::new(max_N);
    let naive_thresh = 32;
    let fft_thresh = 16;

    // size 0

    let e = accumulator_poly_slow(&[]);
    assert!(e.is_empty());

    let e = accumulator_poly(&[], &batch_dom, fft_thresh);
    assert!(e.is_empty());

    let e = accumulator_poly_scheduled(&[], &batch_dom, naive_thresh, fft_thresh);
    assert!(e.is_empty());

    // size 1

    let r = random_scalar(&mut rng);
    let Z_slow = accumulator_poly_slow(vec![r].as_slice());
    assert_eq!(Z_slow[1], Scalar::ONE);
    assert_eq!(Z_slow[0], -r);

    let Z = accumulator_poly(vec![r].as_slice(), &batch_dom, fft_thresh);
    assert_eq!(Z, Z_slow);

    let Z_sched =
        accumulator_poly_scheduled(vec![r].as_slice(), &batch_dom, naive_thresh, fft_thresh);
    assert_eq!(Z_sched, Z_slow);

    // arbitrary size
    for set_size in 2..max_N {
        // println!("Testing set size {set_size} (degree {})", set_size + 1);

        let S = random_scalars(set_size, &mut rng);
        let Z1 = accumulator_poly_slow(S.as_slice());
        let Z2 = accumulator_poly(S.as_slice(), &batch_dom, fft_thresh);
        let Z3 = accumulator_poly_scheduled(S.as_slice(), &batch_dom, naive_thresh, fft_thresh);

        assert_eq!(Z1, Z2);
        assert_eq!(Z1, Z3);

        // Test if $Z(X) = \prod_{i \in S} (X - s_i)$ via Schwartz-Zippel by comparing to $Z(r)$
        // for random $r$.
        let r = random_scalar(&mut rng);
        let Z_r = poly_eval(&Z1, &r);
        let mut expected = Scalar::ONE;
        for s in S {
            expected.mul_assign(r.sub(s));
        }

        assert_eq!(Z_r, expected);
    }
}
