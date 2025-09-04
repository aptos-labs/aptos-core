// Copyright © Velor Foundation
// SPDX-License-Identifier: Apache-2.0

use crate::algebra::{
    evaluation_domain::BatchEvaluationDomain,
    fft::{fft, fft_assign},
    polynomials::{accumulator_poly, poly_differentiate, poly_eval, poly_mul_slow},
};
use blstrs::Scalar;
use ff::{BatchInvert, Field};
use more_asserts::{assert_gt, debug_assert_le};
use std::ops::{Mul, MulAssign};

const FFT_THRESH: usize = 64;

/// Returns all the $N$ Lagrange coefficients for the interpolating set $T = \{\omega^0, \omega^1, \ldots, \omega^{N-1}\}$,
/// where $\omega$ is an $N$th root of unity and $N$ is the size of `dom`.
///
/// Much faster than calling `lagrange_coefficients` on this set, since each Lagrange coefficient
/// has a nice closed-form formula.
///
/// Specifically, if $f(X) = \sum_{i = 0}^{N-1} \ell_i(X) f(\omega^i)$, then expanding $\ell_i(X)$
/// gives $\ell_i(X) = \frac{(1-X^N) \omega^i}{N (\omega^i - X)}$.
///
/// For $X = \alpha$ we get $\ell_i(\alpha) = \frac{(1-\alpha^N) N^{-1} \omega^i}{(\omega^i - \alpha)}$.
///
/// (See <https://ethresear.ch/t/kate-commitments-from-the-lagrange-basis-without-ffts/6950/2>)
#[allow(non_snake_case)]
pub fn all_n_lagrange_coefficients(dom: &BatchEvaluationDomain, alpha: &Scalar) -> Vec<Scalar> {
    let alpha_to_N = alpha.pow_vartime([dom.N() as u64]); // \alpha^N
    let N_inverse = dom.get_subdomain(dom.N()).N_inverse; // N^{-1}
    let one_minus_alpha_to_N = Scalar::ONE - alpha_to_N; // 1 - \alpha^N

    let lhs_numerator = N_inverse * one_minus_alpha_to_N; // (1 - \alpha^N) / N
    let omegas = dom.get_all_roots_of_unity(); // \omega^i, for all i
    let mut denominators = omegas.clone(); // clone
    for i in 0..dom.N() {
        denominators[i] -= alpha // \omega^i - \alpha
    }

    denominators.batch_invert(); // (\omega^i - \alpha)^{-1}

    debug_assert_eq!(denominators.len(), dom.N());

    let mut coeffs = Vec::with_capacity(dom.N());

    for i in 0..dom.N() {
        // i.e., (1 - \alpha^N * \omega^i) / (N (\omega^i - \alpha))
        coeffs.push(lhs_numerator * omegas[i] * denominators[i])
    }

    coeffs
}

/// Let $b$ a bit indicating the `include_zero` truth value.
///
/// If `include_zero` is false, then:
///
///    $$S = {\omega^0, ..., \omega^{n-1}}$$
///
/// else
///
///    $$S = {\omega^0, ..., \omega^{n-1}, 0}$$
///
/// Returns, for all $e \in S$ the multiplicative inverses of
///
///    $$A'(e) = \prod_{e' \ne e, e' \in S} (e - e')$$,
///
/// e.g., when `include_zero` is false:
///
///    $$A'(\omega^i) = \prod_{j \ne i, j \in [0, n)} (\omega^i - \omega^j)$$
///
/// For the `include_zero` is false case, as per Appendix A in [TAB+20e], there is a closed form
/// formula for computing all evaluations: $A'(\omega^i) = n\omega^{-i}$.
///
/// But we cannot always assume `include_zero` is false, and even if we could, we cannot always assume
/// an $n$th primitive root of unity in this algorithm (i.e., $n$ might not be a power of 2), so we
/// instead do a multipoint evaluation of $A'(X)$ above.
///
/// [TAB+20e] Aggregatable Subvector Commitments for Stateless Cryptocurrencies; by Alin Tomescu and
/// Ittai Abraham and Vitalik Buterin and Justin Drake and Dankrad Feist and Dmitry Khovratovich;
/// 2020; <https://eprint.iacr.org/2020/527>
#[allow(non_snake_case)]
pub fn all_lagrange_denominators(
    batch_dom: &BatchEvaluationDomain,
    n: usize,
    include_zero: bool,
) -> Vec<Scalar> {
    // println!(
    //     "all_lagr_denominators> N: {}, n: {n}, include_zero: {include_zero}",
    //     batch_dom.N()
    // );
    // A(X) = \prod_{i \in [0, n-1]} (X - \omega^i)
    let mut A = accumulator_poly_helper(batch_dom, (0..n).collect::<Vec<usize>>().as_slice());

    // A'(X) = \sum_{i \in [0, n-1]} \prod_{j \ne i, j \in [0, n-1]} (X - \omega^j)
    poly_differentiate(&mut A);
    let A_prime = A;
    // println!("all_lagr_denominators> |A_prime|: {}", A_prime.len());

    // A'(\omega^i) = \prod_{j\ne i, j \in [n] } (\omega^i - \omega^j)
    let mut denoms = fft(&A_prime, &batch_dom.get_subdomain(n));
    denoms.truncate(n);

    // If `include_zero`, need to:
    if include_zero {
        // 1. Augment A'(\omega_i) = A'(\omega_i) * \omega^i, for all i\ in [0, n)
        for i in 0..n {
            denoms[i] *= batch_dom.get_root_of_unity(i);
        }

        // 2. Compute A'(0) = \prod_{j \in [0, n)} (0 - \omega^j)
        denoms.push((0..n).map(|i| -batch_dom.get_root_of_unity(i)).product());
    }

    denoms.batch_invert();

    denoms
}

/// Returns the $|T|$ Lagrange coefficients
/// $\ell_i(\alpha) = \prod_{j \in T, j \ne i} \frac{\alpha - \omega^j}{\omega^i - \omega_j}$
/// using the $O(|T| \log^2{|T|})$ algorithm from [TCZ+20], where $\omega$ is an $N$th primitive
/// root of unity (see below for $N$).
///
/// Assumes that the batch evaluation domain in `dom` has all the $N$th roots of unity where $N = 2^k$.
///
/// $T$ contains player identifiers, which are numbers from 0 to $N - 1$ (inclusive).
/// The player with identifier $i$ is associated with $\omega^i$.
///
/// [TCZ+20]: **Towards Scalable Threshold Cryptosystems**, by Alin Tomescu and Robert Chen and
/// Yiming Zheng and Ittai Abraham and Benny Pinkas and Guy Golan Gueta and Srinivas Devadas,
/// *in IEEE S\&P'20*, 2020
#[allow(non_snake_case)]
pub fn lagrange_coefficients(
    dom: &BatchEvaluationDomain,
    T: &[usize],
    alpha: &Scalar,
) -> Vec<Scalar> {
    let N = dom.N();
    let t = T.len();
    assert_gt!(N, 0);

    // Technically, the accumulator poly has degree t, so we need to evaluate it on t+1 points, which
    // will be a problem when t = N, because the evaluation domain will be of size N, not N+1. However,
    // we handle this in `accumulator_poly_helper`
    debug_assert_le!(t, N);

    // The set of $\omega_i$'s for all $i\in [0, N)$.
    let omegas = dom.get_all_roots_of_unity();
    //println!("N = {N}, |T| = t = {t}, T = {:?}, omegas = {:?}", T, omegas);

    // Let $Z(X) = \prod_{i \in T} (X - \omega^i)$
    let mut Z = accumulator_poly_helper(dom, T);

    //println!("Z(0): {}", &Z[0]);
    // Let $Z_i(X) = Z(X) / (X - \omega^i)$, for all $i \in T$.
    // The variable below stores $Z_i(\alpha) = Z(\alpha) / (\alpha - \omega^i)$ for all $i\in T$.
    let Z_i_at_alpha = if alpha.is_zero_vartime() {
        compute_numerators_at_zero(omegas, T, &Z[0])
    } else {
        compute_numerators(&Z, omegas, T, alpha)
    };

    // Compute Z'(X), in place, overwriting Z(X)
    poly_differentiate(&mut Z);

    // Compute $Z'(\omega^i)$ for all $i\in [0, N)$, in place, overwriting $Z'(X)$.
    // (We only need $t$ of them, but computing all of them via an FFT is faster than computing them
    // via a multipoint evaluation.)
    //
    // NOTE: The FFT implementation could be parallelized, but only 17.7% of the time is spent here.
    fft_assign(&mut Z, &dom.get_subdomain(N));

    // Use batch inversion when computing the denominators 1 / Z'(\omega^i) (saves 3 ms)
    let mut denominators = Vec::with_capacity(T.len());
    for i in 0..T.len() {
        debug_assert_ne!(Z[T[i]], Scalar::ZERO);
        denominators.push(Z[T[i]]);
    }
    denominators.batch_invert();

    for i in 0..T.len() {
        Z[i] = Z_i_at_alpha[i].mul(denominators[i]);
    }

    Z.truncate(t);

    Z
}

/// Computes $Z(X) = \prod_{i \in T} (X - \omega^i)$.
#[allow(non_snake_case)]
fn accumulator_poly_helper(dom: &BatchEvaluationDomain, T: &[usize]) -> Vec<Scalar> {
    let omegas = dom.get_all_roots_of_unity();

    // Build the subset of $\omega_i$'s for all $i\in T$.
    let mut set = Vec::with_capacity(T.len());
    for &s in T {
        set.push(omegas[s]);
    }

    // TODO(Performance): This is the performance bottleneck: 75.58% of the time is spent here.
    //
    // Let $Z(X) = \prod_{i \in T} (X - \omega^i)$
    //
    // We handle a nasty edge case here: when doing N out of N interpolation, with N = 2^k, the batch
    // evaluation domain will have N roots of unity, but the degree of the accumulator poly will be
    // N+1. This will trigger an error inside `accumulator_poly` when doing the last FFT-based
    // multiplication, which would require an FFT evaluation domain of size 2N which is not available.
    //
    // To fix this, we handle this case separately by splitting the accumulator poly into an `lhs`
    // of degree `N` which can be safely interpolated with `accumulator_poly` and an `rhs` of degree
    // 1. We then multiply the two together. We do not care about any performance implications of this
    // since we will never use N-out-of-N interpolation.
    //
    // We do this to avoid complicating our Lagrange coefficients API and our BatchEvaluationDomain
    // API (e.g., forbid N out of N Lagrange reconstruction by returning a `Result::Err`).
    let Z = if set.len() < dom.N() {
        accumulator_poly(&set, dom, FFT_THRESH)
    } else {
        // We handle |set| = 1 manually, since the `else` branch would yield an empty `lhs` vector
        // (i.e., a polynomial with zero coefficients) because `set` is empty after `pop()`'ing from
        // it. This makes `poly_mul_slow` bork, since it does not have clear semantics for this case.
        // TODO: Define polynomial multiplication semantics more carefully to avoid such issues.
        if set.len() == 1 {
            accumulator_poly(&set, dom, FFT_THRESH)
        } else {
            let last = set.pop().unwrap();

            let lhs = accumulator_poly(&set, dom, FFT_THRESH);
            let rhs = accumulator_poly(&[last], dom, FFT_THRESH);

            poly_mul_slow(&lhs, &rhs)
        }
    };

    Z
}

/// Let $Z_i(X) = Z(X) / (X - \omega^i)$. Returns a vector of $Z_i(0)$'s, for all $i\in T$.
/// Here, `Z_0` is $Z(0)$.
#[allow(non_snake_case)]
fn compute_numerators_at_zero(omegas: &Vec<Scalar>, T: &[usize], Z_0: &Scalar) -> Vec<Scalar> {
    let N = omegas.len();

    let mut numerators = Vec::with_capacity(T.len());

    for &i in T {
        /*
         * Recall that:
         *
         * When N is even and N > 1:
         *  a) Inverses can be computed fast as: (\omega^k)^{-1} = \omega^{-k} = \omega^N \omega^{-k} = \omega^{N-k}
         *  b) Negations can be computed fast as: -\omega^k = \omega^{k + N/2}
         *
         * So, (0 - \omega^i)^{-1} = (\omega^{i + N/2})^{-1} = \omega^{N - (i + N/2)} = \omega^{N/2 - i}
         * If N/2 < i, then you wrap around to N + N/2 - i.
         *
         * When N = 1 (and thus T = { 0 }), the formula above does not work: it just sets `idx` to
         * N / 2 - i = 1/2 - 0 = 0, which leads to the function using \omega^0 itself instead of
         * -\omega^0.
         */
        if N > 1 {
            let idx = if N / 2 < i { N + N / 2 - i } else { N / 2 - i };

            //println!("Z_{i}(0) = {}", Z_0 * omegas[idx]);
            numerators.push(Z_0 * omegas[idx]);
        } else {
            debug_assert_eq!(T.len(), 1);
            debug_assert_eq!(i, 0);
            numerators.push(Scalar::ONE);
        }
    }

    debug_assert_eq!(numerators.len(), T.len());

    numerators
}

/// Let $Z_i(X) = Z(X) / (X - \omega^i)$. Returns a vector of $Z_i(\alpha)$'s, for all $i\in T$.
#[allow(non_snake_case)]
fn compute_numerators(
    Z: &Vec<Scalar>,
    omegas: &Vec<Scalar>,
    ids: &[usize],
    alpha: &Scalar,
) -> Vec<Scalar> {
    let mut numerators = Vec::with_capacity(ids.len());

    // Z(\alpha)
    let Z_of_alpha = poly_eval(Z, alpha);

    for &i in ids {
        // \alpha - \omega^i
        numerators.push(alpha - omegas[i]);
    }

    // (\alpha - \omega^i)^{-1}
    numerators.batch_invert();

    for i in 0..numerators.len() {
        // Z(\alpha) / (\alpha - \omega^i)^{-1}
        numerators[i].mul_assign(Z_of_alpha);
    }

    numerators
}

#[cfg(test)]
mod test {
    use crate::{
        algebra::{
            evaluation_domain::BatchEvaluationDomain,
            fft::fft_assign,
            lagrange::{all_n_lagrange_coefficients, lagrange_coefficients, FFT_THRESH},
            polynomials::poly_eval,
        },
        utils::random::{random_scalar, random_scalars},
    };
    use blstrs::Scalar;
    use ff::Field;
    use rand::{seq::IteratorRandom, thread_rng};
    use std::ops::Mul;

    #[test]
    fn test_lagrange() {
        let mut rng = thread_rng();

        for n in 1..=FFT_THRESH * 2 {
            for t in 1..=n {
                // println!("t = {t}, n = {n}");
                let deg = t - 1; // the degree of the polynomial

                // pick a random $f(X)$
                let f = random_scalars(deg + 1, &mut rng);

                // give shares to all the $n$ players: i.e., evals[i] = f(\omega^i)
                let batch_dom = BatchEvaluationDomain::new(n);
                let mut evals = f.clone();
                fft_assign(&mut evals, &batch_dom.get_subdomain(n));

                // try to reconstruct $f(0)$ from a random subset of t shares
                let mut players: Vec<usize> = (0..n)
                    .choose_multiple(&mut rng, t)
                    .into_iter()
                    .collect::<Vec<usize>>();

                players.sort();

                let lagr = lagrange_coefficients(&batch_dom, players.as_slice(), &Scalar::ZERO);
                // println!("lagr: {:?}", lagr);

                let mut s = Scalar::ZERO;
                for i in 0..t {
                    s += lagr[i].mul(evals[players[i]]);
                }

                // println!("s   : {s}");
                // println!("f[0]: {}", f[0]);

                assert_eq!(s, f[0]);
            }
        }
    }

    #[test]
    #[allow(non_snake_case)]
    fn test_all_N_lagrange() {
        let mut rng = thread_rng();

        let mut Ns = vec![2];
        while *Ns.last().unwrap() < FFT_THRESH {
            Ns.push(Ns.last().unwrap() * 2);
        }

        for N in Ns {
            // the degree of the polynomial is N - 1

            // pick a random $f(X)$
            let f = random_scalars(N, &mut rng);

            // give shares to all the $n$ players: i.e., evals[i] = f(\omega^i)
            let batch_dom = BatchEvaluationDomain::new(N);
            let mut evals = f.clone();
            fft_assign(&mut evals, &batch_dom.get_subdomain(N));

            // try to reconstruct $f(\alpha)$ from all $N$ shares
            let alpha = random_scalar(&mut rng);
            let lagr1 = all_n_lagrange_coefficients(&batch_dom, &alpha);

            let all = (0..N).collect::<Vec<usize>>();
            let lagr2 = lagrange_coefficients(&batch_dom, all.as_slice(), &alpha);
            assert_eq!(lagr1, lagr2);

            let mut f_of_alpha = Scalar::ZERO;
            for i in 0..N {
                f_of_alpha += lagr1[i].mul(evals[i]);
            }

            let f_of_alpha_eval = poly_eval(&f, &alpha);
            // println!("f(\\alpha) interpolated: {f_of_alpha}");
            // println!("f(\\alpha) evaluated   : {f_of_alpha_eval}");

            assert_eq!(f_of_alpha, f_of_alpha_eval);
        }
    }
}
