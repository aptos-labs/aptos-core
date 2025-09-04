// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use velor_dkg::{
    algebra::polynomials::{
        poly_eval, poly_mul_fft, poly_mul_less_slow, poly_mul_slow, poly_xnmul,
    },
    utils::{
        multi_pairing, parallel_multi_pairing,
        random::{random_g1_point, random_g2_point, random_scalar, random_scalars},
    },
    weighted_vuf::pinkas::MIN_MULTIPAIR_NUM_JOBS,
};
use velor_runtimes::spawn_rayon_thread_pool;
use blstrs::{G1Projective, G2Projective, Scalar};
use ff::Field;
use group::Group;
use rand::thread_rng;
use std::ops::Mul;

/// TODO(Security): This shouldn't fail, but it does.
#[test]
#[should_panic]
#[ignore]
fn test_crypto_g1_multiexp_more_points() {
    let bases = vec![G1Projective::identity(), G1Projective::identity()];
    let scalars = vec![Scalar::ONE];

    let result = G1Projective::multi_exp(&bases, &scalars);

    assert_eq!(result, bases[0]);
}

/// TODO(Security): This failed once out of the blue. Can never call G1Projective::multi_exp directly
///  because of this.
///
/// Last reproduced on Dec. 5th, 2023 with blstrs 0.7.1:
///  ```
///  failures:
///
///  ---- test_multiexp_less_points stdout ----
///  thread 'test_multiexp_less_points' panicked at 'assertion failed: `(left == right)`
///  left: `G1Projective { x: Fp(0x015216375988dea7b8f1642e6667482a0fe06709923f24e629468da4cf265ea6f03f593188d3557d5cf20a50ff28f870), y: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000), z: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001) }`,
///  right: `G1Projective { x: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000), y: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000), z: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000) }`', crates/velor-dkg/tests/crypto.rs:32:5
///  ```
#[test]
#[ignore]
fn test_crypto_g1_multiexp_less_points() {
    let bases = vec![G1Projective::identity()];
    let scalars = vec![Scalar::ONE, Scalar::ONE];

    let result = G1Projective::multi_exp(&bases, &scalars);

    assert_eq!(result, bases[0]);
}

/// At some point I suspected that size-1 multiexps where the scalar is set to 1 had a bug in them.
/// But they seem fine.
#[test]
fn test_crypto_size_1_multiexp_random_base() {
    let mut rng = thread_rng();

    let bases = vec![random_g2_point(&mut rng)];
    let scalars = vec![Scalar::ONE];

    let result = G2Projective::multi_exp(&bases, &scalars);

    assert_eq!(result, bases[0]);
}

/// TODO(Security): Size-1 G2 multiexps on the generator where the scalar is set to one WILL
///  sometimes fail. Can never call G2Projective::multi_exp directly because of this.
///
/// Last reproduced on Dec. 5th, 2023 with blstrs 0.7.0:
/// ```
///  ---- test_size_1_g2_multiexp_generator_base stdout ----
///  thread 'test_size_1_g2_multiexp_generator_base' panicked at 'assertion failed: `(left == right)`
///    left: `G2Projective { x: Fp2 { c0: Fp(0x0eebd388297e6ad4aa4abe2dd6d2b65061c8a38ce9ac87718432dbdf9843c3a60bbc9706251cb8fa74bc9f5a8572a531), c1: Fp(0x18e7670f7afe6f13acd673491d6d835719c40e5ee1786865ea411262ccafa75c6aef2b28ff973b4532cc4b80e5be4936) }, y: Fp2 { c0: Fp(0x0a4548b4e05e80f16df8a1209b68de65252a7a6f8d8a133bc673ac1505ea59eb30a537e1c1b4e64394d8b2f3aa1f0f14), c1: Fp(0x00b47b3a434ab44b045f5009bcf93b6c47710ffd17c90f35b6ae39864af8d4994003fb223e29a209d609b092042cebbd) }, z: Fp2 { c0: Fp(0x06df5e339dc55dc159f0a845f3f792ea1dee8a0933dc0ed950ed588b21cb553cd6b616f49b73ea3e44ab7618125c9875), c1: Fp(0x0e9d03aee09a7603dc069da045848488f10a51bc5655baffd31f4a7b0e3746cdf93fb3345950f70617730e440f71a8e2) } }`,
///   right: `G2Projective { x: Fp2 { c0: Fp(0x024aa2b2f08f0a91260805272dc51051c6e47ad4fa403b02b4510b647ae3d1770bac0326a805bbefd48056c8c121bdb8), c1: Fp(0x13e02b6052719f607dacd3a088274f65596bd0d09920b61ab5da61bbdc7f5049334cf11213945d57e5ac7d055d042b7e) }, y: Fp2 { c0: Fp(0x0ce5d527727d6e118cc9cdc6da2e351aadfd9baa8cbdd3a76d429a695160d12c923ac9cc3baca289e193548608b82801), c1: Fp(0x0606c4a02ea734cc32acd2b02bc28b99cb3e287e85a763af267492ab572e99ab3f370d275cec1da1aaa9075ff05f79be) }, z: Fp2 { c0: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000001), c1: Fp(0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000) } }`', crates/velor-dkg/tests/crypto.rs:67:5
/// ```
#[test]
#[ignore]
fn test_crypto_g_2_to_zero_multiexp() {
    let bases = vec![G2Projective::generator()];
    let scalars = vec![Scalar::ONE];

    let result = G2Projective::multi_exp(&bases, &scalars);

    assert_eq!(result, bases[0]);
}

/// Size-1 G1 multiexps on the generator where the scalar is set to one do NOT seem to be buggy.
#[test]
fn test_crypto_g_1_to_zero_multiexp() {
    let generator = G1Projective::generator();
    let result = G1Projective::multi_exp([generator].as_slice(), [Scalar::ONE].as_slice());

    assert_eq!(result, generator);
}

#[test]
fn test_crypto_poly_multiply() {
    let mut rng = thread_rng();
    for num_coeffs_f in [1, 2, 3, 4, 5, 6, 7, 8] {
        for num_coeffs_g in [1, 2, 3, 4, 5, 6, 7, 8] {
            let f = random_scalars(num_coeffs_f, &mut rng);
            let g = random_scalars(num_coeffs_g, &mut rng);

            // FFT-based multiplication
            let fft_fg = poly_mul_fft(&f, &g);

            // Naive multiplication
            let naive_fg = poly_mul_slow(&f, &g);

            // We test correctness of $h(X) = f(X) \cdot g(X)$ by picking a random point $r$ and
            // comparing $h(r)$ with $f(r) \cdot g(r)$.
            let r = random_scalar(&mut rng);

            let fg_rand = poly_eval(&f, &r).mul(poly_eval(&g, &r));
            let fft_fg_rand = poly_eval(&fft_fg, &r);
            assert_eq!(fft_fg_rand, fg_rand);

            // We also test correctness of the naive multiplication algorithm
            let naive_fg_rand = poly_eval(&naive_fg, &r);
            assert_eq!(naive_fg_rand, fg_rand);

            // Lastly, of course the naive result should be the same as the FFT result (since they are both correct)
            assert_eq!(naive_fg, fft_fg);
        }
    }
}

#[test]
fn test_crypto_poly_multiply_divide_and_conquer() {
    let mut rng = thread_rng();
    for log_n in [1, 2, 3, 4, 5, 6, 7, 8] {
        let n = 1 << log_n;
        let f = random_scalars(n, &mut rng);
        let g = random_scalars(n, &mut rng);

        let fg = poly_mul_less_slow(&f, &g);

        // FFT-based multiplication
        let fft_fg = poly_mul_fft(&f, &g);
        assert_eq!(fg, fft_fg);

        // Schwartz-Zippel test
        let r = random_scalar(&mut rng);
        let fg_rand = poly_eval(&f, &r).mul(poly_eval(&g, &r));
        let our_fg_rand = poly_eval(&fg, &r);
        assert_eq!(our_fg_rand, fg_rand);
    }
}

#[test]
#[allow(non_snake_case)]
fn test_crypto_poly_shift() {
    let mut rng = thread_rng();
    for num_coeffs_f in [1, 2, 3, 4, 5, 6, 7, 8] {
        for n in 0..16 {
            // compute the coefficients of X^n
            let mut Xn = Vec::with_capacity(n + 1);
            Xn.resize(n + 1, Scalar::ZERO);
            Xn[n] = Scalar::ONE;

            // pick a random f
            let f = random_scalars(num_coeffs_f, &mut rng);

            // f(X) * X^n via shift
            let shifted1 = poly_xnmul(&f, n);
            // f(X) * X^n via multiplication
            let shifted2 = poly_mul_fft(&f, &Xn);

            assert_eq!(shifted1, shifted2);
        }
    }
}

#[test]
fn test_parallel_multi_pairing() {
    let mut rng = thread_rng();

    let r1 = [random_g1_point(&mut rng), random_g1_point(&mut rng)];
    let r2 = [random_g2_point(&mut rng), random_g2_point(&mut rng)];

    let pool1 = spawn_rayon_thread_pool("testmultpair".to_string(), Some(1));
    let pool32 = spawn_rayon_thread_pool("testmultpair".to_string(), Some(32));

    for (g1, g2) in vec![
        ([G1Projective::identity(), r1[0]], r2),
        (r1, r2),
        (r1, [G2Projective::identity(), r2[0]]),
    ] {
        let res1 = multi_pairing(g1.iter(), g2.iter());
        let res2 = parallel_multi_pairing(g1.iter(), g2.iter(), &pool1, MIN_MULTIPAIR_NUM_JOBS);
        let res3 = parallel_multi_pairing(g1.iter(), g2.iter(), &pool32, MIN_MULTIPAIR_NUM_JOBS);

        assert_eq!(res1, res2);
        assert_eq!(res1, res3);
    }
}
